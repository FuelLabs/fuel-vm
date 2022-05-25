use crate::{Transaction, ValidationError};

#[cfg(feature = "std")]
use crate::StorageSlot;

use fuel_crypto::Hasher;
use fuel_types::{Bytes32, ContractId, Salt};

#[cfg(feature = "std")]
use fuel_merkle::{binary, common::StorageMap, sparse};

#[cfg(feature = "std")]
use fuel_types::Bytes8;

use alloc::vec::Vec;

#[cfg(feature = "std")]
use core::iter;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Deployable representation of a contract code.
pub struct Contract(Vec<u8>);

impl Contract {
    #[cfg(feature = "std")]
    /// Calculate the code root of the contract, using [`Self::root_from_code`].
    pub fn root(&self) -> Bytes32 {
        Self::root_from_code(self)
    }

    #[cfg(feature = "std")]
    /// Calculate the code root from a contract.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/identifiers.md#contract-id>
    pub fn root_from_code<B>(bytes: B) -> Bytes32
    where
        B: AsRef<[u8]>,
    {
        let mut storage = StorageMap::new();
        let mut tree = binary::MerkleTree::new(&mut storage);

        bytes
            .as_ref()
            .chunks(Bytes8::LEN)
            .map(|c| {
                if c.len() == Bytes8::LEN {
                    // Safety: checked len chunk
                    unsafe { Bytes8::from_slice_unchecked(c) }
                } else {
                    // Potential collision with non-padded input. Consider adding an extra leaf
                    // for padding?
                    let mut b = [0u8; 8];

                    let l = c.len();
                    (&mut b[..l]).copy_from_slice(c);

                    b.into()
                }
            })
            .try_for_each(|l| tree.push(l.as_ref()))
            .and_then(|_| tree.root())
            .expect("In-memory impl should be infallible")
            .into()
    }

    #[cfg(feature = "std")]
    /// Calculate the root of the initial storage slots for this contract
    pub fn initial_state_root<'a, I>(mut storage_slots: I) -> Bytes32
    where
        I: Iterator<Item = &'a StorageSlot>,
    {
        let mut storage = StorageMap::new();
        let mut tree = sparse::MerkleTree::new(&mut storage);

        storage_slots
            .try_for_each(|s| tree.update(s.key(), s.value().as_ref()))
            .expect("In-memory impl should be infallible");

        tree.root().into()
    }

    #[cfg(feature = "std")]
    /// The default state root value without any entries
    pub fn default_state_root() -> Bytes32 {
        Self::initial_state_root(iter::empty())
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

#[cfg(test)]
mod tests {
    use super::*;
    use fuel_types::{bytes::WORD_SIZE, Bytes64};
    use itertools::Itertools;
    use proptest::{prop_assert_eq, proptest};
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use rstest::rstest;

    macro_rules! set_snapshot_suffix {
        ($($expr:expr),*) => {{
            let mut settings = insta::Settings::clone_current();
            settings.set_snapshot_suffix(format!($($expr,)*));
            settings.bind_to_thread();
        }}
    }

    // safe-guard against breaking changes to the code root calculation for valid
    // sizes of bytecode (multiples of instruction size in bytes (half-word))
    #[rstest]
    fn code_root_snapshot(#[values(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100)] instructions: usize) {
        let mut rng = StdRng::seed_from_u64(100);
        let code_len = instructions * WORD_SIZE / 2;
        let mut code = alloc::vec![0u8; code_len];
        rng.fill_bytes(code.as_mut_slice());

        // compute root
        let root = Contract::root_from_code(code);

        // take root snapshot
        set_snapshot_suffix!("instructions-{}", instructions);
        insta::assert_debug_snapshot!(root);
    }

    // validate code_root is always equivalent to contract.root
    proptest! {
        #[test]
        fn contract_root_matches_code_root(instructions in 0usize..100) {
            let mut rng = StdRng::seed_from_u64(100);
            let code_len = instructions * WORD_SIZE / 2;
            let mut code = alloc::vec![0u8; code_len];
            rng.fill_bytes(code.as_mut_slice());
            let contract = Contract::from(code.clone());
            // compute root
            let code_root = Contract::root_from_code(code);
            let contract_root = contract.root();
            prop_assert_eq!(code_root, contract_root);
        }
    }

    #[rstest]
    fn state_root_snapshot(
        #[values(Vec::new(), vec![Bytes64::new([1u8; 64])])] state_slot_bytes: Vec<Bytes64>,
    ) {
        let slots: Vec<StorageSlot> = state_slot_bytes.iter().map(Into::into).collect_vec();
        let state_root = Contract::initial_state_root(&mut slots.iter());
        // take root snapshot
        set_snapshot_suffix!("state-root-{}", slots.len());
        insta::assert_debug_snapshot!(state_root);
    }

    #[test]
    fn default_state_root_snapshot() {
        let default_root = Contract::default_state_root();
        insta::assert_debug_snapshot!(default_root);
    }
}
