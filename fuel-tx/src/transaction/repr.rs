use crate::Transaction;

#[cfg(feature = "std")]
use fuel_types::Word;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
}

impl From<&Transaction> for TransactionRepr {
    fn from(tx: &Transaction) -> Self {
        match tx {
            Transaction::Script { .. } => Self::Script,
            Transaction::Create { .. } => Self::Create,
        }
    }
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
