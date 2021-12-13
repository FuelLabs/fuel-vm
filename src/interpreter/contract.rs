use super::Interpreter;
use crate::consts::*;
use crate::contract::Contract;
use crate::error::RuntimeError;
use crate::storage::InterpreterStorage;

use fuel_asm::{PanicReason, RegisterId, Word};
use fuel_types::{Color, ContractId};

use std::borrow::Cow;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn contract(&self, contract: &ContractId) -> Result<Cow<'_, Contract>, RuntimeError> {
        self.storage
            .storage_contract(contract)
            .map_err(RuntimeError::from_io)?
            .ok_or(PanicReason::ContractNotFound.into())
    }

    pub(crate) fn contract_balance(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        if b > VM_MAX_RAM - Color::LEN as Word || c > VM_MAX_RAM - ContractId::LEN as Word {
            return Err(PanicReason::MemoryOverflow.into());
        }

        Self::is_register_writable(ra)?;

        let (b, c) = (b as usize, c as usize);

        // Safety: memory bounds checked
        let color = unsafe { Color::as_ref_unchecked(&self.memory[b..b + Color::LEN]) };
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..c + ContractId::LEN]) };

        if !self.tx.input_contracts().any(|input| contract == input) {
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let balance = self.balance(contract, color)?;

        self.registers[ra] = balance;

        self.inc_pc()
    }

    pub(crate) fn check_contract_exists(&self, contract: &ContractId) -> Result<bool, RuntimeError> {
        self.storage
            .storage_contract_exists(contract)
            .map_err(RuntimeError::from_io)
    }

    pub(crate) fn balance(&self, contract: &ContractId, color: &Color) -> Result<Word, RuntimeError> {
        Ok(self
            .storage
            .merkle_contract_color_balance(contract, color)
            .map_err(RuntimeError::from_io)?
            .unwrap_or_default())
    }
}
