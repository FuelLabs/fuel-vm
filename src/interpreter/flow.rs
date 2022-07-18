use super::Interpreter;
use crate::call::{Call, CallFrame};
use crate::consts::*;
use crate::error::RuntimeError;
use crate::state::ProgramState;
use crate::storage::InterpreterStorage;

use fuel_asm::{Instruction, InstructionResult};
use fuel_crypto::Hasher;
use fuel_tx::{PanicReason, Receipt};
use fuel_types::bytes::SerializableVec;
use fuel_types::{AssetId, Bytes32, Word};

use std::cmp;

impl<S> Interpreter<S> {
    pub(crate) fn jump(&mut self, j: Word) -> Result<(), RuntimeError> {
        let j = self.registers[REG_IS].saturating_add(j.saturating_mul(Instruction::LEN as Word));

        if j > VM_MAX_RAM - 1 {
            Err(PanicReason::MemoryOverflow.into())
        } else if self.is_predicate() && j <= self.registers[REG_PC] {
            Err(PanicReason::IllegalJump.into())
        } else {
            self.registers[REG_PC] = j;

            Ok(())
        }
    }

    pub(crate) fn jump_not_equal(&mut self, a: Word, b: Word, to: Word) -> Result<(), RuntimeError> {
        if a != b {
            self.jump(to)
        } else {
            self.inc_pc()
        }
    }

    pub(crate) fn jump_not_zero(&mut self, a: Word, to: Word) -> Result<(), RuntimeError> {
        if a != self.registers[REG_ZERO] {
            self.jump(to)
        } else {
            self.inc_pc()
        }
    }

    pub(crate) fn return_from_context(&mut self, receipt: Receipt) -> Result<(), RuntimeError> {
        if let Some(frame) = self.frames.pop() {
            self.registers[REG_CGAS] = self.registers[REG_CGAS]
                .checked_add(frame.context_gas())
                .ok_or(RuntimeError::halt_on_bug("CGAS would overflow"))?;

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

        self.append_receipt(receipt);

        self.inc_pc()
    }

    pub(crate) fn ret(&mut self, a: Word) -> Result<(), RuntimeError> {
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

    pub(crate) fn ret_data(&mut self, a: Word, b: Word) -> Result<Bytes32, RuntimeError> {
        if b > MEM_MAX_ACCESS_SIZE || a >= VM_MAX_RAM - b {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let ab = (a + b) as usize;
        let digest = Hasher::hash(&self.memory[a as usize..ab]);

        let receipt = Receipt::return_data_with_len(
            self.internal_contract_or_default(),
            a,
            b,
            digest,
            self.memory[a as usize..ab].to_vec(),
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.registers[REG_RET] = a;
        self.registers[REG_RETL] = b;

        self.return_from_context(receipt)?;

        Ok(digest)
    }

    pub(crate) fn revert(&mut self, a: Word) {
        let receipt = Receipt::revert(
            self.internal_contract_or_default(),
            a,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.append_receipt(receipt);
    }

    pub(crate) fn append_panic_receipt(&mut self, result: InstructionResult) {
        let pc = self.registers[REG_PC];
        let is = self.registers[REG_IS];

        let receipt = Receipt::panic(self.internal_contract_or_default(), result, pc, is);

        self.append_receipt(receipt);
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<ProgramState, RuntimeError> {
        let (ax, overflow) = a.overflowing_add(32);
        let (cx, of) = c.overflowing_add(32);
        let overflow = overflow || of;

        if overflow || ax > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let call = Call::try_from(&self.memory[a as usize..])?;
        let asset_id =
            AssetId::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        if self.is_external_context() {
            self.external_asset_id_balance_sub(&asset_id, b)?;
        } else {
            let source_contract = *self.internal_contract()?;
            self.balance_decrease(&source_contract, &asset_id, b)?;
        }

        if !self.tx.input_contracts().any(|contract| call.to() == contract) {
            return Err(PanicReason::ContractNotInInputs.into());
        }

        // credit contract asset_id balance
        self.balance_increase(call.to(), &asset_id, b)?;

        let forward_gas_amount = cmp::min(self.registers[REG_CGAS], d);

        // subtract gas
        self.registers[REG_CGAS] = self.registers[REG_CGAS]
            .checked_sub(forward_gas_amount)
            .ok_or(RuntimeError::halt_on_bug("gas invariant violation"))?;

        let mut frame = self.call_frame(call, asset_id)?;

        let stack = frame.to_bytes();
        let len = stack.len() as Word;

        if len > self.registers[REG_HP] || self.registers[REG_SP] > self.registers[REG_HP] - len {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let id = self.internal_contract_or_default();

        self.registers[REG_FP] = self.registers[REG_SP];
        self.registers[REG_SP] += len;
        self.registers[REG_SSP] = self.registers[REG_SP];

        self.memory[self.registers[REG_FP] as usize..self.registers[REG_SP] as usize].copy_from_slice(stack.as_slice());

        self.registers[REG_BAL] = b;
        self.registers[REG_PC] = self.registers[REG_FP].saturating_add(CallFrame::code_offset() as Word);
        self.registers[REG_IS] = self.registers[REG_PC];
        self.registers[REG_CGAS] = forward_gas_amount;

        let receipt = Receipt::call(
            id,
            *frame.to(),
            b,
            *frame.asset_id(),
            d,
            frame.a(),
            frame.b(),
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.append_receipt(receipt);

        self.frames.push(frame);

        self.run_call()
    }
}
