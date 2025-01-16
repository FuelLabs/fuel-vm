use jmt::proof::SparseMerkleProof;

use crate::common::Bytes32;

// Give crate access to fields for testing tampering with proofs
pub struct InclusionProof {
    pub(crate) proof: SparseMerkleProof<sha2::Sha256>,
    pub(crate) key: jmt::KeyHash,
    pub(crate) value: jmt::OwnedValue,
}

pub struct ExclusionProof {
    pub(crate) proof: SparseMerkleProof<sha2::Sha256>,
    pub(crate) key: jmt::KeyHash,
}

pub enum MerkleProof {
    Inclusion(InclusionProof),
    Exclusion(ExclusionProof),
}

impl MerkleProof {
    pub fn verify(&self, root_hash: Bytes32) -> bool {
        match self {
            MerkleProof::Inclusion(inclusion_proof) => {
                let root_hash = jmt::RootHash(root_hash);
                let key = inclusion_proof.key;
                let value = &inclusion_proof.value;
                let proof = &inclusion_proof.proof;

                proof.verify_existence(root_hash, key, value).is_ok()
            }
            MerkleProof::Exclusion(exclusion_proof) => {
                let root_hash = jmt::RootHash(root_hash);
                let key = exclusion_proof.key;
                let proof = &exclusion_proof.proof;

                proof.verify_nonexistence(root_hash, key).is_ok()
            }
        }
    }

    pub fn is_inclusion_proof(&self) -> bool {
        match self {
            MerkleProof::Inclusion(_) => true,
            MerkleProof::Exclusion(_) => false,
        }
    }

    pub fn is_exclusion_proof(&self) -> bool {
        match self {
            MerkleProof::Inclusion(_) => false,
            MerkleProof::Exclusion(_) => true,
        }
    }
}
