use super::{Call, CallFrame, ExecuteError, Interpreter, ProgramState};
use crate::consts::*;
use crate::data::InterpreterStorage;

use fuel_asm::{RegisterId, Word};
use fuel_tx::bytes::SerializableVec;
use fuel_tx::{Bytes32, Color, Input};

use std::cmp;
use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    // TODO add CIMV tests
    pub(crate) fn check_input_maturity(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), ExecuteError> {
        match self.tx.inputs().get(b as usize) {
            Some(Input::Coin { maturity, .. }) if maturity <= &c => {
                self.registers[ra] = 1;

                self.inc_pc();

                Ok(())
            }

            _ => Err(ExecuteError::InputNotFound),
        }
    }

    // TODO add CTMV tests
    pub(crate) fn check_tx_maturity(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        if b <= self.tx.maturity() {
            self.registers[ra] = 1;

            self.inc_pc();

            Ok(())
        } else {
            Err(ExecuteError::TxMaturityFailed)
        }
    }

    pub(crate) fn jump(&mut self, j: Word) -> Result<(), ExecuteError> {
        if j >= VM_MAX_RAM / 4 + self.registers[REG_IS] / 4 {
            return Err(ExecuteError::MemoryOverflow);
        }

        self.registers[REG_PC] = self.registers[REG_IS] + j * 4;

        Ok(())
    }

    pub(crate) fn jump_not_equal_imm(&mut self, a: Word, b: Word, imm: Word) -> Result<(), ExecuteError> {
        if a != b {
            self.jump(imm)
        } else {
            self.inc_pc();

            Ok(())
        }
    }

    pub(crate) fn call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<ProgramState, ExecuteError> {
        if a > VM_MAX_RAM - Bytes32::size_of() as Word || c > VM_MAX_RAM + Color::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (a, c) = (a as usize, c as usize);

        let cx = c + Color::size_of();

        // Safety: checked memory bounds
        let call = Call::try_from(&self.memory[a..])?;
        let color = unsafe { Color::from_slice_unchecked(&self.memory[c..cx]) };

        if self.is_external_context() {
            self.external_color_balance_sub(&color, b)?;
        }

        if !self.tx.input_contracts().any(|contract| call.to() == contract) {
            return Err(ExecuteError::ContractNotInTxInputs);
        }

        // TODO validate external and internal context
        // TODO update color balance

        let mut frame = self.call_frame(call, color)?;

        let stack = frame.to_bytes();
        let len = stack.len() as Word;

        if len > self.registers[REG_HP] || self.registers[REG_SP] > self.registers[REG_HP] - len {
            return Err(ExecuteError::StackOverflow);
        }

        self.registers[REG_FP] = self.registers[REG_SP];
        self.registers[REG_SP] += len;
        self.registers[REG_SSP] = self.registers[REG_SP];

        self.memory[self.registers[REG_FP] as usize..self.registers[REG_SP] as usize].copy_from_slice(stack.as_slice());

        // TODO set balance for forward coins to $bal
        // TODO set forward gas to $cgas

        self.registers[REG_PC] = self.registers[REG_FP] + CallFrame::code_offset() as Word;
        self.registers[REG_IS] = self.registers[REG_PC];
        self.registers[REG_CGAS] = cmp::min(self.registers[REG_GGAS], d);

        self.frames.push(frame);

        self.run_program()
    }

    pub(crate) fn ret(&mut self, ra: RegisterId) -> Result<(), ExecuteError> {
        // TODO Return the unused forwarded gas to the caller

        self.registers[REG_RET] = self.registers[ra];
        self.registers[REG_RETL] = 0;

        if let Some(frame) = self.frames.pop() {
            self.registers[REG_CGAS] += frame.context_gas();

            let cgas = self.registers[REG_CGAS];
            let ggas = self.registers[REG_GGAS];
            let ret = self.registers[REG_RET];
            let retl = self.registers[REG_RETL];

            self.registers.copy_from_slice(frame.registers());

            self.registers[REG_CGAS] = cgas;
            self.registers[REG_GGAS] = ggas;
            self.registers[REG_RET] = ret;
            self.registers[REG_RETL] = retl;
        }

        self.log_return(ra);
        self.inc_pc();

        Ok(())
    }
}
