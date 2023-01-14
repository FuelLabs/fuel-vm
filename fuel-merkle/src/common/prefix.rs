const NODE: u8 = 0x01;
const LEAF: u8 = 0x00;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Prefix {
    Node = NODE,
    Leaf = LEAF,
}

impl Default for Prefix {
    fn default() -> Self {
        Prefix::Leaf
    }
}

impl From<Prefix> for u8 {
    fn from(prefix: Prefix) -> Self {
        match prefix {
            Prefix::Node => NODE,
            Prefix::Leaf => LEAF,
        }
    }
}

impl From<Prefix> for [u8; 1] {
    fn from(prefix: Prefix) -> Self {
        match prefix {
            Prefix::Node => [NODE],
            Prefix::Leaf => [LEAF],
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum PrefixError {
    #[cfg_attr(feature = "std", error("prefix {0} is not valid"))]
    InvalidPrefix(u8),
}

impl TryFrom<u8> for Prefix {
    type Error = PrefixError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            NODE => Ok(Prefix::Node),
            LEAF => Ok(Prefix::Leaf),
            _ => Err(PrefixError::InvalidPrefix(byte)),
        }
    }
}

impl AsRef<[u8]> for Prefix {
    fn as_ref(&self) -> &[u8] {
        match self {
            Prefix::Node => &[NODE],
            Prefix::Leaf => &[LEAF],
        }
    }
}

impl AsRef<[u8; 1]> for Prefix {
    fn as_ref(&self) -> &[u8; 1] {
        match self {
            Prefix::Node => &[NODE],
            Prefix::Leaf => &[LEAF],
        }
    }
}
