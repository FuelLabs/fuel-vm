use crate::{
    common::{
        error::DeserializeError,
        Bytes,
        Prefix,
        PrefixError,
    },
    sparse::node_generic::Node,
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
pub type Primitive<const KeySize: usize> = (u32, u8, Bytes<KeySize>, Bytes<KeySize>);

trait PrimitiveView<const KeySize: usize> {
    fn height(&self) -> u32;
    fn prefix(&self) -> Result<Prefix, PrefixError>;
    fn bytes_lo(&self) -> &Bytes<KeySize>;
    fn bytes_hi(&self) -> &Bytes<KeySize>;
}

impl<const KeySize: usize> PrimitiveView<KeySize> for Primitive<KeySize> {
    fn height(&self) -> u32 {
        self.0
    }

    fn prefix(&self) -> Result<Prefix, PrefixError> {
        Prefix::try_from(self.1)
    }

    fn bytes_lo(&self) -> &Bytes<KeySize> {
        &self.2
    }

    fn bytes_hi(&self) -> &Bytes<KeySize> {
        &self.3
    }
}

impl<const KeySize: usize> From<&Node<KeySize>> for Primitive<KeySize> {
    fn from(node: &Node<KeySize>) -> Self {
        (
            node.height(),
            node.prefix() as u8,
            node.bytes_lo(),
            node.bytes_hi(),
        )
    }
}

impl<const KeySize: usize> TryFrom<Primitive<KeySize>> for Node<KeySize> {
    type Error = DeserializeError;

    fn try_from(primitive: Primitive<KeySize>) -> Result<Self, Self::Error> {
        let height = primitive.height();
        let prefix = primitive.prefix()?;
        let bytes_lo = *primitive.bytes_lo();
        let bytes_hi = *primitive.bytes_hi();
        let node = Self::new(height, prefix, bytes_lo, bytes_hi);
        Ok(node)
    }
}
