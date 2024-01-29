use crate::{
    common::{
        path::{
            Instruction,
            Path,
        },
        Bytes32,
        Prefix,
        ProofSet,
    },
    sparse::{
        empty_sum,
        Node,
    },
};
use core::fmt::Debug;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Proof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
    /// The hash that is used as the initial value when accumulating hashes to
    /// reconstruct the root. An exclusion proof must provide an initial hash
    /// originating from proof generation. If it is not provided, the proof is
    /// invalid. Conversely, an inclusion proof should not provide an initial
    /// hash, as this value is determined by the key-value undergoing proving.
    pub initial_hash: Option<Bytes32>,
}

impl Proof {
    pub fn root(&self) -> &Bytes32 {
        &self.root
    }

    pub fn proof_set(&self) -> &ProofSet {
        &self.proof_set
    }

    pub fn is_inclusion(&self) -> bool {
        self.initial_hash.is_none()
    }

    pub fn is_exclusion(&self) -> bool {
        !self.is_inclusion()
    }

    pub fn verify<K: Into<Bytes32>, V: AsRef<[u8]>>(&self, key: K, value: &V) -> bool {
        let Proof {
            root,
            proof_set,
            initial_hash,
        } = self;
        let key: Bytes32 = key.into();
        let mut current;

        if value.as_ref() == empty_sum() {
            // Exclusion proof
            if self.is_inclusion() {
                return false;
            }
            current = initial_hash.expect("Expected initial hash")
        } else {
            // Inclusion proof
            let leaf = Node::create_leaf(&key, value);
            current = *leaf.hash();
        }

        for (i, side_hash) in proof_set.iter().enumerate() {
            let index = u32::try_from(proof_set.len() - 1 - i).expect("Index is valid");
            let prefix = Prefix::Node;
            current = match key.get_instruction(index).expect("Infallible") {
                Instruction::Left => Node::calculate_hash(&prefix, &current, side_hash),
                Instruction::Right => Node::calculate_hash(&prefix, side_hash, &current),
            };
        }

        current == *root
    }
}
