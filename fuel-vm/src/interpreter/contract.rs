//! This module contains logic on contract management.

use super::{
    gas::{
        gas_charge,
        ProfileGas,
    },
    internal::{
        external_asset_id_balance_sub,
        inc_pc,
        internal_contract,
        set_variable_output,
    },
    ExecutableTransaction,
    Interpreter,
    Memory,
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
    interpreter::{
        receipts::ReceiptsCtx,
        InputContracts,
        PanicContext,
    },
    prelude::Profiler,
    storage::{
        ContractsAssetsStorage,
        ContractsRawCode,
        InterpreterStorage,
    },
};
use fuel_asm::{
    PanicReason,
    RegisterId,
    Word,
};
use fuel_storage::StorageSize;
use fuel_tx::{
    Contract,
    Output,
    Receipt,
};
use fuel_types::{
    Address,
    AssetId,
    Bytes32,
    ContractId,
};

use alloc::borrow::Cow;

#[cfg(test)]
mod tests;

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    pub(crate) fn contract_balance(
        &mut self,
        ra: RegisterId,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = ContractBalanceCtx {
            storage: &self.storage,
            memory: &mut self.memory,
            pc,
            input_contracts: InputContracts::new(
                self.tx.input_contracts(),
                &mut self.panic_context,
            ),
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
            memory: &mut self.memory,
            context: &self.context,
            balances: &mut self.balances,
            receipts: &mut self.receipts,
            profiler: &mut self.profiler,
            new_storage_gas_per_byte,
            tx: &mut self.tx,
            tx_offset,
            cgas,
            ggas,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.transfer(&mut self.panic_context, a, b, c)
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
            memory: &mut self.memory,
            context: &self.context,
            balances: &mut self.balances,
            receipts: &mut self.receipts,
            profiler: &mut self.profiler,
            new_storage_gas_per_byte,
            tx: &mut self.tx,
            tx_offset,
            cgas,
            ggas,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
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

pub(crate) fn contract<'s, S>(
    storage: &'s S,
    contract: &ContractId,
) -> IoResult<Cow<'s, Contract>, S::DataError>
where
    S: InterpreterStorage,
{
    storage
        .storage_contract(contract)
        .map_err(RuntimeError::Storage)?
        .ok_or_else(|| PanicReason::ContractNotFound.into())
}

struct ContractBalanceCtx<'vm, S, I> {
    storage: &'vm S,
    memory: &'vm mut Memory,
    pc: RegMut<'vm, PC>,
    input_contracts: InputContracts<'vm, I>,
}

impl<'vm, S, I> ContractBalanceCtx<'vm, S, I> {
    pub(crate) fn contract_balance(
        mut self,
        result: &mut Word,
        b: Word,
        c: Word,
    ) -> IoResult<(), S::Error>
    where
        I: Iterator<Item = &'vm ContractId>,
        S: ContractsAssetsStorage,
    {
        let asset_id = AssetId::new(self.memory.read_bytes(b)?);
        let contract = ContractId::new(self.memory.read_bytes(c)?);

        self.input_contracts.check(&contract)?;

        let balance = balance(self.storage, &contract, &asset_id)?;

        *result = balance;

        Ok(inc_pc(self.pc)?)
    }
}
struct TransferCtx<'vm, S, Tx> {
    storage: &'vm mut S,
    memory: &'vm mut Memory,
    context: &'vm Context,
    balances: &'vm mut RuntimeBalances,
    receipts: &'vm mut ReceiptsCtx,
    profiler: &'vm mut Profiler,
    new_storage_gas_per_byte: Word,
    tx: &'vm mut Tx,
    tx_offset: usize,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    fp: Reg<'vm, FP>,
    is: Reg<'vm, IS>,
    pc: RegMut<'vm, PC>,
}

impl<'vm, S, Tx> TransferCtx<'vm, S, Tx> {
    /// In Fuel specs:
    /// Transfer $rB coins with asset ID at $rC to contract with ID at $rA.
    /// $rA -> recipient_contract_id_offset
    /// $rB -> transfer_amount
    /// $rC -> asset_id_offset
    pub(crate) fn transfer(
        self,
        panic_context: &mut PanicContext,
        recipient_contract_id_offset: Word,
        transfer_amount: Word,
        asset_id_offset: Word,
    ) -> IoResult<(), S::Error>
    where
        Tx: ExecutableTransaction,
        S: ContractsAssetsStorage,
    {
        let amount = transfer_amount;
        let destination =
            ContractId::from(self.memory.read_bytes(recipient_contract_id_offset)?);
        let asset_id = AssetId::from(self.memory.read_bytes(asset_id_offset)?);

        InputContracts::new(self.tx.input_contracts(), panic_context)
            .check(&destination)?;

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
        let (_, created_new_entry) =
            balance_increase(self.storage, &destination, &asset_id, amount)?;
        if created_new_entry {
            // If a new entry was created, we must charge gas for it
            let profiler = ProfileGas {
                pc: self.pc.as_ref(),
                is: self.is,
                current_contract: internal_context,
                profiler: self.profiler,
            };
            gas_charge(
                self.cgas,
                self.ggas,
                profiler,
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
    Ok(storage
        .size_of_value(contract)
        .map_err(RuntimeError::Storage)?
        .ok_or(PanicReason::ContractNotFound)?)
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
/// Returns new balance, and a boolean indicating if a new entry was created.
pub fn balance_increase<S>(
    storage: &mut S,
    contract: &ContractId,
    asset_id: &AssetId,
    amount: Word,
) -> IoResult<(Word, bool), S::Error>
where
    S: ContractsAssetsStorage + ?Sized,
{
    let balance = balance(storage, contract, asset_id)?;
    let balance = balance
        .checked_add(amount)
        .ok_or(PanicReason::BalanceOverflow)?;

    let old_value = storage
        .contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::Storage)?;

    Ok((balance, old_value.is_none()))
}

/// Decrease the asset balance for a contract.
pub fn balance_decrease<S>(
    storage: &mut S,
    contract: &ContractId,
    asset_id: &AssetId,
    amount: Word,
) -> IoResult<Word, S::Error>
where
    S: ContractsAssetsStorage + ?Sized,
{
    let balance = balance(storage, contract, asset_id)?;
    let balance = balance
        .checked_sub(amount)
        .ok_or(PanicReason::NotEnoughBalance)?;
    let _ = storage
        .contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::Storage)?;
    Ok(balance)
}
