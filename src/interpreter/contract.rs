use super::Interpreter;
use crate::consts::*;
use crate::error::{Bug, BugId, BugVariant, RuntimeError};
use crate::storage::InterpreterStorage;

use fuel_asm::{PanicReason, RegisterId, Word};
use fuel_tx::{Contract, Output, Receipt};
use fuel_types::{Address, AssetId, ContractId};

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
        Self::is_register_writable(ra)?;

        let bx = b
            .checked_add(AssetId::LEN as Word)
            .ok_or_else(|| PanicReason::ArithmeticOverflow)?;

        let cx = c
            .checked_add(ContractId::LEN as Word)
            .ok_or_else(|| PanicReason::ArithmeticOverflow)?;

        if bx > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (b, c, bx, cx) = (b as usize, c as usize, bx as usize, cx as usize);

        // Safety: memory bounds checked
        let asset_id = unsafe { AssetId::as_ref_unchecked(&self.memory[b..bx]) };
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };

        if !self.transaction().input_contracts().any(|input| contract == input) {
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let balance = self.balance(contract, asset_id)?;

        self.registers[ra] = balance;

        self.inc_pc()
    }

    pub(crate) fn transfer(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (ax, overflow) = a.overflowing_add(ContractId::LEN as Word);
        let (cx, of) = c.overflowing_add(AssetId::LEN as Word);
        let overflow = overflow || of;

        if overflow || ax > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let amount = b;
        let destination =
            ContractId::try_from(&self.memory[a as usize..ax as usize]).expect("Unreachable! Checked memory range");
        let asset_id =
            AssetId::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        if !self
            .transaction()
            .input_contracts()
            .any(|contract| &destination == contract)
        {
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
        } else {
            // debit external funding source (i.e. free balance)
            self.external_asset_id_balance_sub(&asset_id, amount)?;
        }
        // credit destination contract
        self.balance_increase(&destination, &asset_id, amount)?;

        let receipt = Receipt::transfer(
            internal_context.unwrap_or_default(),
            destination,
            amount,
            asset_id,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.append_receipt(receipt);

        self.inc_pc()
    }

    pub(crate) fn transfer_output(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (ax, overflow) = a.overflowing_add(ContractId::LEN as Word);
        let (dx, of) = d.overflowing_add(AssetId::LEN as Word);
        let overflow = overflow || of;
        let out_idx = b as usize;

        if overflow || ax > VM_MAX_RAM || dx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let to = Address::try_from(&self.memory[a as usize..ax as usize]).expect("Unreachable! Checked memory range");
        let asset_id =
            AssetId::try_from(&self.memory[d as usize..dx as usize]).expect("Unreachable! Checked memory range");
        let amount = c;

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
        } else {
            // debit external funding source (i.e. UTXOs)
            self.external_asset_id_balance_sub(&asset_id, amount)?;
        }

        // credit variable output
        let variable = Output::variable(to, amount, asset_id);

        self.set_variable_output(out_idx, variable)?;

        self.inc_pc()
    }

    pub(crate) fn check_contract_exists(&self, contract: &ContractId) -> Result<bool, RuntimeError> {
        self.storage
            .storage_contract_exists(contract)
            .map_err(RuntimeError::from_io)
    }

    pub(crate) fn balance(&self, contract: &ContractId, asset_id: &AssetId) -> Result<Word, RuntimeError> {
        Ok(self
            .storage
            .merkle_contract_asset_id_balance(contract, asset_id)
            .map_err(RuntimeError::from_io)?
            .unwrap_or_default())
    }

    /// Increase the asset balance for a contract
    pub(crate) fn balance_increase(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        amount: Word,
    ) -> Result<Word, RuntimeError> {
        let balance = self.balance(&contract, &asset_id)?;
        let balance = balance.checked_add(amount).ok_or(PanicReason::ArithmeticOverflow)?;
        self.storage
            .merkle_contract_asset_id_balance_insert(&contract, &asset_id, balance)
            .map_err(RuntimeError::from_io)?;
        Ok(balance)
    }

    /// Decrease the asset balance for a contract
    pub(crate) fn balance_decrease(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        amount: Word,
    ) -> Result<Word, RuntimeError> {
        let balance = self.balance(&contract, &asset_id)?;
        let balance = balance.checked_sub(amount).ok_or(PanicReason::NotEnoughBalance)?;
        self.storage
            .merkle_contract_asset_id_balance_insert(&contract, &asset_id, balance)
            .map_err(RuntimeError::from_io)?;
        Ok(balance)
    }
}
