use super::Interpreter;
use crate::call::{Call, CallFrame};
use crate::consts::*;
use crate::error::InterpreterError;
use crate::state::ProgramState;
use crate::storage::InterpreterStorage;

use fuel_asm::Instruction;
use fuel_tx::crypto::Hasher;
use fuel_tx::{Input, Receipt};
use fuel_types::bytes::SerializableVec;
use fuel_types::{Bytes32, Color, RegisterId, Word};

use std::cmp;
use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    // TODO add CIMV tests
    pub(crate) fn check_input_maturity(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), InterpreterError> {
        Self::is_register_writable(ra)?;

        match self.tx.inputs().get(b as usize) {
            Some(Input::Coin { maturity, .. }) if maturity <= &c => {
                self.registers[ra] = 1;

                self.inc_pc()
            }

            _ => Err(InterpreterError::InputNotFound),
        }
    }

    // TODO add CTMV tests
    pub(crate) fn check_tx_maturity(&mut self, ra: RegisterId, b: Word) -> Result<(), InterpreterError> {
        Self::is_register_writable(ra)?;

        if b <= self.tx.maturity() {
            self.registers[ra] = 1;

            self.inc_pc()
        } else {
            Err(InterpreterError::TxMaturity)
        }
    }

    pub(crate) fn jump(&mut self, j: Word) -> Result<(), InterpreterError> {
        let j = self.registers[REG_IS].saturating_add(j.saturating_mul(Instruction::LEN as Word));

        if j > VM_MAX_RAM - 1 {
            Err(InterpreterError::MemoryOverflow)
        } else {
            self.registers[REG_PC] = j;

            Ok(())
        }
    }

    pub(crate) fn jump_not_equal_imm(&mut self, a: Word, b: Word, imm: Word) -> Result<(), InterpreterError> {
        if a != b {
            self.jump(imm)
        } else {
            self.inc_pc()
        }
    }

    pub(crate) fn call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<ProgramState, InterpreterError> {
        let (ax, overflow) = a.overflowing_add(32);
        let (cx, of) = c.overflowing_add(32);
        let overflow = overflow || of;

        if overflow || ax > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(InterpreterError::MemoryOverflow);
        }

        let call = Call::try_from(&self.memory[a as usize..])?;
        let color = Color::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        if self.is_external_context() {
            self.external_color_balance_sub(&color, b)?;
        }

        if !self.tx.input_contracts().any(|contract| call.to() == contract) {
            return Err(InterpreterError::ContractNotInTxInputs);
        }

        // TODO validate external and internal context
        // TODO update color balance

        let mut frame = self.call_frame(call, color)?;

        let stack = frame.to_bytes();
        let len = stack.len() as Word;

        if len > self.registers[REG_HP] || self.registers[REG_SP] > self.registers[REG_HP] - len {
            return Err(InterpreterError::StackOverflow);
        }

        self.registers[REG_FP] = self.registers[REG_SP];
        self.registers[REG_SP] += len;
        self.registers[REG_SSP] = self.registers[REG_SP];

        self.memory[self.registers[REG_FP] as usize..self.registers[REG_SP] as usize].copy_from_slice(stack.as_slice());

        // TODO set balance for forward coins to $bal
        // TODO set forward gas to $cgas

        self.registers[REG_PC] = self.registers[REG_FP].saturating_add(CallFrame::code_offset() as Word);
        self.registers[REG_IS] = self.registers[REG_PC];
        self.registers[REG_CGAS] = cmp::min(self.registers[REG_GGAS], d);

        let receipt = Receipt::call(
            self.internal_contract_or_default(),
            *frame.to(),
            b,
            *frame.color(),
            d,
            frame.a(),
            frame.b(),
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.receipts.push(receipt);
        self.frames.push(frame);

        self.run_program()
    }

    pub(crate) fn return_from_context(&mut self, receipt: Receipt) -> Result<(), InterpreterError> {
        if let Some(frame) = self.frames.pop() {
            self.registers[REG_CGAS] += frame.context_gas();

            frame
                .registers()
                .iter()
                .enumerate()
                .zip(self.registers.iter_mut())
                .for_each(|((i, frame), current)| {
                    if i != REG_CGAS && i != REG_GGAS && i != REG_RET && i != REG_RETL {
                        *current = *frame;
                    }
                });
        }

        self.receipts.push(receipt);

        self.inc_pc()
    }

    pub(crate) fn ret(&mut self, a: Word) -> Result<(), InterpreterError> {
        let receipt = Receipt::ret(
            self.internal_contract_or_default(),
            a,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.registers[REG_RET] = a;
        self.registers[REG_RETL] = 0;

        // TODO if ret instruction is in memory boundary, inc_pc shouldn't fail
        self.return_from_context(receipt)
    }

    pub(crate) fn ret_data(&mut self, a: Word, b: Word) -> Result<Bytes32, InterpreterError> {
        if b > MEM_MAX_ACCESS_SIZE || a >= VM_MAX_RAM - b {
            return Err(InterpreterError::MemoryOverflow);
        }

        let ab = (a + b) as usize;
        let digest = Hasher::hash(&self.memory[a as usize..ab]);

        let receipt = Receipt::return_data(
            self.internal_contract_or_default(),
            a,
            b,
            digest,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.registers[REG_RET] = a;
        self.registers[REG_RETL] = b;

        self.return_from_context(receipt)?;

        Ok(digest)
    }

    pub(crate) fn revert(&mut self, a: Word) -> Result<(), InterpreterError> {
        let receipt = Receipt::revert(
            self.internal_contract_or_default(),
            a,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.receipts.push(receipt);

        // TODO
        // All OutputContract outputs will have the same amount and stateRoot as on
        // initialization.
        //
        // All OutputVariable outputs will have to and amount of zero.
        //
        // All OutputContractConditional outputs will have contractID, amount, and
        // stateRoot of zero.

        Ok(())
    }
}
