//! This module contains logic on contract management.

use super::{
    ExecutableTransaction,
    Interpreter,
    Memory,
    internal::{
        external_asset_id_balance_sub,
        inc_pc,
        set_variable_output,
    },
};
use crate::{
    constraints::reg_key::*,
    consts::*,
    convert,
    error::{
        IoResult,
        RuntimeError,
    },
    storage::{
        BlobData,
        ContractsAssetsStorage,
        ContractsRawCode,
        InterpreterStorage,
    },
    verification::Verifier,
};
use fuel_asm::{
    PanicReason,
    RegId,
    Word,
};
use fuel_storage::StorageSize;
use fuel_tx::{
    Output,
    Receipt,
};
use fuel_types::{
    Address,
    AssetId,
    BlobId,
    Bytes32,
    ContractId,
};

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    V: Verifier,
{
    pub(crate) fn contract_balance(
        &mut self,
        ra: RegId,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let asset_id = AssetId::new(self.memory().read_bytes(b)?);
        let contract_id = ContractId::new(self.memory().read_bytes(c)?);
        self.verifier.check_contract_in_inputs(
            &mut self.panic_context,
            &self.input_contracts,
            &contract_id,
        )?;
        let balance = balance(&self.storage, &contract_id, &asset_id)?;
        self.write_user_register_legacy(ra, balance)?;
        Ok(inc_pc(self.registers.pc_mut())?)
    }

    pub(crate) fn transfer(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        {
            let amount = b;
            let destination = ContractId::from(self.memory().read_bytes(a)?);
            let asset_id = AssetId::from(self.memory().read_bytes(c)?);

            self.verifier.check_contract_in_inputs(
                &mut self.panic_context,
                &self.input_contracts,
                &destination,
            )?;

            if amount == 0 {
                return Err(PanicReason::TransferZeroCoins.into())
            }

            let internal_context = match self.internal_contract() {
                // optimistically attempt to load the internal contract id
                Ok(source_contract) => Some(source_contract),
                // revert to external context if no internal contract is set
                Err(PanicReason::ExpectedInternalContext) => None,
                // bubble up any other kind of errors
                Err(e) => return Err(e.into()),
            };

            if let Some(source_contract) = internal_context {
                // debit funding source (source contract balance)
                balance_decrease(&mut self.storage, &source_contract, &asset_id, amount)?;
            } else {
                // debit external funding source (i.e. free balance)
                external_asset_id_balance_sub(
                    &mut self.balances,
                    self.memory.as_mut(),
                    &asset_id,
                    amount,
                )?;
            }
            // credit destination contract
            let created_new_entry =
                balance_increase(&mut self.storage, &destination, &asset_id, amount)?;
            if created_new_entry {
                // If a new entry was created, we must charge gas for it
                self.gas_charge(
                    ((Bytes32::LEN + WORD_SIZE) as u64)
                        .saturating_mul(new_storage_gas_per_byte),
                )?;
            }

            let receipt = Receipt::transfer(
                internal_context.unwrap_or_default(),
                destination,
                amount,
                asset_id,
                self.registers[RegId::PC],
                self.registers[RegId::IS],
            );

            self.receipts.push(receipt)?;

            Ok(inc_pc(self.registers.pc_mut())?)
        }
    }

    pub(crate) fn transfer_output(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        {
            let out_idx = convert::to_usize(b).ok_or(PanicReason::OutputNotFound)?;
            let to = Address::from(self.memory().read_bytes(a)?);
            let asset_id = AssetId::from(self.memory().read_bytes(d)?);
            let amount = c;

            if amount == 0 {
                return Err(PanicReason::TransferZeroCoins.into())
            }

            let internal_context = match self.internal_contract() {
                // optimistically attempt to load the internal contract id
                Ok(source_contract) => Some(source_contract),
                // revert to external context if no internal contract is set
                Err(PanicReason::ExpectedInternalContext) => None,
                // bubble up any other kind of errors
                Err(e) => return Err(e.into()),
            };

            if let Some(source_contract) = internal_context {
                // debit funding source (source contract balance)
                balance_decrease(&mut self.storage, &source_contract, &asset_id, amount)?;
            } else {
                // debit external funding source (i.e. UTXOs)
                external_asset_id_balance_sub(
                    &mut self.balances,
                    self.memory.as_mut(),
                    &asset_id,
                    amount,
                )?;
            }

            // credit variable output
            let variable = Output::variable(to, amount, asset_id);

            let tx_offset = self.tx_offset();
            set_variable_output(
                &mut self.tx,
                self.memory.as_mut(),
                tx_offset,
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

            self.receipts.push(receipt)?;

            Ok(inc_pc(self.registers.pc_mut())?)
        }
    }

    pub(crate) fn check_contract_exists(
        &self,
        contract: &ContractId,
    ) -> IoResult<bool, S::DataError> {
        self.storage
            .storage_contract_exists(contract)
            .map_err(RuntimeError::Storage)
    }
}

pub(crate) fn contract_size<S>(
    storage: &S,
    contract: &ContractId,
) -> IoResult<usize, S::Error>
where
    S: StorageSize<ContractsRawCode> + ?Sized,
{
    let size = storage
        .size_of_value(contract)
        .map_err(RuntimeError::Storage)?
        .ok_or(PanicReason::ContractNotFound)?;
    Ok(size)
}

pub(crate) fn blob_size<S>(storage: &S, blob_id: &BlobId) -> IoResult<usize, S::Error>
where
    S: StorageSize<BlobData> + ?Sized,
{
    let size = storage
        .size_of_value(blob_id)
        .map_err(RuntimeError::Storage)?
        .ok_or(PanicReason::BlobNotFound)?;
    Ok(size)
}

pub(crate) fn balance<S>(
    storage: &S,
    contract: &ContractId,
    asset_id: &AssetId,
) -> IoResult<Word, S::Error>
where
    S: ContractsAssetsStorage + ?Sized,
{
    Ok(storage
        .contract_asset_id_balance(contract, asset_id)
        .map_err(RuntimeError::Storage)?
        .unwrap_or_default())
}

/// Increase the asset balance for a contract, unless the `amount` is zero.
/// A boolean indicating if a new entry was created.
pub fn balance_increase<S>(
    storage: &mut S,
    contract: &ContractId,
    asset_id: &AssetId,
    amount: Word,
) -> IoResult<bool, S::Error>
where
    S: ContractsAssetsStorage + ?Sized,
{
    if amount == 0 {
        // Don't update the balance if the amount is zero
        return Ok(false)
    }

    let balance = balance(storage, contract, asset_id)?;
    let balance = balance
        .checked_add(amount)
        .ok_or(PanicReason::BalanceOverflow)?;
    let old_value = storage
        .contract_asset_id_balance_replace(contract, asset_id, balance)
        .map_err(RuntimeError::Storage)?;
    Ok(old_value.is_none())
}

/// Decrease the asset balance for a contract, unless the `amount` is zero.
pub fn balance_decrease<S>(
    storage: &mut S,
    contract: &ContractId,
    asset_id: &AssetId,
    amount: Word,
) -> IoResult<(), S::Error>
where
    S: ContractsAssetsStorage + ?Sized,
{
    if amount == 0 {
        // Don't update the balance if the amount is zero
        return Ok(())
    }

    let balance = balance(storage, contract, asset_id)?;
    let balance = balance
        .checked_sub(amount)
        .ok_or(PanicReason::NotEnoughBalance)?;
    storage
        .contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::Storage)?;
    Ok(())
}
