use crate::common::{
    empty_sum_sha256,
    Bytes32,
};

use sha2::Sha256;

type Hash = Sha256;

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub const fn empty_sum() -> &'static Bytes32 {
    empty_sum_sha256()
}
