use super::{ExecuteError, Interpreter};
use crate::consts::*;
use crate::crypto;
use crate::data::InterpreterStorage;

use fuel_asm::Word;
use fuel_tx::crypto as tx_crypto;
use fuel_tx::{Color, ContractId, Transaction, ValidationError};

use std::convert::TryFrom;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractColor {
    contract: ContractId,
    color: Color,
}

impl ContractColor {
    pub const fn new(contract: ContractId, color: Color) -> Self {
        Self { contract, color }
    }

    pub const fn contract(&self) -> &ContractId {
        &self.contract
    }

    pub const fn color(&self) -> &Color {
        &self.color
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Contract(Vec<u8>);

impl From<Vec<u8>> for Contract {
    fn from(c: Vec<u8>) -> Self {
        Self(c)
    }
}

impl From<&[u8]> for Contract {
    fn from(c: &[u8]) -> Self {
        Self(c.into())
    }
}

impl From<&mut [u8]> for Contract {
    fn from(c: &mut [u8]) -> Self {
        Self(c.into())
    }
}

impl From<Contract> for Vec<u8> {
    fn from(c: Contract) -> Vec<u8> {
        c.0
    }
}

impl AsRef<[u8]> for Contract {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for Contract {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl TryFrom<&Transaction> for Contract {
    type Error = ValidationError;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
        match tx {
            Transaction::Create {
                bytecode_witness_index,
                witnesses,
                ..
            } => witnesses
                .get(*bytecode_witness_index as usize)
                .map(|c| c.as_ref().into())
                .ok_or(ValidationError::TransactionCreateBytecodeWitnessIndex),

            _ => Err(ValidationError::TransactionScriptOutputContractCreated { index: 0 }),
        }
    }
}

impl Contract {
    pub fn address(&self, salt: &[u8]) -> ContractId {
        let mut input = VM_CONTRACT_ID_BASE.to_vec();

        input.extend_from_slice(salt);
        input.extend_from_slice(crypto::merkle_root(self.0.as_slice()).as_ref());

        (*tx_crypto::hash(input.as_slice())).into()
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn contract(&self, address: &ContractId) -> Result<Option<Contract>, ExecuteError> {
        Ok(self.storage.get(address)?)
    }

    pub fn check_contract_exists(&self, address: &ContractId) -> Result<bool, ExecuteError> {
        Ok(self.storage.contains_key(address)?)
    }

    pub fn set_balance(&mut self, key: ContractColor, balance: Word) -> Result<(), ExecuteError> {
        self.storage.insert(key, balance)?;

        Ok(())
    }

    pub fn balance(&self, key: &ContractColor) -> Result<Word, ExecuteError> {
        Ok(self.storage.get(key)?.unwrap_or(0))
    }

    pub fn balance_add(&mut self, key: ContractColor, value: Word) -> Result<Word, ExecuteError> {
        let balance = self.balance(&key)?;
        let balance = balance.checked_add(value).ok_or(ExecuteError::NotEnoughBalance)?;

        self.set_balance(key, balance)?;

        Ok(balance)
    }

    pub fn balance_sub(&mut self, key: ContractColor, value: Word) -> Result<Word, ExecuteError> {
        let balance = self.balance(&key)?;
        let balance = balance.checked_sub(value).ok_or(ExecuteError::NotEnoughBalance)?;

        self.set_balance(key, balance)?;

        Ok(balance)
    }
}
