use crate::common::{
    Bytes32,
    ProofSet,
};

pub enum Proof {
    InclusionProof(InclusionProof),
    ExclusionProof(ExclusionProof),
}

impl Proof {
    pub fn root(&self) -> &Bytes32 {
        match self {
            Proof::InclusionProof(InclusionProof { root, .. }) => root,
            Proof::ExclusionProof(ExclusionProof { root, .. }) => root,
        }
    }

    pub fn proof_set(&self) -> &ProofSet {
        match self {
            Proof::InclusionProof(InclusionProof { proof_set, .. }) => proof_set,
            Proof::ExclusionProof(ExclusionProof { proof_set, .. }) => proof_set,
        }
    }
}

pub struct InclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
}

pub struct ExclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
    pub leaf_key: Bytes32,
    pub hash: Bytes32,
}
