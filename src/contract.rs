//! Chain contract definition

use crate::crypto;
use crate::error::InterpreterError;

use fuel_crypto::Hasher;
use fuel_tx::{StorageSlot, Transaction, ValidationError};
use fuel_types::{Bytes32, ContractId, Salt};

use std::cmp;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Deployable representation of a contract code.
pub struct Contract(Vec<u8>);

impl Contract {
    /// Calculate the code root from a contract.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/identifiers.md#contract-id>
    pub fn root(&self) -> Bytes32 {
        let root = self.0.chunks(8).map(|c| {
            let mut bytes = [0u8; 8];

            let l = cmp::min(c.len(), 8);
            (&mut bytes[..l]).copy_from_slice(c);

            bytes
        });

        crypto::ephemeral_merkle_root(root)
    }

    /// Calculate the root of the initial storage slots for this contract
    /// TODO: Use a sparse merkle tree once the implementation is available
    pub fn initial_state_root(storage_slots: &[StorageSlot]) -> Bytes32 {
        let leaves = storage_slots.iter().map(|slot| {
            let mut buf = [0u8; 64];
            buf[..32].copy_from_slice(slot.key().as_slice());
            buf[32..].copy_from_slice(slot.value().as_slice());
            buf
        });
        crypto::ephemeral_merkle_root(leaves)
    }

    /// The default state root value without any entries
    pub fn default_state_root() -> Bytes32 {
        Self::initial_state_root(&[])
    }

    /// Calculate and return the contract id, provided a salt, code root and state root.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/identifiers.md#contract-id>
    pub fn id(&self, salt: &Salt, root: &Bytes32, state_root: &Bytes32) -> ContractId {
        let mut hasher = Hasher::default();

        hasher.input(ContractId::SEED);
        hasher.input(salt);
        hasher.input(root);
        hasher.input(state_root);

        ContractId::from(*hasher.digest())
    }
}

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
    type Error = InterpreterError;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
        match tx {
            Transaction::Create {
                bytecode_witness_index,
                witnesses,
                ..
            } => witnesses
                .get(*bytecode_witness_index as usize)
                .map(|c| c.as_ref().into())
                .ok_or_else(|| ValidationError::TransactionCreateBytecodeWitnessIndex.into()),

            _ => Err(ValidationError::TransactionScriptOutputContractCreated { index: 0 }.into()),
        }
    }
}
