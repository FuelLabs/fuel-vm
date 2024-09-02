use serde::{
    Deserialize,
    Serialize,
};

/// Untyped key pointing to a registry table entry.
/// The last key (all bits set) is reserved for the default value and cannot be written
/// to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RawKey([u8; Self::SIZE]);
impl RawKey {
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
    #[allow(clippy::cast_possible_truncation)]
    pub fn add_u32(self, rhs: u32) -> Self {
        let lhs = self.as_u32() as u64;
        let rhs = rhs as u64;
        // Safety: cannot overflow as both operands are limited to 32 bits
        let result = (lhs + rhs) % (Self::DEFAULT_VALUE.as_u32() as u64);
        // Safety: cannot truncate as we are already limited to 24 bits by modulo
        let v = result as u32;
        let v = v.to_be_bytes();
        Self([v[1], v[2], v[3]])
    }

    /// Wraps around just below max/default value.
    pub fn next(self) -> Self {
        self.add_u32(1)
    }

    /// Is `self` between `start` and `end`? i.e. in the half-open logical range
    /// `start`..`end`, so that wrap-around cases are handled correctly.
    ///
    /// Panics if max/default value is used.
    pub fn is_between(self, start: Self, end: Self) -> bool {
        assert!(
            self != Self::DEFAULT_VALUE,
            "Cannot use max/default value in is_between"
        );
        assert!(
            start != Self::DEFAULT_VALUE,
            "Cannot use max/default value in is_between"
        );
        assert!(
            end != Self::DEFAULT_VALUE,
            "Cannot use max/default value in is_between"
        );

        let low = start.as_u32();
        let high = end.as_u32();
        let v = self.as_u32();

        if high >= low {
            low <= v && v < high
        } else {
            v < high || v >= low
        }
    }
}
impl TryFrom<u32> for RawKey {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let v = value.to_be_bytes();
        if v[0] != 0 {
            return Err("RawKey must be less than 2^24");
        }

        let mut bytes = [0u8; 3];
        bytes.copy_from_slice(&v[1..]);
        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::RawKey;

    #[test]
    fn key_next() {
        assert_eq!(RawKey::ZERO.next(), RawKey([0, 0, 1]));
        assert_eq!(RawKey::ZERO.next().next(), RawKey([0, 0, 2]));
        assert_eq!(RawKey([0, 0, 255]).next(), RawKey([0, 1, 0]));
        assert_eq!(RawKey([0, 1, 255]).next(), RawKey([0, 2, 0]));
        assert_eq!(RawKey([0, 255, 255]).next(), RawKey([1, 0, 0]));
        assert_eq!(RawKey::MAX_WRITABLE.next(), RawKey::ZERO);
    }
}
