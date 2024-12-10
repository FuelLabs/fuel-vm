use fuel_asm::{
    PanicReason,
    Word,
};

use crate::Transaction;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    fuel_types::canonical::Serialize,
    fuel_types::canonical::Deserialize,
)]
#[repr(u64)]
pub enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
    Mint = 0x02,
    Upgrade = 0x03,
    Upload = 0x04,
    Blob = 0x05,
}

impl From<&Transaction> for TransactionRepr {
    fn from(tx: &Transaction) -> Self {
        match tx {
            Transaction::Script { .. } => Self::Script,
            Transaction::Create { .. } => Self::Create,
            Transaction::Mint { .. } => Self::Mint,
            Transaction::Upgrade { .. } => Self::Upgrade,
            Transaction::Upload { .. } => Self::Upload,
            Transaction::Blob { .. } => Self::Blob,
        }
    }
}

impl TryFrom<Word> for TransactionRepr {
    type Error = PanicReason;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Script),
            0x01 => Ok(Self::Create),
            0x02 => Ok(Self::Mint),
            0x03 => Ok(Self::Upgrade),
            0x04 => Ok(Self::Upload),
            0x05 => Ok(Self::Blob),
            _ => Err(PanicReason::InvalidTransactionType),
        }
    }
}
