use std::mem::size_of;

mod msb;
mod node;
mod path_iterator;
mod position;
mod storage_map;
mod subtree;

pub use msb::Msb;
pub use node::{Node, ParentNode};
pub use path_iterator::AsPathIterator;
pub use position::Position;
pub use storage_map::{StorageError, StorageMap};
pub use subtree::Subtree;

pub const NODE: u8 = 0x01;
pub const LEAF: u8 = 0x00;

pub type Bytes1 = [u8; 1];
pub type Bytes2 = [u8; 2];
pub type Bytes4 = [u8; 4];
pub type Bytes8 = [u8; 8];
pub type Bytes16 = [u8; 16];
pub type Bytes32 = [u8; 32];

/// For a leaf:
/// `00 - 01`: Prefix (1 byte, 0x00),
/// `01 - 33`: hash(Key) (32 bytes),
/// `33 - 65`: hash(Data) (32 bytes)
///
/// For a node:
/// `00 - 01`: Prefix (1 byte, 0x01),
/// `01 - 32`: Left child key (32 bytes),
/// `33 - 65`: Right child key (32 bytes)
///
const BUFFER_SIZE: usize = size_of::<Bytes1>() + size_of::<Bytes32>() + size_of::<Bytes32>();
pub type Buffer = [u8; BUFFER_SIZE];
