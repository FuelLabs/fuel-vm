pub(crate) use digest::Digest;

use lazy_static::lazy_static;
use sha2::Sha256;
use std::convert::TryInto;

use crate::common::Bytes32;

pub(crate) type Hash = Sha256;

lazy_static! {
    static ref EMPTY_SUM: Bytes32 = Hash::new().finalize().try_into().unwrap();
    static ref ZERO_SUM: Bytes32 = [0; 32];
}

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub fn empty_sum() -> &'static Bytes32 {
    &*EMPTY_SUM
}

pub fn zero_sum() -> &'static Bytes32 {
    &*ZERO_SUM
}

pub fn sum(data: &[u8]) -> Bytes32 {
    let mut hash = Hash::new();
    hash.update(&data);
    hash.finalize().try_into().unwrap()
}
