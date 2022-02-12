#[cfg(feature = "std")]
use fuel_types::Word;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
}

#[cfg(feature = "std")]
impl TryFrom<Word> for TransactionRepr {
    type Error = std::io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        use std::io;

        match b {
            0x00 => Ok(Self::Script),
            0x01 => Ok(Self::Create),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}
