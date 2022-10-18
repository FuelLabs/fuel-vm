use super::{ExecutableTransaction, Interpreter};
use crate::call::CallFrame;
use crate::consts::*;
use crate::error::{Bug, BugId, BugVariant, RuntimeError};
use crate::storage::InterpreterStorage;

use fuel_asm::PanicReason;
use fuel_tx::{Input, Output, Receipt};
use fuel_types::bytes::{self, Deserializable};
use fuel_types::{Address, AssetId, Bytes32, Bytes8, ContractId, RegisterId, Word};

use crate::arith::{add_usize, checked_add_usize, checked_add_word, checked_sub_word};
use core::{mem, slice};

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    pub(crate) fn coinbase(&self) -> Result<Address, RuntimeError> {
        self.storage.coinbase().map_err(RuntimeError::from_io)
    }

    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let ssp = self.registers[REG_SSP];
        let sp = self.registers[REG_SP];
        let fp = self.registers[REG_FP] as usize;

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
        if memory_offset_end > self.registers[REG_HP] as usize
            || contract_id_end as Word > VM_MAX_RAM
            || length > MEM_MAX_ACCESS_SIZE as usize
            || length > self.params.contract_max_size as usize
        {
            return Err(PanicReason::MemoryOverflow.into());
        }

        // compiler will optimize to memset
        self.memory[memory_offset..memory_offset_end]
            .iter_mut()
            .for_each(|m| *m = 0);

        // fetch the contract id
        let contract_id = &self.memory[contract_id..contract_id_end];

        // Safety: Memory bounds are checked and consistent
        let contract_id = unsafe { ContractId::as_ref_unchecked(contract_id) };

        // the contract must be declared in the transaction inputs
        if !self.transaction().input_contracts().any(|id| id == contract_id) {
            return Err(PanicReason::ContractNotInInputs.into());
        };

        // fetch the storage contract
        let contract = self.contract(contract_id)?;
        let contract = contract.as_ref().as_ref();

        if contract_offset > contract.len() {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let contract = &contract[contract_offset..];
        let len = contract.len().min(length);

        let code = &contract[..len];

        // Safety: all bounds are checked
        let memory = &self.memory[memory_offset] as *const u8;
        let memory = unsafe { slice::from_raw_parts_mut(memory as *mut u8, len) };

        // perform the code copy
        memory.copy_from_slice(code);

        self.registers[REG_SP]
            //TODO this is looser than the compare against [REG_HP,REG_SSP+length]
            .checked_add(length as Word)
            .map(|sp| {
                self.registers[REG_SP] = sp;
                self.registers[REG_SSP] = sp;
            })
            .ok_or_else(|| Bug::new(BugId::ID007, BugVariant::StackPointerOverflow))?;

        // update frame pointer, if we have a stack frame (e.g. fp > 0)
        if fp > 0 {
            let fpx = add_usize(fp, CallFrame::code_size_offset());

            self.memory[fp..fpx].copy_from_slice(&length.to_be_bytes());
        }

        self.inc_pc()
    }

    pub(crate) fn burn(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let asset_id = unsafe { AssetId::as_ref_unchecked(&self.memory[c..cx]) };

        let balance = self.balance(contract, asset_id)?;
        let balance = balance.checked_sub(a).ok_or(PanicReason::NotEnoughBalance)?;

        self.storage
            .merkle_contract_asset_id_balance_insert(contract, asset_id, balance)
            .map_err(RuntimeError::from_io)?;

        self.inc_pc()
    }

    pub(crate) fn mint(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let asset_id = unsafe { AssetId::as_ref_unchecked(&self.memory[c..cx]) };

        let balance = self.balance(contract, asset_id)?;
        let balance = checked_add_word(balance, a)?;

        self.storage
            .merkle_contract_asset_id_balance_insert(contract, asset_id, balance)
            .map_err(RuntimeError::from_io)?;

        self.inc_pc()
    }

    pub(crate) fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let bx = checked_add_word(b, ContractId::LEN as Word)?;
        let cd = checked_add_word(c, d)?;

        if d > MEM_MAX_ACCESS_SIZE || a > checked_sub_word(VM_MAX_RAM, d)? || bx > VM_MAX_RAM || cd > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c, d) = (a as usize, b as usize, c as usize, d as usize);
        let (bx, cd) = (bx as usize, cd as usize);

        // Safety: Memory bounds are checked by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[b..bx]) };

        if !self
            .transaction()
            .inputs()
            .iter()
            .any(|input| matches!(input, Input::Contract { contract_id, .. } if contract_id == contract))
        {
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let contract = self.contract(contract)?.into_owned();

        if contract.as_ref().len() < d {
            self.try_zeroize(a, d)?;
        } else {
            self.try_mem_write(a, &contract.as_ref()[c..cd])?;
        }

        self.inc_pc()
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let hash = self.storage.block_hash(b as u32).map_err(|e| e.into())?;

        self.try_mem_write(a as usize, hash.as_ref())?;

        self.inc_pc()
    }

    pub(crate) fn set_block_height(&mut self, ra: RegisterId) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        self.context
            .block_height()
            .map(|h| h as Word)
            .map(|h| self.registers[ra] = h)
            .ok_or(PanicReason::TransactionValidity)?;

        self.inc_pc()
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> Result<(), RuntimeError> {
        self.coinbase()
            .and_then(|data| self.try_mem_write(a as usize, data.as_ref()))?;

        self.inc_pc()
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let ax = checked_add_word(a, Bytes32::LEN as Word)?;
        let bx = checked_add_word(b, ContractId::LEN as Word)?;

        if ax > VM_MAX_RAM || bx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b) = (a as usize, b as usize);
        let bx = bx as usize;

        // Safety: Memory bounds are checked by the interpreter
        let contract_id = unsafe { ContractId::as_ref_unchecked(&self.memory[b..bx]) };

        let (_, root) = self
            .storage
            .storage_contract_root(contract_id)
            .transpose()
            .ok_or(PanicReason::ContractNotFound)?
            .map_err(RuntimeError::from_io)?
            .into_owned();

        self.try_mem_write(a, root.as_ref())?;

        self.inc_pc()
    }

    pub(crate) fn code_size(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        let bx = checked_add_word(b, ContractId::LEN as Word)?;

        if bx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (b, bx) = (b as usize, bx as usize);

        // Safety: Memory bounds are checked by the interpreter
        let contract_id = unsafe { ContractId::as_ref_unchecked(&self.memory[b..bx]) };

        self.registers[ra] = self.contract(contract_id)?.as_ref().as_ref().len() as Word;

        self.inc_pc()
    }

    pub(crate) fn state_read_word(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        let bx = checked_add_word(b, Bytes32::LEN as Word)?;

        if bx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (b, bx) = (b as usize, bx as usize);

        let contract = self.internal_contract()?;

        // Safety: Memory bounds are checked by the interpreter
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..bx]) };

        self.registers[ra] = self
            .storage
            .merkle_contract_state(contract, key)
            .map_err(RuntimeError::from_io)?
            .map(|state| unsafe { Bytes8::from_slice_unchecked(state.as_ref().as_ref()).into() })
            .map(Word::from_be_bytes)
            .unwrap_or(0);

        self.inc_pc()
    }

    pub(crate) fn state_read_qword(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let ax = checked_add_word(a, Bytes32::LEN as Word)?;
        let bx = checked_add_word(b, Bytes32::LEN as Word)?;

        if ax > VM_MAX_RAM || bx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b) = (a as usize, b as usize);
        let bx = bx as usize;

        let contract = self.internal_contract()?;

        // Safety: Memory bounds are checked by the interpreter
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..bx]) };

        let state = self
            .storage
            .merkle_contract_state(contract, key)
            .map_err(RuntimeError::from_io)?
            .map(|s| s.into_owned())
            .unwrap_or_default();

        self.try_mem_write(a, state.as_ref())?;

        self.inc_pc()
    }

    pub(crate) fn state_write_word(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let ax = checked_add_word(a, Bytes32::LEN as Word)?;

        if ax > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, ax) = (a as usize, ax as usize);
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[a..ax]) };

        let mut value = Bytes32::default();

        value[..WORD_SIZE].copy_from_slice(&b.to_be_bytes());

        self.storage
            .merkle_contract_state_insert(contract, key, &value)
            .map_err(RuntimeError::from_io)?;

        self.inc_pc()
    }

    pub(crate) fn state_write_qword(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let ax = checked_add_word(a, Bytes32::LEN as Word)?;
        let bx = checked_add_word(b, Bytes32::LEN as Word)?;

        if ax > VM_MAX_RAM || bx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b) = (a as usize, b as usize);
        let (ax, bx) = (ax as usize, bx as usize);
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[a..ax]) };
        let value = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..bx]) };

        self.storage
            .merkle_contract_state_insert(contract, key, value)
            .map_err(RuntimeError::from_io)?;

        self.inc_pc()
    }

    pub(crate) fn timestamp(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        self.block_height()
            .and_then(|c| (b <= c as Word).then_some(()).ok_or(PanicReason::TransactionValidity))?;

        let b = u32::try_from(b).map_err(|_| PanicReason::ArithmeticOverflow)?;

        self.registers[ra] = self.storage.timestamp(b).map_err(|e| e.into())?;

        self.inc_pc()
    }

    pub(crate) fn message_output(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let ax = checked_add_word(a, Address::LEN as Word)?;
        let bx = checked_add_word(ax, b)?;

        //TODO check on b vs MEM_MAX_ACCESS_SIZE is looser than msg length check
        if b > self.params.max_message_data_length || b > MEM_MAX_ACCESS_SIZE || bx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, ax, bx) = (a as usize, ax as usize, bx as usize);

        let idx = c;
        let amount = d;

        // Safety: checked len
        let recipient = unsafe { Address::from_slice_unchecked(&self.memory[a..ax]) };
        if recipient == Address::zeroed() {
            return Err(PanicReason::ZeroedMessageOutputRecipient.into());
        }

        let offset = self
            .transaction()
            .outputs_offset_at(c as usize)
            .and_then(|ofs| ofs.checked_add(self.tx_offset()))
            .ok_or(PanicReason::OutputNotFound)?;

        // halt with I/O error because tx should be serialized correctly into vm memory
        let output = Output::from_bytes(&self.memory[offset..])?;

        // amount isn't checked because we are allowed to send zero balances with a message
        if !matches!(output, Output::Message { recipient, .. } if recipient == Address::zeroed()) {
            return Err(PanicReason::NonZeroMessageOutputRecipient.into());
        }

        // validations passed, perform the mutations

        self.base_asset_balance_sub(amount)?;

        let fp = self.registers[REG_FP] as usize;
        let txid = self.tx_id();
        let data = ax;
        let data = &self.memory[data..bx];
        let data = data.to_vec();

        let fpx = checked_add_usize(fp, Address::LEN)?;

        // Safety: $fp is guaranteed to contain enough bytes
        let sender = unsafe { Address::from_slice_unchecked(&self.memory[fp..fpx]) };

        let message = Output::message(recipient, amount);
        let receipt = Receipt::message_out_from_tx_output(txid, idx, sender, recipient, amount, data);

        self.set_message_output(idx as usize, message)?;
        self.append_receipt(receipt);

        self.inc_pc()
    }
}
