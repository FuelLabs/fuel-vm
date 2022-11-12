use crate::common::{self, Bytes32, LEAF, NODE};

use digest::Digest;
use sha2::Sha256;

type Hash = Sha256;

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub const fn empty_sum() -> &'static Bytes32 {
    common::empty_sum_sha256()
}

// Merkle tree hash of an n-element list D[n]
// MTH(D[n]) = Hash(0x01 || MTH(D[0:k]) || MTH(D[k:n])
pub fn node_sum(lhs_data: &[u8], rhs_data: &[u8]) -> Bytes32 {
    let mut hash = Hash::new();

    hash.update([NODE]);
    hash.update(lhs_data);
    hash.update(rhs_data);

    hash.finalize().into()
}

// Merkle tree hash of a list with one entry
// MTH({d(0)}) = Hash(0x00 || d(0))
pub fn leaf_sum(data: &[u8]) -> Bytes32 {
    let mut hash = Hash::new();

    hash.update([LEAF]);
    hash.update(data);

    hash.finalize().into()
}
