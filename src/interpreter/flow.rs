use super::{Call, CallFrame, ExecuteError, Interpreter, ProgramState};
use crate::consts::*;
use crate::data::InterpreterStorage;

use fuel_asm::{RegisterId, Word};
use fuel_tx::bytes::SerializableVec;
use fuel_tx::{Color, Input};

use std::cmp;
use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    // TODO add CIMV tests
    pub fn check_input_maturity(&mut self, ra: RegisterId, b: Word, c: Word) -> bool {
        match self.tx.inputs().get(b as usize) {
            Some(Input::Coin { maturity, .. }) if maturity <= &c => {
                self.registers[ra] = 1;

                true
            }

            _ => false,
        }
    }

    // TODO add CTMV tests
    pub fn check_tx_maturity(&mut self, ra: RegisterId, b: Word) -> bool {
        if b <= self.tx.maturity() {
            self.registers[ra] = 1;

            true
        } else {
            false
        }
    }

    pub fn jump(&mut self, j: Word) -> bool {
        let j = self.registers[REG_IS].saturating_add(j.saturating_mul(4));

        if j > VM_MAX_RAM - 1 {
            false
        } else {
            self.registers[REG_PC] = j;

            true
        }
    }

    pub fn jump_not_equal_imm(&mut self, a: Word, b: Word, imm: Word) -> bool {
        if a != b {
            self.jump(imm)
        } else {
            self.inc_pc()
        }
    }

    pub fn call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<ProgramState, ExecuteError> {
        let (ax, overflow) = a.overflowing_add(32);
        let (cx, of) = c.overflowing_add(32);
        let overflow = overflow || of;

        if overflow || ax > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(ExecuteError::MemoryOverflow);
        }

        let call = Call::try_from(&self.memory[a as usize..])?;
        let color = Color::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        if self.is_external_context() {
            self.external_color_balance_sub(&color, b)?;
        }

        if !self.tx.input_contracts().any(|contract| call.to() == contract) {
            return Err(ExecuteError::ContractNotInTxInputs);
        }

        // TODO validate external and internal context
        // TODO update color balance

        let mut frame = self.call_frame(call, color)?;

        let sp = self.registers[REG_SP];
        self.push_stack(frame.to_bytes().as_slice())?;
        self.registers[REG_FP] = sp;

        // TODO set balance for forward coins to $bal
        // TODO set forward gas to $cgas

        self.registers[REG_PC] = self.registers[REG_FP].saturating_add(CallFrame::code_offset() as Word);
        self.registers[REG_IS] = self.registers[REG_PC];
        self.registers[REG_CGAS] = cmp::min(self.registers[REG_GGAS], d);

        self.frames.push(frame);

        self.run_program()
    }

    pub fn ret(&mut self, ra: RegisterId) -> bool {
        // TODO Return the unused forwarded gas to the caller

        if !self
            .registers
            .get(ra)
            .copied()
            .map(|a| {
                self.registers[REG_RET] = a;
                self.registers[REG_RETL] = 0;
                true
            })
            .unwrap_or(false)
        {
            return false;
        }

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

        self.log_return(ra)
    }
}
