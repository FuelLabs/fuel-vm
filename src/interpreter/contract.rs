use super::Interpreter;
use crate::consts::*;
use crate::contract::Contract;
use crate::error::RuntimeError;
use crate::storage::InterpreterStorage;

use fuel_asm::{PanicReason, RegisterId, Word};
use fuel_tx::Receipt;
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

    pub(crate) fn transfer(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (ax, overflow) = a.overflowing_add(32);
        let (cx, of) = c.overflowing_add(32);
        let overflow = overflow || of;

        if overflow || ax > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let amount = b;
        let destination =
            ContractId::try_from(&self.memory[a as usize..ax as usize]).expect("Unreachable! Checked memory range");
        let asset_id =
            Color::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        if !self.tx.input_contracts().any(|contract| &destination == contract) {
            return Err(PanicReason::ContractNotInInputs.into());
        }

        if amount == 0 {
            return Err(PanicReason::NotEnoughBalance.into());
        }

        let internal_context = match self.internal_contract() {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(*source_contract),
            // revert to external context if no internal contract is set
            Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e),
        };

        if let Some(source_contract) = internal_context {
            // debit funding source (source contract balance)
            self.balance_decrease(&source_contract, &asset_id, amount)?;

            // credit destination
            self.balance_increase(&destination, &asset_id, amount)?;
        } else {
            // debit external funding source (i.e. UTXOs)
            self.external_color_balance_sub(&asset_id, amount)?;

            // credit destination
            self.balance_increase(&destination, &asset_id, amount)?;
        }

        self.receipts.push(Receipt::transfer(
            internal_context.unwrap_or_default(),
            destination,
            amount,
            asset_id,
            self.registers[REG_PC],
            self.registers[REG_IS],
        ));

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

    /// Increase the asset balance for a contract
    pub(crate) fn balance_increase(
        &mut self,
        contract: &ContractId,
        asset_id: &Color,
        amount: Word,
    ) -> Result<Word, RuntimeError> {
        let balance = self.balance(&contract, &asset_id)?;
        let balance = balance.checked_add(amount).ok_or(PanicReason::ArithmeticOverflow)?;
        self.storage
            .merkle_contract_color_balance_insert(&contract, &asset_id, balance)
            .map_err(RuntimeError::from_io)?;
        Ok(balance)
    }

    /// Decrease the asset balance for a contract
    pub(crate) fn balance_decrease(
        &mut self,
        contract: &ContractId,
        asset_id: &Color,
        amount: Word,
    ) -> Result<Word, RuntimeError> {
        let balance = self.balance(&contract, &asset_id)?;
        let balance = balance.checked_sub(amount).ok_or(PanicReason::NotEnoughBalance)?;
        self.storage
            .merkle_contract_color_balance_insert(&contract, &asset_id, balance)
            .map_err(RuntimeError::from_io)?;
        Ok(balance)
    }
}
