use crate::{
    common::{error::DeserializeError, Bytes32, Prefix, PrefixError},
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
pub type Primitive = (u32, u8, Bytes32, Bytes32);

trait PrimitiveView {
    fn height(&self) -> u32;
    fn prefix(&self) -> Result<Prefix, PrefixError>;
    fn bytes_lo(&self) -> &Bytes32;
    fn bytes_hi(&self) -> &Bytes32;
}

impl PrimitiveView for Primitive {
    fn height(&self) -> u32 {
        self.0
    }

    fn prefix(&self) -> Result<Prefix, PrefixError> {
        Prefix::try_from(self.1)
    }

    fn bytes_lo(&self) -> &Bytes32 {
        &self.2
    }

    fn bytes_hi(&self) -> &Bytes32 {
        &self.3
    }
}

impl From<&Node> for Primitive {
    fn from(node: &Node) -> Self {
        (
            node.height(),
            node.prefix() as u8,
            *node.bytes_lo(),
            *node.bytes_hi(),
        )
    }
}

impl TryFrom<Primitive> for Node {
    type Error = DeserializeError;

    fn try_from(primitive: Primitive) -> Result<Self, Self::Error> {
        let height = primitive.height();
        let prefix = primitive.prefix()?;
        let bytes_lo = *primitive.bytes_lo();
        let bytes_hi = *primitive.bytes_hi();
        let node = Self::new(height, prefix, bytes_lo, bytes_hi);
        Ok(node)
    }
}
