use crate::{Instruction, PanicReason, RawInstruction, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
/// Describe a panic reason with the instruction that generated it
pub struct InstructionResult {
    reason: PanicReason,
    instruction: RawInstruction,
}

impl InstructionResult {
    /// Represents an error described by a reason and an instruction.
    pub const fn error(reason: PanicReason, instruction: RawInstruction) -> Self {
        Self { reason, instruction }
    }

    /// Underlying panic reason
    pub const fn reason(&self) -> &PanicReason {
        &self.reason
    }

    /// Underlying instruction
    pub const fn instruction(&self) -> &RawInstruction {
        &self.instruction
    }

    /// This result represents success?
    pub const fn is_success(&self) -> bool {
        matches!(self.reason, PanicReason::Success)
    }

    /// This result represents error?
    pub const fn is_error(&self) -> bool {
        !self.is_success()
    }
}

const WORD_SIZE: usize = core::mem::size_of::<Word>();
const REASON_OFFSET: Word = (WORD_SIZE * 8 - 8) as Word;
const INSTR_OFFSET: Word = REASON_OFFSET - (Instruction::SIZE * 8) as Word;

impl From<InstructionResult> for Word {
    fn from(r: InstructionResult) -> Word {
        let reason = Word::from(r.reason as u8);
        let instruction = Word::from(r.instruction);
        (reason << REASON_OFFSET) | (instruction << INSTR_OFFSET)
    }
}

impl From<Word> for InstructionResult {
    fn from(val: Word) -> Self {
        // Safe to cast as we've shifted the 8 MSB.
        let reason_u8 = (val >> REASON_OFFSET) as u8;
        // Cast to truncate in order to remove the `reason` bits.
        let instruction = (val >> INSTR_OFFSET) as u32;
        let reason = PanicReason::from(reason_u8);
        Self { reason, instruction }
    }
}
