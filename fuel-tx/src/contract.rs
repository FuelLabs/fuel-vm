use crate::{CheckError, StorageSlot, Transaction};

use derivative::Derivative;
use fuel_crypto::Hasher;
use fuel_merkle::binary::in_memory::MerkleTree as BinaryMerkleTree;
use fuel_merkle::sparse::in_memory::MerkleTree as SparseMerkleTree;
use fuel_types::{fmt_truncated_hex, Bytes32, ContractId, Salt};

use alloc::vec::Vec;
use core::iter;

/// The target size of Merkle tree leaves in bytes. Contract code will will be divided into chunks
/// of this size and pushed to the Merkle tree.
///
/// See https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/id/contract.md#contract-id
const LEAF_SIZE: usize = 16 * 1024;
/// In the event that contract code cannot be divided evenly by the `LEAF_SIZE`, the remainder must
/// be padded to the nearest multiple of 8 bytes. Padding is achieved by repeating the
/// `PADDING_BYTE`.
const PADDING_BYTE: u8 = 0u8;

/// See https://stackoverflow.com/a/9194117
fn next_multiple<const N: usize>(x: usize) -> usize {
    ((x + N - 1) / N) * N
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Deployable representation of a contract code.
pub struct Contract(#[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))] Vec<u8>);

impl Contract {
    /// Calculate the code root of the contract, using [`Self::root_from_code`].
    pub fn root(&self) -> Bytes32 {
        Self::root_from_code(self)
    }

    /// Calculate the code root from a contract.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/id/contract.md#contract-id>
    pub fn root_from_code<B>(bytes: B) -> Bytes32
    where
        B: AsRef<[u8]>,
    {
        let mut tree = BinaryMerkleTree::new();
        let mut bytes = bytes.as_ref().to_vec();

        // If the bytecode is not a multiple of 16 KiB, the final leaf should be
        // zero-padded rounding up to the nearest multiple of 8 bytes.
        let len = bytes.len();
        let padding_size = next_multiple::<8>(len) - len;
        let padding = iter::repeat(PADDING_BYTE).take(padding_size);
        for byte in padding {
            bytes.push(byte)
        }
        bytes.chunks(LEAF_SIZE).for_each(|leaf| tree.push(leaf));

        tree.root().into()
    }

    /// Calculate the root of the initial storage slots for this contract
    pub fn initial_state_root<'a, I>(storage_slots: I) -> Bytes32
    where
        I: Iterator<Item = &'a StorageSlot>,
    {
        let mut tree = SparseMerkleTree::new();

        storage_slots.for_each(|s| tree.update(s.key(), s.value().as_ref()));

        tree.root().into()
    }

    /// The default state root value without any entries
    pub fn default_state_root() -> Bytes32 {
        Self::initial_state_root(iter::empty())
    }

    /// Calculate and return the contract id, provided a salt, code root and state root.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/id/contract.md#contract-id>
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
    type Error = CheckError;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
        match tx {
            Transaction::Create(create) => TryFrom::try_from(create),
            _ => Err(CheckError::TransactionScriptOutputContractCreated { index: 0 }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuel_types::{bytes::WORD_SIZE, Bytes64};
    use itertools::Itertools;
    use quickcheck_macros::quickcheck;
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use rstest::rstest;

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
        insta::with_settings!(
            {snapshot_suffix => format!("instructions-{instructions}")},
            {
                insta::assert_debug_snapshot!(root);
            }
        );
    }

    // validate code_root is always equivalent to contract.root
    #[quickcheck]
    fn contract_root_matches_code_root(instructions: u8) -> bool {
        let mut rng = StdRng::seed_from_u64(100);
        let code_len = instructions as usize * WORD_SIZE / 2;
        let mut code = alloc::vec![0u8; code_len];
        rng.fill_bytes(code.as_mut_slice());
        let contract = Contract::from(code.clone());
        // compute root
        let code_root = Contract::root_from_code(code);
        let contract_root = contract.root();
        code_root == contract_root
    }

    #[rstest]
    fn state_root_snapshot(#[values(Vec::new(), vec![Bytes64::new([1u8; 64])])] state_slot_bytes: Vec<Bytes64>) {
        let slots: Vec<StorageSlot> = state_slot_bytes.iter().map(Into::into).collect_vec();
        let state_root = Contract::initial_state_root(&mut slots.iter());
        // take root snapshot
        insta::with_settings!(
            {snapshot_suffix => format!("state-root-{}", slots.len())},
            {
                insta::assert_debug_snapshot!(state_root);
            }
        );
    }

    #[test]
    fn default_state_root_snapshot() {
        let default_root = Contract::default_state_root();
        insta::assert_debug_snapshot!(default_root);
    }

    #[test]
    fn multi_leaf_state_root_snapshot() {
        let mut rng = StdRng::seed_from_u64(0xF00D);
        // 5 full leaves and a partial 6th leaf with 4 bytes of data
        let code_len = 5 * LEAF_SIZE + 4;
        let mut code = alloc::vec![0u8; code_len];
        rng.fill_bytes(code.as_mut_slice());

        // compute root
        let root = Contract::root_from_code(code);

        // take root snapshot
        insta::with_settings!(
            {snapshot_suffix => "multi-leaf-state-root"},
            {
                insta::assert_debug_snapshot!(root);
            }
        );
    }

    #[rstest]
    #[case(1)]
    #[case(8)]
    #[case(500)]
    #[case(1000)]
    #[case(1024)]
    #[case(1025)]
    fn partial_leaf_state_root(#[case] partial_leaf_size: usize) {
        let mut rng = StdRng::seed_from_u64(0xF00D);
        let code_len = partial_leaf_size;
        let mut code = alloc::vec![0u8; code_len];
        rng.fill_bytes(code.as_mut_slice());

        // Compute root
        let root = Contract::root_from_code(code.clone());

        // Compute expected root
        let expected_root = {
            let mut tree = BinaryMerkleTree::new();

            // Push partial leaf with manual padding.
            // We start by generating an n-byte array, where n is the code
            // length rounded to the nearest multiple of 8, and each byte is the
            // PADDING_BYTE by default. The leaf is generated by copying the
            // remaining data bytes into the start of this array.
            let sz = next_multiple::<8>(partial_leaf_size);
            let mut padded_leaf = vec![PADDING_BYTE; sz];
            padded_leaf[0..code_len].clone_from_slice(&code);
            tree.push(&padded_leaf);
            tree.root().into()
        };

        assert_eq!(root, expected_root);
    }

    #[rstest]
    #[case(1)]
    #[case(8)]
    #[case(500)]
    #[case(1000)]
    #[case(1024)]
    #[case(1025)]
    fn multi_leaf_state_root(#[case] partial_leaf_size: usize) {
        let mut rng = StdRng::seed_from_u64(0xF00D);
        // 3 full leaves and a partial 4th leaf
        let code_len = 3 * LEAF_SIZE + partial_leaf_size;
        let mut code = alloc::vec![0u8; code_len];
        rng.fill_bytes(code.as_mut_slice());

        // Compute root
        let root = Contract::root_from_code(code.clone());

        // Compute expected root
        let expected_root = {
            let mut tree = BinaryMerkleTree::new();

            let leaves = code.chunks(LEAF_SIZE).into_iter().collect::<Vec<_>>();
            tree.push(leaves[0]);
            tree.push(leaves[1]);
            tree.push(leaves[2]);

            // Push partial leaf with manual padding.
            // We start by generating an n-byte array, where n is the code
            // length rounded to the nearest multiple of 8, and each byte is the
            // PADDING_BYTE by default. The leaf is generated by copying the
            // remaining data bytes into the start of this array.
            let sz = next_multiple::<8>(partial_leaf_size);
            let mut padded_leaf = vec![PADDING_BYTE; sz];
            padded_leaf[0..partial_leaf_size].clone_from_slice(&leaves[3]);
            tree.push(&padded_leaf);
            tree.root().into()
        };

        assert_eq!(root, expected_root);
    }
}
