use super::internal::{
    append_receipt, external_asset_id_balance_sub, inc_pc, internal_contract, set_variable_output, AppendReceipt,
};
use super::{ExecutableTransaction, Interpreter, RuntimeBalances};
use crate::constraints::reg_key::*;
use crate::context::Context;
use crate::error::RuntimeError;
use crate::interpreter::PanicContext;
use crate::storage::{ContractsAssets, ContractsAssetsStorage, InterpreterStorage};
use crate::{consts::*, storage::ContractsRawCode};

use fuel_asm::{PanicReason, RegisterId, Word};
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
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = ContractBalanceCtx {
            storage: &self.storage,
            memory: &mut self.memory,
            pc,
            panic_context: &mut self.panic_context,
            input_contracts: self.tx.input_contracts(),
        };
        input.contract_balance(result, b, c)
    }

    pub(crate) fn transfer(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, is, pc, .. }, _) = split_registers(&mut self.registers);
        let input = TransferCtx {
            storage: &mut self.storage,
            memory: &mut self.memory,
            context: &self.context,
            balances: &mut self.balances,
            receipts: &mut self.receipts,
            tx: &mut self.tx,
            tx_offset: self.params.tx_offset(),
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.transfer(&mut self.panic_context, a, b, c)
    }

    pub(crate) fn transfer_output(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, is, pc, .. }, _) = split_registers(&mut self.registers);
        let input = TransferCtx {
            storage: &mut self.storage,
            memory: &mut self.memory,
            context: &self.context,
            balances: &mut self.balances,
            receipts: &mut self.receipts,
            tx: &mut self.tx,
            tx_offset: self.params.tx_offset(),
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.transfer_output(a, b, c, d)
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

struct ContractBalanceCtx<'vm, S, I> {
    storage: &'vm S,
    memory: &'vm mut [u8; MEM_SIZE],
    pc: RegMut<'vm, PC>,
    input_contracts: I,
    panic_context: &'vm mut PanicContext,
}

impl<'vm, S, I> ContractBalanceCtx<'vm, S, I> {
    pub(crate) fn contract_balance(mut self, result: &mut Word, b: Word, c: Word) -> Result<(), RuntimeError>
    where
        I: Iterator<Item = &'vm ContractId>,
        S: ContractsAssetsStorage,
        <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
    {
        let bx = b
            .checked_add(AssetId::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow)?;

        let cx = c
            .checked_add(ContractId::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow)?;

        //if above usize::MAX then it cannot be safely cast to usize,
        // check the tighter bound between VM_MAX_RAM and usize::MAX
        if bx > MIN_VM_MAX_RAM_USIZE_MAX || cx > MIN_VM_MAX_RAM_USIZE_MAX {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (b, c, bx, cx) = (b as usize, c as usize, bx as usize, cx as usize);

        // Safety: memory bounds checked
        let asset_id = unsafe { AssetId::as_ref_unchecked(&self.memory[b..bx]) };
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };

        if !self.input_contracts.any(|input| contract == input) {
            *self.panic_context = PanicContext::ContractId(*contract);
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let balance = balance(self.storage, contract, asset_id)?;

        *result = balance;

        inc_pc(self.pc)
    }
}
struct TransferCtx<'vm, S, Tx> {
    storage: &'vm mut S,
    memory: &'vm mut [u8; MEM_SIZE],
    context: &'vm Context,
    balances: &'vm mut RuntimeBalances,
    receipts: &'vm mut Vec<Receipt>,
    tx: &'vm mut Tx,
    tx_offset: usize,
    fp: Reg<'vm, FP>,
    is: Reg<'vm, IS>,
    pc: RegMut<'vm, PC>,
}
impl<'vm, S, Tx> TransferCtx<'vm, S, Tx> {
    pub(crate) fn transfer(
        self,
        panic_context: &mut PanicContext,
        a: Word,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError>
    where
        Tx: ExecutableTransaction,
        S: ContractsAssetsStorage,
        <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
    {
        let ax = a
            .checked_add(ContractId::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow)?;

        let cx = c
            .checked_add(AssetId::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow)?;

        //if above usize::MAX then it cannot be safely cast to usize,
        // check the tighter bound between VM_MAX_RAM and usize::MAX
        if ax > MIN_VM_MAX_RAM_USIZE_MAX || cx > MIN_VM_MAX_RAM_USIZE_MAX {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let amount = b;
        let destination =
            ContractId::try_from(&self.memory[a as usize..ax as usize]).expect("Unreachable! Checked memory range");
        let asset_id =
            AssetId::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        if !self.tx.input_contracts().any(|contract| &destination == contract) {
            *panic_context = PanicContext::ContractId(destination);
            return Err(PanicReason::ContractNotInInputs.into());
        }

        if amount == 0 {
            return Err(PanicReason::NotEnoughBalance.into());
        }

        let internal_context = match internal_contract(self.context, self.fp, self.memory) {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(*source_contract),
            // revert to external context if no internal contract is set
            Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e),
        };

        if let Some(source_contract) = internal_context {
            // debit funding source (source contract balance)
            balance_decrease(self.storage, &source_contract, &asset_id, amount)?;
        } else {
            // debit external funding source (i.e. free balance)
            external_asset_id_balance_sub(self.balances, self.memory, &asset_id, amount)?;
        }
        // credit destination contract
        balance_increase(self.storage, &destination, &asset_id, amount)?;

        let receipt = Receipt::transfer(
            internal_context.unwrap_or_default(),
            destination,
            amount,
            asset_id,
            *self.pc,
            *self.is,
        );

        append_receipt(
            AppendReceipt {
                receipts: self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset: self.tx_offset,
                memory: self.memory,
            },
            receipt,
        );

        inc_pc(self.pc)
    }

    pub(crate) fn transfer_output(self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError>
    where
        Tx: ExecutableTransaction,
        S: ContractsAssetsStorage,
        <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
    {
        let ax = a
            .checked_add(ContractId::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow)?;

        let dx = d
            .checked_add(AssetId::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow)?;

        //if above usize::MAX then it cannot be safely cast to usize,
        // check the tighter bound between VM_MAX_RAM and usize::MAX
        if ax > MIN_VM_MAX_RAM_USIZE_MAX || dx > MIN_VM_MAX_RAM_USIZE_MAX {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let out_idx = b as usize;
        let to = Address::try_from(&self.memory[a as usize..ax as usize]).expect("Unreachable! Checked memory range");
        let asset_id =
            AssetId::try_from(&self.memory[d as usize..dx as usize]).expect("Unreachable! Checked memory range");
        let amount = c;

        let internal_context = match internal_contract(self.context, self.fp, self.memory) {
            // optimistically attempt to load the internal contract id
            Ok(source_contract) => Some(*source_contract),
            // revert to external context if no internal contract is set
            Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)) => None,
            // bubble up any other kind of errors
            Err(e) => return Err(e),
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

        append_receipt(
            AppendReceipt {
                receipts: self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset: self.tx_offset,
                memory: self.memory,
            },
            receipt,
        );

        inc_pc(self.pc)
    }
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
