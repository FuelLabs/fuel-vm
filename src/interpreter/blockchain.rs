use super::{ContractData, ContractState, ExecuteError, Interpreter, MemoryRange};
use crate::consts::*;
use crate::data::{InterpreterStorage, KeyedMerkleStorage};

use fuel_asm::{RegisterId, Word};
use fuel_tx::{Address, Bytes32, ContractId, Input};

use std::convert::TryFrom;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockData {
    height: u32,
    hash: Bytes32,
}

impl BlockData {
    pub const fn new(height: u32, hash: Bytes32) -> Self {
        Self { height, hash }
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn hash(&self) -> &Bytes32 {
        &self.hash
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn block_data(&self, block_height: u32) -> Result<BlockData, ExecuteError> {
        Ok(self.storage.block_data(block_height)?)
    }

    pub(crate) fn coinbase(&self) -> Result<Address, ExecuteError> {
        Ok(self.storage.coinbase()?)
    }

    pub(crate) fn burn(&mut self, a: Word) -> Result<bool, ExecuteError> {
        self.internal_contract()
            .map(|contract| (contract, (*contract).into()))
            .and_then(|(contract, color)| self.balance_sub(contract, color, a))
            .map(|_| self.inc_pc())
    }

    pub(crate) fn mint(&mut self, a: Word) -> Result<bool, ExecuteError> {
        self.internal_contract()
            .map(|contract| (contract, (*contract).into()))
            .and_then(|(contract, color)| self.balance_add(contract, color, a))
            .map(|_| self.inc_pc())
    }

    // TODO add CCP tests
    pub(crate) fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> bool {
        let (ad, overflow) = a.overflowing_add(d);
        let (bx, of) = b.overflowing_add(ContractId::size_of() as Word);
        let overflow = overflow || of;
        let (cd, of) = c.overflowing_add(d);
        let overflow = overflow || of;

        let range = MemoryRange::new(a, d);
        if overflow
            || ad >= VM_MAX_RAM
            || bx >= VM_MAX_RAM
            || d > MEM_MAX_ACCESS_SIZE
            || !self.has_ownership_range(&range)
        {
            return false;
        }

        let contract =
            ContractId::try_from(&self.memory[b as usize..bx as usize]).expect("Memory bounds logically checked");

        if !self
            .tx
            .inputs()
            .iter()
            .any(|input| matches!(input, Input::Contract { contract_id, .. } if contract_id == &contract))
        {
            return false;
        }

        // TODO optmize
        let contract = match self.contract(&contract) {
            Ok(Some(c)) => c,
            _ => return false,
        };

        let memory = &mut self.memory[a as usize..ad as usize];
        if contract.as_ref().len() < cd as usize {
            memory.iter_mut().for_each(|m| *m = 0);
        } else {
            memory.copy_from_slice(&contract.as_ref()[..d as usize]);
        }

        true
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> Result<bool, ExecuteError> {
        self.block_data(b as u32)
            .and_then(|data| self.try_mem_write(a, data.hash().as_ref()))
            .map(|_| self.inc_pc())
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> Result<bool, ExecuteError> {
        self.coinbase()
            .and_then(|data| self.try_mem_write(a, data.as_ref()))
            .map(|_| self.inc_pc())
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> Result<bool, ExecuteError> {
        if a >= VM_MAX_RAM - 32 || b >= VM_MAX_RAM - 32 {
            return Err(ExecuteError::MemoryOverflow);
        }

        let contract_id = <[u8; ContractId::size_of()]>::try_from(&self.memory[b as usize..b as usize + 32])
            .expect("Checked memory bounds!")
            .into();

        <S as KeyedMerkleStorage<ContractId, ContractData, (), ContractState>>::metadata(&self.storage, &contract_id)
            .or(Err(ExecuteError::ContractNotFound))
            .and_then(|data| self.try_mem_write(a, data.root().as_ref()))
            .map(|_| self.inc_pc())
    }

    pub(crate) fn code_size(&mut self, ra: RegisterId, b: Word) -> Result<bool, ExecuteError> {
        if b >= VM_MAX_RAM - 32 {
            return Err(ExecuteError::MemoryOverflow);
        }

        let contract_id = <[u8; ContractId::size_of()]>::try_from(&self.memory[b as usize..b as usize + 32])
            .expect("Checked memory bounds!")
            .into();

        self.contract(&contract_id)
            .transpose()
            .ok_or(ExecuteError::ContractNotFound)?
            .and_then(|contract| Ok(self.registers[ra] = contract.as_ref().len() as Word))
            .map(|_| self.inc_pc())
    }
}
