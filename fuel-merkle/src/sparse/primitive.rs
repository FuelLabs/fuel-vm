use crate::sparse::merkle_tree::MerkleTreeKey;
use crate::{
    common::{error::DeserializeError, Prefix, PrefixError},
    sparse::Node,
};

/// **Leaf buffer:**
///
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 04`  | Height (4 bytes)           |
/// | `04 - 05`  | Prefix (1 byte, `0x00`)    |
/// | `05 - 37`  | hash(Key) (32 bytes)       |
/// | `37 - 69`  | hash(Data) (32 bytes)      |
///
/// **Node buffer:**
///
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 04`  | Height (4 bytes)           |
/// | `04 - 05`  | Prefix (1 byte, `0x01`)    |
/// | `05 - 37`  | Left child key (32 bytes)  |
/// | `37 - 69`  | Right child key (32 bytes) |
///
pub type Primitive<Key> = (u32, u8, Key, Key);

trait PrimitiveView<Key> {
    fn height(&self) -> u32;
    fn prefix(&self) -> Result<Prefix, PrefixError>;
    fn bytes_lo(&self) -> &Key;
    fn bytes_hi(&self) -> &Key;
}

impl<Key> PrimitiveView<Key> for Primitive<Key> {
    fn height(&self) -> u32 {
        self.0
    }

    fn prefix(&self) -> Result<Prefix, PrefixError> {
        Prefix::try_from(self.1)
    }

    fn bytes_lo(&self) -> &Key {
        &self.2
    }

    fn bytes_hi(&self) -> &Key {
        &self.3
    }
}

impl<Key> From<&Node<Key>> for Primitive<Key>
where
    Key: MerkleTreeKey,
{
    fn from(node: &Node<Key>) -> Self {
        (node.height(), node.prefix() as u8, *node.bytes_lo(), *node.bytes_hi())
    }
}

impl<Key> TryFrom<Primitive<Key>> for Node<Key>
where
    Key: MerkleTreeKey,
{
    type Error = DeserializeError;

    fn try_from(primitive: Primitive<Key>) -> Result<Self, Self::Error> {
        let height = primitive.height();
        let prefix = primitive.prefix()?;
        let bytes_lo = *primitive.bytes_lo();
        let bytes_hi = *primitive.bytes_hi();
        let node = Self::new(height, prefix, bytes_lo, bytes_hi);
        Ok(node)
    }
}
