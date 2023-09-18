use num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};

use crate::Transaction;

#[derive(Debug, Clone, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u64)]
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
