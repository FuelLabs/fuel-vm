use crate::{Transaction, ValidationError};

#[cfg(feature = "std")]
use crate::StorageSlot;

use fuel_crypto::Hasher;
use fuel_types::{Bytes32, ContractId, Salt};

#[cfg(feature = "std")]
use fuel_merkle::{binary::MerkleTree, common::StorageMap};

#[cfg(feature = "std")]
use fuel_types::Bytes64;

use alloc::vec::Vec;

#[cfg(feature = "std")]
use core::cmp;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Deployable representation of a contract code.
pub struct Contract(Vec<u8>);

impl Contract {
    #[cfg(feature = "std")]
    /// Calculate a binary merkle root with in-memory storage
    fn ephemeral_merkle_root<L, I>(mut leaves: I) -> Bytes32
    where
        L: AsRef<[u8]>,
        I: Iterator<Item = L>,
    {
        let mut storage = StorageMap::new();
        let mut tree = MerkleTree::new(&mut storage);

        // TODO fuel-merkle should have infallible in-memory struct
        leaves
            .try_for_each(|l| tree.push(l.as_ref()))
            .and_then(|_| tree.root())
            .expect("In-memory impl should be infallible")
            .into()
    }

    #[cfg(feature = "std")]
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

        Self::ephemeral_merkle_root(root)
    }

    #[cfg(feature = "std")]
    /// Calculate the root of the initial storage slots for this contract
    /// TODO: Use a sparse merkle tree once the implementation is available
    pub fn initial_state_root(storage_slots: &[StorageSlot]) -> Bytes32 {
        let leaves = storage_slots.iter().map(Bytes64::from);

        Self::ephemeral_merkle_root(leaves)
    }

    #[cfg(feature = "std")]
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
