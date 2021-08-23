use super::{ExecuteError, Interpreter};
use crate::consts::*;
use crate::data::{InterpreterStorage, MerkleStorage, Storage};

use fuel_asm::{RegisterId, Word};
use fuel_tx::{Address, Bytes32, Bytes8, Color, ContractId, Input, Salt};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn coinbase(&self) -> Result<Address, ExecuteError> {
        Ok(self.storage.coinbase()?)
    }

    pub(crate) fn burn(&mut self, a: Word) -> Result<(), ExecuteError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let color = unsafe { Color::as_ref_unchecked(&self.memory[c..cx]) };

        let balance = self.balance(contract, color)?;
        let balance = balance.checked_sub(a).ok_or(ExecuteError::NotEnoughBalance)?;

        <S as MerkleStorage<ContractId, Color, Word>>::insert(&mut self.storage, contract, color, &balance)?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn mint(&mut self, a: Word) -> Result<(), ExecuteError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let color = unsafe { Color::as_ref_unchecked(&self.memory[c..cx]) };

        let balance = self.balance(contract, color)?;
        let balance = balance.checked_add(a).ok_or(ExecuteError::ArithmeticOverflow)?;

        <S as MerkleStorage<ContractId, Color, Word>>::insert(&mut self.storage, contract, color, &balance)?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), ExecuteError> {
        if d > MEM_MAX_ACCESS_SIZE
            || a > VM_MAX_RAM - d
            || b > VM_MAX_RAM - ContractId::size_of() as Word
            || c > VM_MAX_RAM - d
        {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (a, b, c, d) = (a as usize, b as usize, c as usize, d as usize);

        let bx = b + ContractId::size_of();
        let cd = c + d;

        // Safety: Memory bounds are checked by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[b..bx]) };

        if !self
            .tx
            .inputs()
            .iter()
            .any(|input| matches!(input, Input::Contract { contract_id, .. } if contract_id == contract))
        {
            return Err(ExecuteError::ContractNotInTxInputs);
        }

        let contract = self.contract(contract)?.ok_or(ExecuteError::ContractNotFound)?;

        if contract.as_ref().len() < d {
            self.try_zeroize(a, d)?;
        } else {
            self.try_mem_write(a, &contract.as_ref()[c..cd])?;
        }

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> Result<(), ExecuteError> {
        let hash = self.storage.block_hash(b as u32)?;

        self.try_mem_write(a as usize, hash.as_ref())?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> Result<(), ExecuteError> {
        self.coinbase()
            .and_then(|data| self.try_mem_write(a as usize, data.as_ref()))?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> Result<(), ExecuteError> {
        if a > VM_MAX_RAM - Bytes32::size_of() as Word || b > VM_MAX_RAM - ContractId::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (a, b) = (a as usize, b as usize);

        // Safety: Memory bounds are checked by the interpreter
        let contract_id = unsafe { ContractId::as_ref_unchecked(&self.memory[b..b + ContractId::size_of()]) };

        let (_, root) = <S as Storage<ContractId, (Salt, Bytes32)>>::get(&self.storage, contract_id)
            .transpose()
            .ok_or(ExecuteError::ContractNotFound)??;

        self.try_mem_write(a, root.as_ref())?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn code_size(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        if b > VM_MAX_RAM - ContractId::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let b = b as usize;

        // Safety: Memory bounds are checked by the interpreter
        let contract_id = unsafe { ContractId::as_ref_unchecked(&self.memory[b..b + ContractId::size_of()]) };

        self.contract(contract_id)
            .transpose()
            .ok_or(ExecuteError::ContractNotFound)?
            .map(|contract| self.registers[ra] = contract.as_ref().len() as Word)?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn state_read_word(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        if b > VM_MAX_RAM - Bytes32::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let b = b as usize;

        let contract = self.internal_contract()?;

        // Safety: Memory bounds are checked by the interpreter
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..b + Bytes32::size_of()]) };

        self.registers[ra] = <S as MerkleStorage<ContractId, Bytes32, Bytes32>>::get(&self.storage, &contract, key)?
            .map(|state| unsafe { Bytes8::from_slice_unchecked(state.as_ref()).into() })
            .map(Word::from_be_bytes)
            .unwrap_or(0);

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn state_read_qword(&mut self, a: Word, b: Word) -> Result<(), ExecuteError> {
        if a > VM_MAX_RAM - Bytes32::size_of() as Word || b > VM_MAX_RAM - Bytes32::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (a, b) = (a as usize, b as usize);

        let contract = self.internal_contract()?;

        // Safety: Memory bounds are checked by the interpreter
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..b + Bytes32::size_of()]) };

        let state =
            <S as MerkleStorage<ContractId, Bytes32, Bytes32>>::get(&self.storage, &contract, key)?.unwrap_or_default();

        self.try_mem_write(a, state.as_ref())?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn state_write_word(&mut self, a: Word, b: Word) -> Result<(), ExecuteError> {
        if a > VM_MAX_RAM - Bytes32::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let a = a as usize;
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[a..a + Bytes32::size_of()]) };

        let mut value = Bytes32::default();

        (&mut value[..WORD_SIZE]).copy_from_slice(&b.to_be_bytes());

        <S as MerkleStorage<ContractId, Bytes32, Bytes32>>::insert(&mut self.storage, contract, key, &value)?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn state_write_qword(&mut self, a: Word, b: Word) -> Result<(), ExecuteError> {
        if a > VM_MAX_RAM - Bytes32::size_of() as Word || b > VM_MAX_RAM - Bytes32::size_of() as Word {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (a, b) = (a as usize, b as usize);
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };
        let key = unsafe { Bytes32::as_ref_unchecked(&self.memory[a..a + Bytes32::size_of()]) };
        let value = unsafe { Bytes32::as_ref_unchecked(&self.memory[b..b + Bytes32::size_of()]) };

        <S as MerkleStorage<ContractId, Bytes32, Bytes32>>::insert(&mut self.storage, &contract, key, value)?;

        self.inc_pc();

        Ok(())
    }
}
