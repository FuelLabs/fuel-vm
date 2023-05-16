use crate::panic_reason::InvalidPanicReason;
use crate::{Instruction, PanicReason, RawInstruction, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
/// Describe a panic reason with the instruction that generated it
pub struct PanicInstruction {
    reason: PanicReason,
    instruction: RawInstruction,
}

impl PanicInstruction {
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
}

const WORD_SIZE: usize = core::mem::size_of::<Word>();
const REASON_OFFSET: Word = (WORD_SIZE * 8 - 8) as Word;
const INSTR_OFFSET: Word = REASON_OFFSET - (Instruction::SIZE * 8) as Word;

impl From<PanicInstruction> for Word {
    fn from(r: PanicInstruction) -> Word {
        let reason = Word::from(r.reason as u8);
        let instruction = Word::from(r.instruction);
        (reason << REASON_OFFSET) | (instruction << INSTR_OFFSET)
    }
}

impl TryFrom<Word> for PanicInstruction {
    type Error = InvalidPanicReason;

    fn try_from(val: Word) -> Result<Self, Self::Error> {
        // Safe to cast as we've shifted the 8 MSB.
        let reason_u8 = (val >> REASON_OFFSET) as u8;
        // Cast to truncate in order to remove the `reason` bits.
        let instruction = (val >> INSTR_OFFSET) as u32;
        let reason = PanicReason::try_from(reason_u8)?;
        Ok(Self { reason, instruction })
    }
}
