use crate::Transaction;

#[cfg(feature = "std")]
use fuel_types::Word;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
    Mint = 0x02,
}

impl From<&Transaction> for TransactionRepr {
    fn from(tx: &Transaction) -> Self {
        match tx {
            Transaction::Script { .. } => Self::Script,
            Transaction::Create { .. } => Self::Create,
            Transaction::Mint { .. } => Self::Mint,
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
            0x02 => Ok(Self::Mint),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}
