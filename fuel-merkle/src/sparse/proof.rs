use crate::common::{
    Bytes32,
    ProofSet,
};
use alloc::{
    format,
    string::String,
    vec::Vec,
};
use core::{
    fmt,
    fmt::{
        Debug,
        Formatter,
    },
};

#[derive(Debug, Clone)]
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

#[derive(Clone)]
pub struct InclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
}

impl Debug for InclusionProof {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let proof_set = self
            .proof_set
            .iter()
            .map(hex::encode)
            .collect::<Vec<String>>();
        let proof_set = format!("[{}]", proof_set.join(", "));
        f.debug_struct("InclusionProof")
            .field("Root", &hex::encode(self.root))
            .field("Proof Set", &proof_set)
            .finish()
    }
}

#[derive(Clone)]
pub struct ExclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
    pub path: Bytes32,
    pub hash: Bytes32,
}

impl Debug for ExclusionProof {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let proof_set = self
            .proof_set
            .iter()
            .map(hex::encode)
            .collect::<Vec<String>>();
        let proof_set = format!("[{}]", proof_set.join(", "));
        f.debug_struct("ExclusionProof")
            .field("Root", &hex::encode(self.root))
            .field("Proof Set", &proof_set)
            .field("Path", &hex::encode(self.path))
            .field("Hash", &hex::encode(self.hash))
            .finish()
    }
}
