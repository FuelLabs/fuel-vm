use super::internal::{external_asset_id_balance_sub, inc_pc, set_variable_output};
use super::{ExecutableTransaction, Interpreter};
use crate::constraints::reg_key::*;
use crate::error::RuntimeError;
use crate::interpreter::PanicContext;
use crate::storage::ContractsRawCode;
use crate::storage::{ContractsAssets, ContractsAssetsStorage, InterpreterStorage};

use fuel_asm::{PanicReason, RegId, RegisterId, Word};
use fuel_storage::{StorageInspect, StorageSize};
use fuel_tx::{Contract, Output, Receipt};
use fuel_types::{Address, AssetId, ContractId};

use std::borrow::Cow;

#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    pub(crate) fn contract_balance(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        let asset_id = AssetId::from(self.mem_read_bytes(b)?);
        let contract = ContractId::from(self.mem_read_bytes(c)?);

        if !self.tx.input_contracts().any(|input| contract == *input) {
            self.panic_context = PanicContext::ContractId(contract);
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let balance = balance(&self.storage, &contract, &asset_id)?;

        self.registers[ra] = balance;

        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn transfer(
        &mut self,
        destination_ptr: Word,
        amount: Word,
        asset_id_ptr: Word,
    ) -> Result<(), RuntimeError> {
        // let input = TransferCtx {
        //     storage: &mut self.storage,
        //     memory: &mut self.memory,
        //     context: &self.context,
        //     balances: &mut self.balances,
        //     receipts: &mut self.receipts,
        //     tx: &mut self.tx,
        //     tx_offset: self.params.tx_offset(),
        //     fp: fp.as_ref(),
        //     is: is.as_ref(),
        //     pc,
        // };

        let destination = ContractId::from(self.mem_read_bytes(destination_ptr)?);
        let asset_id = AssetId::from(self.mem_read_bytes(asset_id_ptr)?);

        if !self.tx.input_contracts().any(|contract| &destination == contract) {
            self.panic_context = PanicContext::ContractId(destination);
            return Err(PanicReason::ContractNotInInputs.into());
        }

        if amount == 0 {
            return Err(PanicReason::NotEnoughBalance.into());
        }

        let internal_context = match self.internal_contract() {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(source_contract),
            // revert to external context if no internal contract is set
            Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e),
        };

        if let Some(source_contract) = internal_context {
            // debit funding source (source contract balance)
            balance_decrease(&mut self.storage, &source_contract, &asset_id, amount)?;
        } else {
            // debit external funding source (i.e. free balance)
            external_asset_id_balance_sub(&mut self.balances, &mut self.memory, &asset_id, amount)?;
        }
        // credit destination contract
        balance_increase(&mut self.storage, &destination, &asset_id, amount)?;

        let receipt = Receipt::transfer(
            internal_context.unwrap_or_default(),
            destination,
            amount,
            asset_id,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.append_receipt(receipt);

        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn transfer_output(
        &mut self,
        to_ptr: Word,
        out_idx: Word,
        amount: Word,
        asset_id_ptr: Word,
    ) -> Result<(), RuntimeError> {
        let to = Address::from(self.mem_read_bytes(to_ptr)?);
        let asset_id = AssetId::from(self.mem_read_bytes(asset_id_ptr)?);
        let out_idx = out_idx as usize; // TODO: check bounds

        let internal_context = match self.internal_contract() {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(source_contract),
            // revert to external context if no internal contract is set
            Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e),
        };

        if let Some(source_contract) = internal_context {
            // debit funding source (source contract balance)
            balance_decrease(&mut self.storage, &source_contract, &asset_id, amount)?;
        } else {
            // debit external funding source (i.e. UTXOs)
            external_asset_id_balance_sub(&mut self.balances, &mut self.memory, &asset_id, amount)?;
        }

        // credit variable output
        let variable = Output::variable(to, amount, asset_id);

        set_variable_output(
            &mut self.tx,
            &mut self.memory,
            self.params.tx_offset(),
            out_idx,
            variable,
        )?;

        let receipt = Receipt::transfer_out(
            internal_context.unwrap_or_default(),
            to,
            amount,
            asset_id,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.append_receipt(receipt);

        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn check_contract_exists(&self, contract: &ContractId) -> Result<bool, RuntimeError> {
        self.storage
            .storage_contract_exists(contract)
            .map_err(RuntimeError::from_io)
    }
}

pub(crate) fn contract<'s, S>(storage: &'s S, contract: &ContractId) -> Result<Cow<'s, Contract>, RuntimeError>
where
    S: InterpreterStorage,
{
    storage
        .storage_contract(contract)
        .map_err(RuntimeError::from_io)?
        .ok_or_else(|| PanicReason::ContractNotFound.into())
}

pub(crate) fn contract_size<S>(storage: &S, contract: &ContractId) -> Result<Word, RuntimeError>
where
    S: StorageSize<ContractsRawCode> + ?Sized,
    <S as StorageInspect<ContractsRawCode>>::Error: Into<std::io::Error>,
{
    Ok(storage
        .size_of_value(contract)
        .map_err(RuntimeError::from_io)?
        .ok_or(PanicReason::ContractNotFound)? as Word)
}

pub(crate) fn balance<S>(storage: &S, contract: &ContractId, asset_id: &AssetId) -> Result<Word, RuntimeError>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    Ok(storage
        .merkle_contract_asset_id_balance(contract, asset_id)
        .map_err(RuntimeError::from_io)?
        .unwrap_or_default())
}

/// Increase the asset balance for a contract
pub(crate) fn balance_increase<S>(
    storage: &mut S,
    contract: &ContractId,
    asset_id: &AssetId,
    amount: Word,
) -> Result<Word, RuntimeError>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    let balance = balance(storage, contract, asset_id)?;
    let balance = balance.checked_add(amount).ok_or(PanicReason::ArithmeticOverflow)?;
    storage
        .merkle_contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::from_io)?;
    Ok(balance)
}

/// Decrease the asset balance for a contract
pub(crate) fn balance_decrease<S>(
    storage: &mut S,
    contract: &ContractId,
    asset_id: &AssetId,
    amount: Word,
) -> Result<Word, RuntimeError>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    let balance = balance(storage, contract, asset_id)?;
    let balance = balance.checked_sub(amount).ok_or(PanicReason::NotEnoughBalance)?;
    storage
        .merkle_contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::from_io)?;
    Ok(balance)
}
