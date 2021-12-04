use super::Interpreter;
use crate::consts::*;
use crate::contract::Contract;
use crate::storage::InterpreterStorage;

use fuel_asm::{PanicReason, RegisterId, Word};
use fuel_types::{Color, ContractId};

use std::borrow::Cow;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn contract(&self, contract: &ContractId) -> Result<Cow<'_, Contract>, PanicReason> {
        self.storage
            .storage_contract(contract)
            .transpose()
            .ok_or(PanicReason::ContractNotFound)?
    }

    pub(crate) fn contract_balance(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), PanicReason> {
        if b > VM_MAX_RAM - Color::LEN as Word || c > VM_MAX_RAM - ContractId::LEN as Word {
            return Err(PanicReason::MemoryOverflow);
        }

        Self::is_register_writable(ra)?;

        let (b, c) = (b as usize, c as usize);

        // Safety: memory bounds checked
        let color = unsafe { Color::as_ref_unchecked(&self.memory[b..b + Color::LEN]) };
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..c + ContractId::LEN]) };

        if !self.tx.input_contracts().any(|input| contract == input) {
            return Err(PanicReason::ContractNotInInputs);
        }

        let balance = self.balance(contract, color)?;

        self.registers[ra] = balance;

        self.inc_pc()
    }

    pub(crate) fn check_contract_exists(&self, contract: &ContractId) -> Result<bool, PanicReason> {
        self.storage.storage_contract_exists(contract)
    }

    pub(crate) fn balance(&self, contract: &ContractId, color: &Color) -> Result<Word, PanicReason> {
        Ok(self
            .storage
            .merkle_contract_color_balance(contract, color)?
            .unwrap_or_default())
    }
}
