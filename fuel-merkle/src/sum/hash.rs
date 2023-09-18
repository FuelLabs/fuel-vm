use crate::common::{
    self,
    Bytes32,
    Prefix,
};

use digest::Digest;
use sha2::Sha256;

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub const fn empty_sum() -> &'static Bytes32 {
    common::empty_sum_sha256()
}

// Merkle tree hash of an n-element list D[n]
// MTH(D[n]) = Hash(0x01 || LHS fee || MTH(D[0:k]) || RHS fee || MTH(D[k:n])
pub fn node_sum(lhs_fee: u64, lhs_data: &[u8], rhs_fee: u64, rhs_data: &[u8]) -> Bytes32 {
    let mut hash = Sha256::new();
    hash.update(Prefix::Node);
    hash.update(lhs_fee.to_be_bytes());
    hash.update(lhs_data);
    hash.update(rhs_fee.to_be_bytes());
    hash.update(rhs_data);
    hash.finalize().try_into().unwrap()
}

// Merkle tree hash of a list with one entry
// MTH({d(0)}) = Hash(0x00 || fee || d(0))
pub fn leaf_sum(fee: u64, data: &[u8]) -> Bytes32 {
    let mut hash = Sha256::new();
    hash.update(Prefix::Leaf);
    hash.update(fee.to_be_bytes());
    hash.update(data);
    hash.finalize().try_into().unwrap()
}
