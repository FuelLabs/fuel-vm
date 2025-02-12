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
