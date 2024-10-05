use serde::{
    Deserialize,
    Serialize,
};

/// Untyped key pointing to a registry table entry.
/// The last key (all bits set) is reserved for the default value and cannot be written
/// to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegistryKey([u8; Self::SIZE]);
impl RegistryKey {
    /// Key mapping to default value for the table type.
    pub const DEFAULT_VALUE: Self = Self([u8::MAX; Self::SIZE]);
    /// Maximum writable key.
    pub const MAX_WRITABLE: Self = Self([u8::MAX, u8::MAX, u8::MAX - 1]);
    /// Size of the key, in bytes.
    pub const SIZE: usize = 3;
    /// Zero key.
    pub const ZERO: Self = Self([0; Self::SIZE]);

    /// Convert to u32, big-endian.
    pub fn as_u32(self) -> u32 {
        u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]])
    }

    /// Wraps around just below max/default value.
    /// Panics for max/default value.
    pub fn next(self) -> Self {
        if self == Self::DEFAULT_VALUE {
            panic!("Max/default value has no next key");
        }
        let next_raw = self.as_u32() + 1u32;
        if next_raw == Self::DEFAULT_VALUE.as_u32() {
            Self::ZERO
        } else {
            Self::try_from(next_raw)
                .expect("The procedure above always produces a valid key")
        }
    }
}

impl TryFrom<u32> for RegistryKey {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let v = value.to_be_bytes();
        if v[0] != 0 {
            return Err("RegistryKey must be less than 2^24");
        }

        let mut bytes = [0u8; 3];
        bytes.copy_from_slice(&v[1..]);
        Ok(Self(bytes))
    }
}

impl TryFrom<&[u8]> for RegistryKey {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::SIZE {
            return Err("RegistryKey must be 3 bytes long");
        }

        let mut bytes = [0u8; 3];
        bytes.copy_from_slice(value);
        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for RegistryKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::RegistryKey;

    #[test]
    fn key_next() {
        assert_eq!(RegistryKey::ZERO.next(), RegistryKey([0, 0, 1]));
        assert_eq!(RegistryKey::ZERO.next().next(), RegistryKey([0, 0, 2]));
        assert_eq!(RegistryKey([0, 0, 255]).next(), RegistryKey([0, 1, 0]));
        assert_eq!(RegistryKey([0, 1, 255]).next(), RegistryKey([0, 2, 0]));
        assert_eq!(RegistryKey([0, 255, 255]).next(), RegistryKey([1, 0, 0]));
        assert_eq!(RegistryKey::MAX_WRITABLE.next(), RegistryKey::ZERO);
    }
}
