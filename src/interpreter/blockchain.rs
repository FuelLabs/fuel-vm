use super::Interpreter;
use crate::consts::*;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_tx::Input;
use fuel_types::{Address, Bytes32, Bytes8, Color, ContractId, RegisterId, Word};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn coinbase(&self) -> Result<Address, InterpreterError> {
        self.storage.coinbase().map_err(|e| e.into())
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
        Self::is_register_writable(ra)?;

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
