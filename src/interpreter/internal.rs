use super::{ExecuteError, Interpreter};
use crate::consts::*;

use fuel_asm::{RegisterId, Word};
use fuel_tx::{ContractId, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum Context {
    Predicate,
    Script,
    Call,
    NotInitialized,
}

impl Default for Context {
    fn default() -> Self {
        Self::NotInitialized
    }
}

impl Context {
    pub const fn is_external(&self) -> bool {
        matches!(self, Self::Predicate | Self::Script)
    }
}

impl From<&Transaction> for Context {
    fn from(tx: &Transaction) -> Self {
        if tx.is_script() {
            Self::Script
        } else {
            Self::Predicate
        }
    }
}

impl<S> Interpreter<S> {
    pub(crate) fn push_stack(&mut self, data: &[u8]) -> Result<(), ExecuteError> {
        let (ssp, overflow) = self.registers[REG_SSP].overflowing_add(data.len() as Word);

        if overflow || !self.is_external_context() && ssp > self.registers[REG_FP] {
            Err(ExecuteError::StackOverflow)
        } else {
            self.memory[self.registers[REG_SSP] as usize..ssp as usize].copy_from_slice(data);
            self.registers[REG_SSP] = ssp;

            Ok(())
        }
    }

    pub(crate) const fn block_height(&self) -> u32 {
        self.block_height
    }

    pub(crate) fn set_flag(&mut self, a: Word) {
        self.registers[REG_FLAG] = a;

        self.inc_pc();
    }

    pub(crate) fn clear_err(&mut self) {
        self.registers[REG_ERR] = 0;
    }

    pub(crate) fn set_err(&mut self) {
        self.registers[REG_ERR] = 1;
    }

    pub(crate) fn inc_pc(&mut self) {
        self.registers[REG_PC] += 4;
    }

    pub(crate) const fn context(&self) -> Context {
        if self.registers[REG_FP] == 0 {
            self.context
        } else {
            Context::Call
        }
    }

    pub(crate) const fn is_external_context(&self) -> bool {
        self.context().is_external()
    }

    pub(crate) const fn is_internal_context(&self) -> bool {
        !self.is_external_context()
    }

    pub(crate) const fn is_predicate(&self) -> bool {
        matches!(self.context, Context::Predicate)
    }

    pub(crate) const fn is_register_writable(ra: RegisterId) -> bool {
        ra > REG_FLAG
    }

    pub(crate) const fn transaction(&self) -> &Transaction {
        &self.tx
    }

    pub(crate) fn internal_contract(&self) -> Result<&ContractId, ExecuteError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };

        Ok(contract)
    }

    pub(crate) fn internal_contract_bounds(&self) -> Result<(usize, usize), ExecuteError> {
        self.is_internal_context()
            .then(|| {
                let c = self.registers[REG_FP] as usize;
                let cx = c + ContractId::size_of();

                (c, cx)
            })
            .ok_or(ExecuteError::ExpectedInternalContext)
    }
}
