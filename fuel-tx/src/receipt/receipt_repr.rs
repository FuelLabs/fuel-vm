use crate::receipt::Receipt;
use fuel_types::Word;

use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReceiptRepr {
    Call = 0x00,
    Return = 0x01,
    ReturnData = 0x02,
    Panic = 0x03,
    Revert = 0x04,
    Log = 0x05,
    LogData = 0x06,
    Transfer = 0x07,
    TransferOut = 0x08,
    ScriptResult = 0x09,
}

impl From<&Receipt> for ReceiptRepr {
    fn from(receipt: &Receipt) -> Self {
        match receipt {
            Receipt::Call { .. } => Self::Call,
            Receipt::Return { .. } => Self::Return,
            Receipt::ReturnData { .. } => Self::ReturnData,
            Receipt::Panic { .. } => Self::Panic,
            Receipt::Revert { .. } => Self::Revert,
            Receipt::Log { .. } => Self::Log,
            Receipt::LogData { .. } => Self::LogData,
            Receipt::Transfer { .. } => Self::Transfer,
            Receipt::TransferOut { .. } => Self::TransferOut,
            Receipt::ScriptResult { .. } => Self::ScriptResult,
        }
    }
}

impl TryFrom<Word> for ReceiptRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Call),
            0x01 => Ok(Self::Return),
            0x02 => Ok(Self::ReturnData),
            0x03 => Ok(Self::Panic),
            0x04 => Ok(Self::Revert),
            0x05 => Ok(Self::Log),
            0x06 => Ok(Self::LogData),
            0x07 => Ok(Self::Transfer),
            0x08 => Ok(Self::TransferOut),
            0x09 => Ok(Self::ScriptResult),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}
