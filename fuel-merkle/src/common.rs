mod msb;
mod node;
mod path_iterator;
mod position;
mod position_path;
mod prefix;
mod storage_map;
mod subtree;

pub(crate) mod error;
pub(crate) mod path;

pub use node::{Node, ParentNode};
pub use path_iterator::AsPathIterator;
pub use position::Position;
pub use storage_map::StorageMap;
pub use subtree::Subtree;

pub(crate) use msb::{Bit, Msb};
pub(crate) use node::{ChildError, ChildResult};
pub(crate) use position_path::PositionPath;
pub(crate) use prefix::{Prefix, PrefixError};

pub type Bytes1 = [u8; 1];
pub type Bytes2 = [u8; 2];
pub type Bytes4 = [u8; 4];
pub type Bytes8 = [u8; 8];
pub type Bytes16 = [u8; 16];
pub type Bytes32 = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct Wrapped<T> {
    value: T,
}

impl AsRef<Bytes32> for Wrapped<Bytes32> {
    fn as_ref(&self) -> &Bytes32 {
        &self.value
    }
}

impl AsRef<Bytes32> for Wrapped<&Bytes32> {
    fn as_ref(&self) -> &Bytes32 {
        self.value
    }
}

impl From<Bytes32> for Wrapped<Bytes32> {
    fn from(value: Bytes32) -> Self {
        Wrapped { value }
    }
}

impl<'a> From<&'a Bytes32> for Wrapped<Bytes32> {
    fn from(value: &Bytes32) -> Self {
        Wrapped { value: *value }
    }
}

// impl<'a> From<&'a Bytes32> for &Wrapped<Bytes32> {
//     fn from(value: &Bytes32) -> Self {
//         Wrapped { value: *value }
//     }
// }

use alloc::vec::Vec;

pub type ProofSet = Vec<Bytes32>;

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub const fn empty_sum_sha256() -> &'static Bytes32 {
    const EMPTY_SUM: Bytes32 = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24, 0x27, 0xae,
        0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
    ];

    &EMPTY_SUM
}

#[test]
fn empty_sum_sha256_is_empty_hash() {
    use digest::Digest;
    use sha2::Sha256;

    let sum = empty_sum_sha256();
    let empty = Bytes32::from(Sha256::new().finalize());

    assert_eq!(&empty, sum);
}
