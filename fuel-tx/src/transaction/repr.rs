use ::core::result::Result;

use ::fuel_types::canonical::{
    Deserialize,
    Error,
    Input,
};

use crate::Transaction;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Serialize)]
#[repr(u64)]
pub enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
    Mint = 0x02,
    Upgrade = 0x03,
    Upload = 0x04,
    Blob = 0x05,
    Unknown = 0x16,
}

impl Deserialize for TransactionRepr {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        const SCRIPT: u64 = TransactionRepr::Script as u64;
        const CREATE: u64 = TransactionRepr::Create as u64;
        const MINT: u64 = TransactionRepr::Mint as u64;
        const UPGRADE: u64 = TransactionRepr::Upgrade as u64;
        const UPLOAD: u64 = TransactionRepr::Upload as u64;
        const BLOB: u64 = TransactionRepr::Blob as u64;

        match <u64 as Deserialize>::decode(buffer)? {
            SCRIPT => Result::Ok(TransactionRepr::Script),
            CREATE => Result::Ok(TransactionRepr::Create),
            MINT => Result::Ok(TransactionRepr::Mint),
            UPGRADE => Result::Ok(TransactionRepr::Upgrade),
            UPLOAD => Result::Ok(TransactionRepr::Upload),
            BLOB => Result::Ok(TransactionRepr::Blob),
            _ => Result::Ok(TransactionRepr::Unknown),
        }
    }

    fn decode_dynamic<I: Input + ?Sized>(
        &mut self,
        _buffer: &mut I,
    ) -> Result<(), Error> {
        Result::Ok(())
    }
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
            Transaction::Unknown => Self::Unknown,
        }
    }
}
