use crate::receipt::Receipt;

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
    MessageOut = 0x0A,
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
            Receipt::MessageOut { .. } => Self::MessageOut,
        }
    }
}
