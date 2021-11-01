use super::Interpreter;
use crate::consts::*;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_tx::{consts::CONTRACT_MAX_SIZE, Input};
use fuel_types::{bytes, Address, Bytes32, Bytes8, Color, ContractId, RegisterId, Word};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn coinbase(&self) -> Result<Address, InterpreterError> {
        self.storage.coinbase().map_err(|e| e.into())
    }

    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(&mut self, a: Word, b: Word, c: Word) -> Result<(), InterpreterError> {
        let ssp = self.registers[REG_SSP];
        let sp = self.registers[REG_SP];
        let hp = self.registers[REG_HP];
        let fp = self.registers[REG_FP];

        let id_addr = a as usize; // address of contract ID
        let start_in_contract = b as usize; // start offset
        let length_to_copy_unpadded = c; // length to copy

        // Validate arguments
        if ssp + length_to_copy_unpadded > hp
            || (id_addr + ContractId::LEN) as u64 > VM_MAX_RAM
            || length_to_copy_unpadded > CONTRACT_MAX_SIZE.min(MEM_MAX_ACCESS_SIZE)
        {
            return Err(InterpreterError::MemoryOverflow);
        }

        if ssp != sp {
            return Err(InterpreterError::ExpectedEmptyStack);
        }

        // Fetch the contract id
        let contract_id = unsafe {
            // Safety: this area of memory will not be modified until this ref is dropped
            let bytes = core::slice::from_raw_parts(self.memory.as_ptr().add(id_addr), ContractId::LEN);
            // Safety: Memory bounds are checked by the interpreter
            ContractId::as_ref_unchecked(bytes)
        };

        // Check that the contract exists
        if !self.tx.input_contracts().any(|id| id == contract_id) {
            return Err(InterpreterError::ContractNotInTxInputs);
        };

        // Calculate the word aligned padded len based on $rC
        let contract_len = {
            let cow_contract = self.contract(contract_id)?;
            let contract = cow_contract.as_ref().as_ref();
            contract.len()
        };
        let padded_len = bytes::padded_len_usize(length_to_copy_unpadded as usize);
        let padding_len = padded_len - (length_to_copy_unpadded as usize);
        let end_in_contract = (start_in_contract + padded_len).min(contract_len);
        let copy_len = end_in_contract - start_in_contract;

        // Increment the frame code size by len defined in memory
        let offset_in_frame = ContractId::LEN + Color::LEN + WORD_SIZE * VM_REGISTER_COUNT;
        let start = (fp as usize) + offset_in_frame;
        // Safety: bounds enforced by the interpreter
        let old = Word::from_be_bytes(unsafe {
            bytes::from_slice_unchecked::<WORD_SIZE>(&self.memory[start..start + WORD_SIZE])
        });
        let new = ((old as usize) + padded_len) as Word;
        self.memory[start..start + WORD_SIZE].copy_from_slice(&new.to_be_bytes());

        // Push the contract code to the stack
        let cow_contract = self.contract(contract_id)?;
        let contract = cow_contract.as_ref().as_ref();
        // Safety: Pushing to stack doesn't modify the contract
        unsafe {
            let code = core::slice::from_raw_parts(contract.as_ptr().add(start_in_contract), copy_len);
            self.push_stack(&code)?;
        }
        self.push_stack(&[0; core::mem::size_of::<Word>()][..padding_len])?;
        self.registers[REG_SP] = ssp + (padded_len as u64);

        self.inc_pc()
    }

    pub(crate) fn burn(&mut self, a: Word) -> Result<(), InterpreterError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let color = unsafe { Color::as_ref_unchecked(&self.memory[c..cx]) };

        let balance = self.balance(contract, color)?;
        let balance = balance.checked_sub(a).ok_or(InterpreterError::NotEnoughBalance)?;

        self.storage
            .merkle_contract_color_balance_insert(contract, color, balance)?;

        self.inc_pc()
    }

    pub(crate) fn mint(&mut self, a: Word) -> Result<(), InterpreterError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let color = unsafe { Color::as_ref_unchecked(&self.memory[c..cx]) };

        let balance = self.balance(contract, color)?;
        let balance = balance.checked_add(a).ok_or(InterpreterError::ArithmeticOverflow)?;

        self.storage
            .merkle_contract_color_balance_insert(contract, color, balance)?;

        self.inc_pc()
    }

    pub(crate) fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), InterpreterError> {
        if d > MEM_MAX_ACCESS_SIZE
            || a > VM_MAX_RAM - d
            || b > VM_MAX_RAM - ContractId::LEN as Word
            || c > VM_MAX_RAM - d
        {
            return Err(InterpreterError::MemoryOverflow);
        }

        let (a, b, c, d) = (a as usize, b as usize, c as usize, d as usize);

        let bx = b + ContractId::LEN;
        let cd = c + d;

        // Safety: Memory bounds are checked by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[b..bx]) };

        if !self
            .tx
            .inputs()
            .iter()
            .any(|input| matches!(input, Input::Contract { contract_id, .. } if contract_id == contract))
        {
            return Err(InterpreterError::ContractNotInTxInputs);
        }

        let contract = self.contract(contract)?.into_owned();

        if contract.as_ref().len() < d {
            self.try_zeroize(a, d)?;
        } else {
            self.try_mem_write(a, &contract.as_ref()[c..cd])?;
        }

        self.inc_pc()
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> Result<(), InterpreterError> {
        let hash = self.storage.block_hash(b as u32).map_err(|e| e.into())?;

        self.try_mem_write(a as usize, hash.as_ref())?;

        self.inc_pc()
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> Result<(), InterpreterError> {
        self.coinbase()
            .and_then(|data| self.try_mem_write(a as usize, data.as_ref()))?;

        self.inc_pc()
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> Result<(), InterpreterError> {
        if a > VM_MAX_RAM - Bytes32::LEN as Word || b > VM_MAX_RAM - ContractId::LEN as Word {
            return Err(InterpreterError::MemoryOverflow);
        }

        let (a, b) = (a as usize, b as usize);

        // Safety: Memory bounds are checked by the interpreter
        let contract_id = unsafe { ContractId::as_ref_unchecked(&self.memory[b..b + ContractId::LEN]) };

        let (_, root) = self
            .storage
            .storage_contract_root(contract_id)
            .transpose()
            .ok_or(InterpreterError::ContractNotFound)??
            .into_owned();

        self.try_mem_write(a, root.as_ref())?;

        self.inc_pc()
    }

    pub(crate) fn code_size(&mut self, ra: RegisterId, b: Word) -> Result<(), InterpreterError> {
        if b > VM_MAX_RAM - ContractId::LEN as Word {
            return Err(InterpreterError::MemoryOverflow);
        }

        let b = b as usize;

        // Safety: Memory bounds are checked by the interpreter
        let contract_id = unsafe { ContractId::as_ref_unchecked(&self.memory[b..b + ContractId::LEN]) };

        self.registers[ra] = self.contract(contract_id)?.as_ref().as_ref().len() as Word;

        self.inc_pc()
    }

    pub(crate) fn state_read_word(&mut self, ra: RegisterId, b: Word) -> Result<(), InterpreterError> {
        if b > VM_MAX_RAM - Bytes32::LEN as Word {
            return Err(InterpreterError::MemoryOverflow);
        }

        let b = b as usize;

        let contract = self.internal_contract()?;

        // Safety: Memory bounds are checked by the interpreter
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..b + Bytes32::LEN]) };

        self.registers[ra] = self
            .storage
            .merkle_contract_state(contract, key)?
            .map(|state| unsafe { Bytes8::from_slice_unchecked(state.as_ref().as_ref()).into() })
            .map(Word::from_be_bytes)
            .unwrap_or(0);

        self.inc_pc()
    }

    pub(crate) fn state_read_qword(&mut self, a: Word, b: Word) -> Result<(), InterpreterError> {
        if a > VM_MAX_RAM - Bytes32::LEN as Word || b > VM_MAX_RAM - Bytes32::LEN as Word {
            return Err(InterpreterError::MemoryOverflow);
        }

        let (a, b) = (a as usize, b as usize);

        let contract = self.internal_contract()?;

        // Safety: Memory bounds are checked by the interpreter
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..b + Bytes32::LEN]) };

        let state = self
            .storage
            .merkle_contract_state(contract, key)?
            .map(|s| s.into_owned())
            .unwrap_or_default();

        self.try_mem_write(a, state.as_ref())?;

        self.inc_pc()
    }

    pub(crate) fn state_write_word(&mut self, a: Word, b: Word) -> Result<(), InterpreterError> {
        if a > VM_MAX_RAM - Bytes32::LEN as Word {
            return Err(InterpreterError::MemoryOverflow);
        }

        let a = a as usize;
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[a..a + Bytes32::LEN]) };

        let mut value = Bytes32::default();

        (&mut value[..WORD_SIZE]).copy_from_slice(&b.to_be_bytes());

        self.storage.merkle_contract_state_insert(contract, key, &value)?;

        self.inc_pc()
    }

    pub(crate) fn state_write_qword(&mut self, a: Word, b: Word) -> Result<(), InterpreterError> {
        if a > VM_MAX_RAM - Bytes32::LEN as Word || b > VM_MAX_RAM - Bytes32::LEN as Word {
            return Err(InterpreterError::MemoryOverflow);
        }

        let (a, b) = (a as usize, b as usize);
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[a..a + Bytes32::LEN]) };
        let value = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..b + Bytes32::LEN]) };

        self.storage.merkle_contract_state_insert(contract, key, value)?;

        self.inc_pc()
    }
}
