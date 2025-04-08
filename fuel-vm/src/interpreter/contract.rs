//! This module contains logic on contract management.

use alloc::collections::BTreeSet;

use super::{
    gas::gas_charge,
    internal::{
        external_asset_id_balance_sub,
        inc_pc,
        internal_contract,
        set_variable_output,
    },
    ExecutableTransaction,
    Interpreter,
    Memory,
    MemoryInstance,
    PanicContext,
    RuntimeBalances,
};
use crate::{
    constraints::reg_key::*,
    consts::*,
    context::Context,
    convert,
    error::{
        IoResult,
        RuntimeError,
    },
    interpreter::receipts::ReceiptsCtx,
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
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = ContractBalanceCtx {
            storage: &self.storage,
            memory: self.memory.as_mut(),
            pc,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            verifier: &mut self.verifier,
        };
        input.contract_balance(result, b, c)?;
        Ok(())
    }

    pub(crate) fn transfer(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        let tx_offset = self.tx_offset();
        let (
            SystemRegisters {
                cgas,
                ggas,
                fp,
                is,
                pc,
                ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let input = TransferCtx {
            storage: &mut self.storage,
            memory: self.memory.as_mut(),
            context: &self.context,
            balances: &mut self.balances,
            receipts: &mut self.receipts,
            new_storage_gas_per_byte,
            tx: &mut self.tx,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            tx_offset,
            cgas,
            ggas,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
            verifier: &mut self.verifier,
        };
        input.transfer(a, b, c)
    }

    pub(crate) fn transfer_output(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let tx_offset = self.tx_offset();
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        let (
            SystemRegisters {
                cgas,
                ggas,
                fp,
                is,
                pc,
                ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let input = TransferCtx {
            storage: &mut self.storage,
            memory: self.memory.as_mut(),
            context: &self.context,
            balances: &mut self.balances,
            receipts: &mut self.receipts,
            new_storage_gas_per_byte,
            tx: &mut self.tx,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            tx_offset,
            cgas,
            ggas,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
            verifier: &mut self.verifier,
        };
        input.transfer_output(a, b, c, d)
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

struct ContractBalanceCtx<'vm, S, V> {
    storage: &'vm S,
    memory: &'vm mut MemoryInstance,
    pc: RegMut<'vm, PC>,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    verifier: &'vm mut V,
}

impl<S, V> ContractBalanceCtx<'_, S, V> {
    pub(crate) fn contract_balance(
        self,
        result: &mut Word,
        b: Word,
        c: Word,
    ) -> IoResult<(), S::Error>
    where
        S: ContractsAssetsStorage,
        V: Verifier,
    {
        let asset_id = AssetId::new(self.memory.read_bytes(b)?);
        let contract_id = ContractId::new(self.memory.read_bytes(c)?);

        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            &contract_id,
        )?;

        let balance = balance(self.storage, &contract_id, &asset_id)?;

        *result = balance;

        Ok(inc_pc(self.pc)?)
    }
}
struct TransferCtx<'vm, S, Tx, V> {
    storage: &'vm mut S,
    memory: &'vm mut MemoryInstance,
    context: &'vm Context,
    balances: &'vm mut RuntimeBalances,
    receipts: &'vm mut ReceiptsCtx,

    new_storage_gas_per_byte: Word,
    tx: &'vm mut Tx,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    tx_offset: usize,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    fp: Reg<'vm, FP>,
    is: Reg<'vm, IS>,
    pc: RegMut<'vm, PC>,
    verifier: &'vm mut V,
}

impl<S, Tx, V> TransferCtx<'_, S, Tx, V> {
    /// In Fuel specs:
    /// Transfer $rB coins with asset ID at $rC to contract with ID at $rA.
    /// $rA -> recipient_contract_id_offset
    /// $rB -> transfer_amount
    /// $rC -> asset_id_offset
    pub(crate) fn transfer(
        self,
        recipient_contract_id_offset: Word,
        transfer_amount: Word,
        asset_id_offset: Word,
    ) -> IoResult<(), S::Error>
    where
        Tx: ExecutableTransaction,
        S: ContractsAssetsStorage,
        V: Verifier,
    {
        let amount = transfer_amount;
        let destination =
            ContractId::from(self.memory.read_bytes(recipient_contract_id_offset)?);
        let asset_id = AssetId::from(self.memory.read_bytes(asset_id_offset)?);

        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            &destination,
        )?;

        if amount == 0 {
            return Err(PanicReason::TransferZeroCoins.into())
        }

        let internal_context = match internal_contract(self.context, self.fp, self.memory)
        {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(source_contract),
            // revert to external context if no internal contract is set
            Err(PanicReason::ExpectedInternalContext) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e.into()),
        };

        if let Some(source_contract) = internal_context {
            // debit funding source (source contract balance)
            balance_decrease(self.storage, &source_contract, &asset_id, amount)?;
        } else {
            // debit external funding source (i.e. free balance)
            external_asset_id_balance_sub(self.balances, self.memory, &asset_id, amount)?;
        }
        // credit destination contract
        let created_new_entry =
            balance_increase(self.storage, &destination, &asset_id, amount)?;
        if created_new_entry {
            // If a new entry was created, we must charge gas for it
            gas_charge(
                self.cgas,
                self.ggas,
                ((Bytes32::LEN + WORD_SIZE) as u64)
                    .saturating_mul(self.new_storage_gas_per_byte),
            )?;
        }

        let receipt = Receipt::transfer(
            internal_context.unwrap_or_default(),
            destination,
            amount,
            asset_id,
            *self.pc,
            *self.is,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }

    /// In Fuel specs:
    /// Transfer $rC coins with asset ID at $rD to address at $rA, with output $rB.
    /// $rA -> recipient_offset
    /// $rB -> output_index
    /// $rC -> transfer_amount
    /// $rD -> asset_id_offset
    pub(crate) fn transfer_output(
        self,
        recipient_offset: Word,
        output_index: Word,
        transfer_amount: Word,
        asset_id_offset: Word,
    ) -> IoResult<(), S::Error>
    where
        Tx: ExecutableTransaction,
        S: ContractsAssetsStorage,
        V: Verifier,
    {
        let out_idx =
            convert::to_usize(output_index).ok_or(PanicReason::OutputNotFound)?;
        let to = Address::from(self.memory.read_bytes(recipient_offset)?);
        let asset_id = AssetId::from(self.memory.read_bytes(asset_id_offset)?);
        let amount = transfer_amount;

        if amount == 0 {
            return Err(PanicReason::TransferZeroCoins.into())
        }

        let internal_context = match internal_contract(self.context, self.fp, self.memory)
        {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(source_contract),
            // revert to external context if no internal contract is set
            Err(PanicReason::ExpectedInternalContext) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e.into()),
        };

        if let Some(source_contract) = internal_context {
            // debit funding source (source contract balance)
            balance_decrease(self.storage, &source_contract, &asset_id, amount)?;
        } else {
            // debit external funding source (i.e. UTXOs)
            external_asset_id_balance_sub(self.balances, self.memory, &asset_id, amount)?;
        }

        // credit variable output
        let variable = Output::variable(to, amount, asset_id);

        set_variable_output(self.tx, self.memory, self.tx_offset, out_idx, variable)?;

        let receipt = Receipt::transfer_out(
            internal_context.unwrap_or_default(),
            to,
            amount,
            asset_id,
            *self.pc,
            *self.is,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
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

/// Increase the asset balance for a contract.
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

/// Decrease the asset balance for a contract.
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
