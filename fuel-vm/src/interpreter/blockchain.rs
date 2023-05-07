use super::contract::{balance, balance_decrease, contract_size};
use super::gas::{dependent_gas_charge, ProfileGas};
use super::internal::{
    append_receipt, base_asset_balance_sub, current_contract, inc_pc, internal_contract, internal_contract_bounds,
    tx_id, AppendReceipt,
};
use super::memory::{try_mem_write, try_zeroize, OwnershipRegisters};
use super::{ExecutableTransaction, Interpreter, MemoryRange, RuntimeBalances};
use crate::arith::{add_usize, checked_add_usize, checked_add_word, checked_sub_word};
use crate::call::CallFrame;
use crate::constraints::{reg_key::*, CheckedMemConstLen, CheckedMemRange, CheckedMemValue};
use crate::context::Context;
use crate::error::{Bug, BugId, BugVariant, RuntimeError};
use crate::gas::DependentCost;
use crate::interpreter::receipts::ReceiptsCtx;
use crate::interpreter::PanicContext;
use crate::prelude::Profiler;
use crate::storage::{ContractsAssets, ContractsAssetsStorage, ContractsRawCode, InterpreterStorage, ContractInfo};
use crate::{arith, consts::*};

use fuel_asm::PanicReason;
use fuel_storage::{StorageInspect, StorageSize};
use fuel_tx::Receipt;
use fuel_types::{bytes, BlockHeight};
use fuel_types::{Address, AssetId, Bytes32, ContractId, RegisterId, Word};

use std::borrow::Borrow;
use std::ops::Range;

#[cfg(test)]
mod code_tests;
#[cfg(test)]
mod other_tests;
#[cfg(test)]
mod smo_tests;
#[cfg(test)]
mod test;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (
            SystemRegisters {
                ssp, sp, hp, fp, pc, ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let input = LoadContractCodeCtx {
            memory: &mut self.memory,
            storage: &mut self.storage,
            contract_max_size: self.params.contract_max_size,
            input_contracts: self.tx.input_contracts(),
            panic_context: &mut self.panic_context,
            ssp,
            sp,
            fp: fp.as_ref(),
            hp: hp.as_ref(),
            pc,
        };
        input.load_contract_code(a, b, c)
    }

    pub(crate) fn burn(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, pc, .. }, _) = split_registers(&mut self.registers);
        burn(&mut self.storage, &self.memory, &self.context, fp.as_ref(), pc, a)
    }

    pub(crate) fn mint(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, pc, .. }, _) = split_registers(&mut self.registers);
        mint(&mut self.storage, &self.memory, &self.context, fp.as_ref(), pc, a)
    }

    pub(crate) fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let input = CodeCopyCtx {
            memory: &mut self.memory,
            input_contracts: self.tx.input_contracts(),
            panic_context: &mut self.panic_context,
            storage: &mut self.storage,
            owner,
            pc: self.registers.pc_mut(),
        };
        input.code_copy(a, b, c, d)
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        block_hash(&self.storage, &mut self.memory, owner, self.registers.pc_mut(), a, b)
    }

    pub(crate) fn block_height(&mut self, ra: RegisterId) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        block_height(&self.context, pc, result)
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        coinbase(&self.storage, &mut self.memory, owner, self.registers.pc_mut(), a)
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        code_root(&self.storage, &mut self.memory, owner, self.registers.pc_mut(), a, b)
    }

    pub(crate) fn code_size(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        let current_contract = current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?.copied();
        let (SystemRegisters { cgas, ggas, pc, is, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = CodeSizeCtx {
            memory: &mut self.memory,
            storage: &mut self.storage,
            gas_cost: self.gas_costs.csiz,
            profiler: &mut self.profiler,
            current_contract,
            cgas,
            ggas,
            pc,
            is: is.as_ref(),
        };
        input.code_size(result, b)
    }

    pub(crate) fn state_clear_qword(&mut self, a: Word, rb: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let contract_id = self.internal_contract().copied();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateClearQWord::new(a, c)?;
        let Self {
            ref mut storage,
            ref memory,
            ..
        } = self;

        state_clear_qword(&contract_id?, storage, memory, pc, result, input)
    }

    pub(crate) fn state_read_word(&mut self, ra: RegisterId, rb: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, pc, .. }, mut w) = split_registers(&mut self.registers);
        let (result, got_result) = w
            .get_mut_two(WriteRegKey::try_from(ra)?, WriteRegKey::try_from(rb)?)
            .ok_or(RuntimeError::Recoverable(PanicReason::ReservedRegisterNotWritable))?;
        let Self {
            ref mut storage,
            ref memory,
            ref context,
            ..
        } = self;
        state_read_word(
            StateWordCtx {
                storage,
                memory,
                context,
                fp: fp.as_ref(),
                pc,
            },
            result,
            got_result,
            c,
        )
    }

    pub(crate) fn state_read_qword(&mut self, a: Word, rb: RegisterId, c: Word, d: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let contract_id = self.internal_contract().copied();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateReadQWord::new(a, c, d, owner)?;
        let Self {
            ref storage,
            ref mut memory,
            ..
        } = self;

        state_read_qword(&contract_id?, storage, memory, pc, result, input)
    }

    pub(crate) fn state_write_word(&mut self, a: Word, rb: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, pc, .. }, mut w) = split_registers(&mut self.registers);
        let exists = &mut w[WriteRegKey::try_from(rb)?];
        let Self {
            ref mut storage,
            ref memory,
            ref context,
            ..
        } = self;
        state_write_word(
            StateWordCtx {
                storage,
                memory,
                context,
                fp: fp.as_ref(),
                pc,
            },
            a,
            exists,
            c,
        )
    }

    pub(crate) fn state_write_qword(&mut self, a: Word, rb: RegisterId, c: Word, d: Word) -> Result<(), RuntimeError> {
        let contract_id = self.internal_contract().copied();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateWriteQWord::new(a, c, d)?;
        let Self {
            ref mut storage,
            ref mut memory,
            ..
        } = self;

        state_write_qword(&contract_id?, storage, memory.as_mut(), pc, result, input)
    }

    pub(crate) fn timestamp(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        let block_height = self.get_block_height()?;
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        timestamp(&self.storage, block_height, pc, result, b)
    }

    pub(crate) fn message_output(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, pc, .. }, _) = split_registers(&mut self.registers);
        let input = MessageOutputCtx {
            max_message_data_length: self.params.max_message_data_length,
            memory: &mut self.memory,
            tx_offset: self.params.tx_offset(),
            receipts: &mut self.receipts,
            tx: &mut self.tx,
            balances: &mut self.balances,
            storage: &mut self.storage,
            current_contract: self.frames.last().map(|frame| frame.to()).copied(),
            fp: fp.as_ref(),
            pc,
            recipient_mem_address: a,
            msg_data_ptr: b,
            msg_data_len: c,
            amount_coins_to_send: d,
        };
        input.message_output()
    }
}

struct LoadContractCodeCtx<'vm, S, I> {
    contract_max_size: u64,
    memory: &'vm mut [u8; MEM_SIZE],
    input_contracts: I,
    panic_context: &'vm mut PanicContext,
    storage: &'vm S,
    ssp: RegMut<'vm, SSP>,
    sp: RegMut<'vm, SP>,
    fp: Reg<'vm, FP>,
    hp: Reg<'vm, HP>,
    pc: RegMut<'vm, PC>,
}

impl<'vm, S, I> LoadContractCodeCtx<'vm, S, I> {
    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError>
    where
        I: Iterator<Item = &'vm ContractId>,
        S: InterpreterStorage,
    {
        let ssp = *self.ssp;
        let sp = *self.sp;
        let fp = *self.fp as usize;

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into());
        }

        let contract_id = a as usize;
        let contract_id_end = checked_add_usize(ContractId::LEN, contract_id)?;
        let contract_offset = b as usize;
        let length = bytes::padded_len_usize(c as usize);

        let memory_offset = ssp as usize;
        let memory_offset_end = checked_add_usize(memory_offset, length)?;

        // Validate arguments
        if memory_offset_end >= *self.hp as usize
            || contract_id_end as Word > VM_MAX_RAM
            || length > MEM_MAX_ACCESS_SIZE as usize
            || length > self.contract_max_size as usize
        {
            return Err(PanicReason::MemoryOverflow.into());
        }

        // compiler will optimize to memset
        self.memory[memory_offset..memory_offset_end]
            .iter_mut()
            .for_each(|m| *m = 0);

        // fetch the contract id
        let contract_id: &[u8; ContractId::LEN] = &self.memory[contract_id..contract_id_end]
            .try_into()
            .expect("This can't fail, because we checked the bounds above.");

        // Safety: Memory bounds are checked and consistent
        let contract_id = ContractId::from_bytes_ref(contract_id);

        // the contract must be declared in the transaction inputs
        if !self.input_contracts.any(|id| id == contract_id) {
            *self.panic_context = PanicContext::ContractId(*contract_id);
            return Err(PanicReason::ContractNotInInputs.into());
        };

        // fetch the storage contract
        let contract = super::contract::contract(self.storage, contract_id)?;
        let contract = contract.as_ref().as_ref();

        if contract_offset > contract.len() {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let contract = &contract[contract_offset..];
        let len = contract.len().min(length);

        let code = &contract[..len];

        let memory = self
            .memory
            .get_mut(memory_offset..arith::checked_add_usize(memory_offset, len)?)
            .ok_or(PanicReason::MemoryOverflow)?;

        // perform the code copy
        memory.copy_from_slice(code);

        self.sp
            //TODO this is looser than the compare against [RegId::HP,RegId::SSP+length]
            .checked_add(length as Word)
            .map(|sp| {
                *self.sp = sp;
                *self.ssp = sp;
            })
            .ok_or_else(|| Bug::new(BugId::ID007, BugVariant::StackPointerOverflow))?;

        // update frame pointer, if we have a stack frame (e.g. fp > 0)
        if fp > 0 {
            let fp_code_size = add_usize(fp, CallFrame::code_size_offset());
            let fp_code_size_end = add_usize(fp_code_size, WORD_SIZE);

            let length = Word::from_be_bytes(
                self.memory[fp_code_size..fp_code_size_end]
                    .try_into()
                    .map_err(|_| PanicReason::MemoryOverflow)?,
            )
            .checked_add(length as Word)
            .ok_or(PanicReason::MemoryOverflow)?;

            self.memory[fp_code_size..fp_code_size_end].copy_from_slice(&length.to_be_bytes());
        }

        inc_pc(self.pc)
    }
}

pub(crate) fn burn<S>(
    storage: &mut S,
    memory: &[u8; MEM_SIZE],
    context: &Context,
    fp: Reg<FP>,
    pc: RegMut<PC>,
    a: Word,
) -> Result<(), RuntimeError>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    let range = internal_contract_bounds(context, fp)?;

    let contract = ContractId::from_bytes_ref(range.clone().read(memory));
    let asset_id = AssetId::from_bytes_ref(range.read(memory));

    let balance = balance(storage, contract, asset_id)?;
    let balance = balance.checked_sub(a).ok_or(PanicReason::NotEnoughBalance)?;

    storage
        .merkle_contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::from_io)?;

    inc_pc(pc)
}

pub(crate) fn mint<S>(
    storage: &mut S,
    memory: &[u8; MEM_SIZE],
    context: &Context,
    fp: Reg<FP>,
    pc: RegMut<PC>,
    a: Word,
) -> Result<(), RuntimeError>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    let range = internal_contract_bounds(context, fp)?;

    let contract = ContractId::from_bytes_ref(range.clone().read(memory));
    let asset_id = AssetId::from_bytes_ref(range.read(memory));

    let balance = balance(storage, contract, asset_id)?;
    let balance = checked_add_word(balance, a)?;

    storage
        .merkle_contract_asset_id_balance_insert(contract, asset_id, balance)
        .map_err(RuntimeError::from_io)?;

    inc_pc(pc)
}

struct CodeCopyCtx<'vm, S, I> {
    memory: &'vm mut [u8; MEM_SIZE],
    input_contracts: I,
    panic_context: &'vm mut PanicContext,
    storage: &'vm S,
    owner: OwnershipRegisters,
    pc: RegMut<'vm, PC>,
}

impl<'vm, S, I> CodeCopyCtx<'vm, S, I> {
    pub(crate) fn code_copy(mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError>
    where
        I: Iterator<Item = &'vm ContractId>,
        S: InterpreterStorage,
    {
        let contract = CheckedMemConstLen::<{ ContractId::LEN }>::new(b)?;
        let cd = checked_add_word(c, d)?;

        if d > MEM_MAX_ACCESS_SIZE || a > checked_sub_word(VM_MAX_RAM, d)? || cd > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, c, d) = (a as usize, c as usize, d as usize);
        let cd = cd as usize;

        let contract = ContractId::from_bytes_ref(contract.read(self.memory));

        if !self.input_contracts.any(|input| input == contract) {
            *self.panic_context = PanicContext::ContractId(*contract);
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let contract = super::contract::contract(self.storage, contract)?.into_owned();

        if contract.as_ref().len() < d {
            try_zeroize(a, d, self.owner, self.memory)?;
        } else {
            try_mem_write(a, &contract.as_ref()[c..cd], self.owner, self.memory)?;
        }

        inc_pc(self.pc)
    }
}

pub(crate) fn block_hash<S: InterpreterStorage>(
    storage: &S,
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> Result<(), RuntimeError> {
    let height = u32::try_from(b).map_err(|_| PanicReason::ArithmeticOverflow)?.into();
    let hash = storage.block_hash(height).map_err(|e| e.into())?;

    try_mem_write(a as usize, hash.as_ref(), owner, memory)?;

    inc_pc(pc)
}

pub(crate) fn block_height(context: &Context, pc: RegMut<PC>, result: &mut Word) -> Result<(), RuntimeError> {
    context
        .block_height()
        .map(|h| *h as Word)
        .map(|h| *result = h)
        .ok_or(PanicReason::TransactionValidity)?;

    inc_pc(pc)
}

pub(crate) fn coinbase<S: InterpreterStorage>(
    storage: &S,
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
) -> Result<(), RuntimeError> {
    storage
        .coinbase()
        .map_err(RuntimeError::from_io)
        .and_then(|data| try_mem_write(a as usize, data.as_ref(), owner, memory))?;

    inc_pc(pc)
}

pub(crate) fn code_root<S>(
    storage: &S,
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> Result<(), RuntimeError>
where
    S: InterpreterStorage,
{
    let ax = checked_add_word(a, Bytes32::LEN as Word)?;
    let contract_id = CheckedMemConstLen::<{ ContractId::LEN }>::new(b)?;

    if ax > VM_MAX_RAM {
        return Err(PanicReason::MemoryOverflow.into());
    }

    let contract_id = ContractId::from_bytes_ref(contract_id.read(memory));

    let ContractInfo{ root, ..} = storage
        .storage_contract_root(contract_id)
        .transpose()
        .ok_or(PanicReason::ContractNotFound)?
        .map_err(RuntimeError::from_io)?
        .into_owned();

    try_mem_write(a as usize, root.as_ref(), owner, memory)?;

    inc_pc(pc)
}

struct CodeSizeCtx<'vm, S> {
    storage: &'vm S,
    memory: &'vm mut [u8; MEM_SIZE],
    gas_cost: DependentCost,
    profiler: &'vm mut Profiler,
    current_contract: Option<ContractId>,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S> CodeSizeCtx<'vm, S> {
    pub(crate) fn code_size(self, result: &mut Word, b: Word) -> Result<(), RuntimeError>
    where
        S: StorageSize<ContractsRawCode>,
        <S as StorageInspect<ContractsRawCode>>::Error: Into<std::io::Error>,
    {
        let contract_id = CheckedMemConstLen::<{ ContractId::LEN }>::new(b)?;

        let contract_id = ContractId::from_bytes_ref(contract_id.read(self.memory));

        let len = contract_size(self.storage, contract_id)?;
        let profiler = ProfileGas {
            pc: self.pc.as_ref(),
            is: self.is,
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge(self.cgas, self.ggas, profiler, self.gas_cost, len)?;
        *result = len;

        inc_pc(self.pc)
    }
}

pub(crate) struct StateWordCtx<'vm, S> {
    pub storage: &'vm mut S,
    pub memory: &'vm [u8; MEM_SIZE],
    pub context: &'vm Context,
    pub fp: Reg<'vm, FP>,
    pub pc: RegMut<'vm, PC>,
}

pub(crate) fn state_read_word<S: InterpreterStorage>(
    StateWordCtx {
        storage,
        memory,
        context,
        fp,
        pc,
    }: StateWordCtx<S>,
    result: &mut Word,
    got_result: &mut Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let key = CheckedMemConstLen::<{ Bytes32::LEN }>::new(c)?;

    let contract = internal_contract(context, fp, memory)?;

    let key = Bytes32::from_bytes_ref(key.read(memory));

    let value = storage
        .merkle_contract_state(contract, key)
        .map_err(RuntimeError::from_io)?
        .map(|state| bytes::from_array(state.as_ref().borrow()))
        .map(Word::from_be_bytes);

    *result = value.unwrap_or(0);
    *got_result = value.is_some() as Word;

    inc_pc(pc)
}

pub(crate) fn state_write_word<S: InterpreterStorage>(
    StateWordCtx {
        storage,
        memory,
        context,
        fp,
        pc,
    }: StateWordCtx<S>,
    a: Word,
    exists: &mut Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let key = CheckedMemConstLen::<{ Bytes32::LEN }>::new(a)?;

    let contract = internal_contract_bounds(context, fp)?;

    // Safety: Memory bounds logically verified by the interpreter
    let contract = ContractId::from_bytes_ref(contract.read(memory));
    let key = Bytes32::from_bytes_ref(key.read(memory));

    let mut value = Bytes32::default();

    value[..WORD_SIZE].copy_from_slice(&c.to_be_bytes());

    let result = storage
        .merkle_contract_state_insert(contract, key, &value)
        .map_err(RuntimeError::from_io)?;

    *exists = result.is_some() as Word;

    inc_pc(pc)
}

pub(crate) fn timestamp(
    storage: &impl InterpreterStorage,
    block_height: BlockHeight,
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
) -> Result<(), RuntimeError> {
    let b = u32::try_from(b).map_err(|_| PanicReason::ArithmeticOverflow)?.into();
    (b <= block_height)
        .then_some(())
        .ok_or(PanicReason::TransactionValidity)?;

    *result = storage.timestamp(b).map_err(|e| e.into())?;

    inc_pc(pc)
}
struct MessageOutputCtx<'vm, Tx, S>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    max_message_data_length: u64,
    memory: &'vm mut [u8; MEM_SIZE],
    tx_offset: usize,
    receipts: &'vm mut ReceiptsCtx,
    tx: &'vm mut Tx,
    balances: &'vm mut RuntimeBalances,
    storage: &'vm mut S,
    current_contract: Option<ContractId>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    /// A
    recipient_mem_address: Word,
    /// B
    msg_data_ptr: Word,
    /// C
    msg_data_len: Word,
    /// D
    amount_coins_to_send: Word,
}

impl<Tx, S> MessageOutputCtx<'_, Tx, S>
where
    S: ContractsAssetsStorage + ?Sized,
    <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
{
    pub(crate) fn message_output(self) -> Result<(), RuntimeError>
    where
        Tx: ExecutableTransaction,
    {
        let recipient_address = CheckedMemValue::<Address>::new::<{ Address::LEN }>(self.recipient_mem_address)?;

        if self.msg_data_len > MEM_MAX_ACCESS_SIZE {
            return Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow));
        }

        if self.msg_data_len > self.max_message_data_length {
            return Err(RuntimeError::Recoverable(PanicReason::MessageDataTooLong));
        }

        let msg_data_range = CheckedMemRange::new(self.msg_data_ptr, self.msg_data_len as usize)?;

        let recipient = recipient_address.try_from(self.memory)?;

        // validations passed, perform the mutations

        if let Some(source_contract) = self.current_contract {
            balance_decrease(
                self.storage,
                &source_contract,
                &AssetId::BASE,
                self.amount_coins_to_send,
            )?;
        } else {
            base_asset_balance_sub(self.balances, self.memory, self.amount_coins_to_send)?;
        }

        let sender = CheckedMemConstLen::<{ Address::LEN }>::new(*self.fp)?;
        let txid = tx_id(self.memory);
        let msg_data = msg_data_range.read(self.memory).to_vec();
        let sender = Address::from_bytes_ref(sender.read(self.memory));

        let receipt = Receipt::message_out_from_tx_output(
            txid,
            self.receipts.len() as Word,
            *sender,
            recipient,
            self.amount_coins_to_send,
            msg_data,
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

struct StateReadQWord {
    /// The destination memory address is
    /// stored in this range of memory.
    destination_address_memory_range: Range<usize>,
    /// The starting storage key location is stored
    /// in this range of memory.
    origin_key_memory_range: CheckedMemConstLen<{ Bytes32::LEN }>,
    /// Number of slots to read.
    num_slots: Word,
}

impl StateReadQWord {
    fn new(
        destination_memory_address: Word,
        origin_key_memory_address: Word,
        num_slots: Word,
        ownership_registers: OwnershipRegisters,
    ) -> Result<Self, RuntimeError> {
        let mem_range = MemoryRange::new(
            destination_memory_address,
            (Bytes32::LEN as Word).saturating_mul(num_slots),
        );
        if !ownership_registers.has_ownership_range(&mem_range) {
            return Err(PanicReason::MemoryOwnership.into());
        }
        if ownership_registers.context.is_external() {
            return Err(PanicReason::ExpectedInternalContext.into());
        }
        let dest_end = checked_add_word(
            destination_memory_address,
            Bytes32::LEN.saturating_mul(num_slots as usize) as Word,
        )?;
        let origin_key_memory_range = CheckedMemConstLen::<{ Bytes32::LEN }>::new(origin_key_memory_address)?;
        if dest_end > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }
        Ok(Self {
            destination_address_memory_range: (destination_memory_address as usize)..(dest_end as usize),
            origin_key_memory_range,
            num_slots,
        })
    }
}

fn state_read_qword(
    contract_id: &ContractId,
    storage: &impl InterpreterStorage,
    memory: &mut [u8; MEM_SIZE],
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateReadQWord,
) -> Result<(), RuntimeError> {
    let origin_key = Bytes32::from_bytes_ref(input.origin_key_memory_range.read(memory));

    let mut all_set = true;
    let result: Vec<u8> = storage
        .merkle_contract_state_range(contract_id, origin_key, input.num_slots)
        .map_err(RuntimeError::from_io)?
        .into_iter()
        .flat_map(|bytes| match bytes {
            Some(bytes) => **bytes,
            None => {
                all_set = false;
                *Bytes32::zeroed()
            }
        })
        .collect();

    *result_register = all_set as Word;

    memory[input.destination_address_memory_range].copy_from_slice(&result);

    inc_pc(pc)?;

    Ok(())
}

struct StateWriteQWord {
    /// The starting storage key location is stored
    /// in this range of memory.
    starting_storage_key_memory_range: CheckedMemConstLen<{ Bytes32::LEN }>,
    /// The source data memory address is
    /// stored in this range of memory.
    source_address_memory_range: Range<usize>,
}

impl StateWriteQWord {
    fn new(
        starting_storage_key_memory_address: Word,
        source_memory_address: Word,
        num_slots: Word,
    ) -> Result<Self, RuntimeError> {
        let source_end = checked_add_word(
            source_memory_address,
            Bytes32::LEN.saturating_mul(num_slots as usize) as Word,
        )?;
        let starting_storage_key_memory_range =
            CheckedMemConstLen::<{ Bytes32::LEN }>::new(starting_storage_key_memory_address)?;
        if source_end > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }
        Ok(Self {
            source_address_memory_range: (source_memory_address as usize)..(source_end as usize),
            starting_storage_key_memory_range,
        })
    }
}

fn state_write_qword(
    contract_id: &ContractId,
    storage: &mut impl InterpreterStorage,
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateWriteQWord,
) -> Result<(), RuntimeError> {
    let destination_key = Bytes32::from_bytes_ref(input.starting_storage_key_memory_range.read(memory));

    let values: Vec<_> = memory[input.source_address_memory_range]
        .chunks_exact(Bytes32::LEN)
        .flat_map(|chunk| Some(Bytes32::from(<[u8; 32]>::try_from(chunk).ok()?)))
        .collect();

    let any_none = storage
        .merkle_contract_state_insert_range(contract_id, destination_key, &values)
        .map_err(RuntimeError::from_io)?
        .is_some();
    *result_register = any_none as Word;

    inc_pc(pc)?;

    Ok(())
}

struct StateClearQWord {
    /// The starting storage key location is stored
    /// in this range of memory.
    start_storage_key_memory_range: CheckedMemConstLen<{ Bytes32::LEN }>,
    /// Number of slots to read.
    num_slots: Word,
}

impl StateClearQWord {
    fn new(start_storage_key_memory_address: Word, num_slots: Word) -> Result<Self, RuntimeError> {
        let start_storage_key_memory_range =
            CheckedMemConstLen::<{ Bytes32::LEN }>::new(start_storage_key_memory_address)?;
        Ok(Self {
            start_storage_key_memory_range,
            num_slots,
        })
    }
}

fn state_clear_qword(
    contract_id: &ContractId,
    storage: &mut impl InterpreterStorage,
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateClearQWord,
) -> Result<(), RuntimeError> {
    let start_key = Bytes32::from_bytes_ref(input.start_storage_key_memory_range.read(memory));

    let all_previously_set = storage
        .merkle_contract_state_remove_range(contract_id, start_key, input.num_slots)
        .map_err(RuntimeError::from_io)?
        .is_some();

    *result_register = all_previously_set as Word;

    inc_pc(pc)?;

    Ok(())
}
