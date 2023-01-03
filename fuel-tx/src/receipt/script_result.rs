use fuel_types::Word;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScriptExecutionResult {
    Success,
    Revert,
    Panic,
    // Generic failure case since any u64 is valid here
    GenericFailure(u64),
}

impl From<ScriptExecutionResult> for Word {
    fn from(result: ScriptExecutionResult) -> Self {
        match result {
            ScriptExecutionResult::Success => 0x00,
            ScriptExecutionResult::Revert => 0x01,
            ScriptExecutionResult::Panic => 0x02,
            ScriptExecutionResult::GenericFailure(value) => value,
        }
    }
}

impl From<Word> for ScriptExecutionResult {
    fn from(value: u64) -> Self {
        match value {
            0x00 => Self::Success,
            0x01 => Self::Revert,
            0x02 => Self::Panic,
            value => Self::GenericFailure(value),
        }
    }
}
