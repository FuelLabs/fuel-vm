use crate::common::{
    Bytes,
    Prefix,
    PrefixError,
};

/// **Leaf buffer (32 byte KEY_SIZE):**
///
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 04`  | Height (4 bytes)           |
/// | `04 - 05`  | Prefix (1 byte, `0x00`)    |
/// | `05 - 37`  | hash(Key) (32 bytes)       |
/// | `37 - 69`  | hash(Data) (32 bytes)      |
///
/// **Node buffer (32 byte KEY_SIZE):**
///
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 04`  | Height (4 bytes)           |
/// | `04 - 05`  | Prefix (1 byte, `0x01`)    |
/// | `05 - 37`  | Left child key (32 bytes)  |
/// | `37 - 69`  | Right child key (32 bytes) |
pub type Primitive<const KEY_SIZE: usize> = (u32, u8, Bytes<KEY_SIZE>, Bytes<KEY_SIZE>);

pub trait PrimitiveView<const KEY_SIZE: usize> {
    fn height(&self) -> u32;
    fn prefix(&self) -> Result<Prefix, PrefixError>;
    fn bytes_lo(&self) -> &Bytes<KEY_SIZE>;
    fn bytes_hi(&self) -> &Bytes<KEY_SIZE>;
}

impl<const KEY_SIZE: usize> PrimitiveView<KEY_SIZE> for Primitive<KEY_SIZE> {
    fn height(&self) -> u32 {
        self.0
    }

    fn prefix(&self) -> Result<Prefix, PrefixError> {
        Prefix::try_from(self.1)
    }

    fn bytes_lo(&self) -> &Bytes<KEY_SIZE> {
        &self.2
    }

    fn bytes_hi(&self) -> &Bytes<KEY_SIZE> {
        &self.3
    }
}
