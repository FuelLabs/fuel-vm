use crate::{Instruction, PanicReason, RawInstruction, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
/// Describe a panic reason with the instruction that generated it
pub struct InstructionResult {
    reason: PanicReason,
    instruction: Instruction,
}

impl InstructionResult {
    /// Represents an error described by a reason and an instruction.
    pub const fn error(reason: PanicReason, instruction: Instruction) -> Self {
        Self { reason, instruction }
    }

    /// Underlying panic reason
    pub const fn reason(&self) -> &PanicReason {
        &self.reason
    }

    /// Underlying instruction
    pub const fn instruction(&self) -> &Instruction {
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
const INSTR_OFFSET: Word = REASON_OFFSET - (core::mem::size_of::<RawInstruction>() * 8) as Word;

impl From<InstructionResult> for Word {
    fn from(r: InstructionResult) -> Word {
        let reason = Word::from(r.reason as u8);
        let instruction = Word::from(u32::from(r.instruction));
        (reason << REASON_OFFSET) | (instruction << INSTR_OFFSET)
    }
}

// TODO: Should be `TryFrom`
impl From<Word> for InstructionResult {
    fn from(val: Word) -> Self {
        // NOTE: Safe to cast as we've shifted the 8 MSB.
        let reason_u8 = (val >> REASON_OFFSET) as u8;
        // NOTE: Cast to truncate in order to remove the `reason` bits.
        let inst_u32 = (val >> INSTR_OFFSET) as u32;
        let reason = PanicReason::from(reason_u8);
        let instruction = Instruction::try_from(inst_u32).expect("invalid instruction");
        Self { reason, instruction }
    }
}

// TODO: `PanicReason` shouldn't list reserved variants, and this should be `TryFrom` with no
// `unsafe`.
impl From<u8> for PanicReason {
    fn from(b: u8) -> Self {
        // Currently, the language doesn't support customized type coercion
        //
        // Safety: all possible values of `b` are either allocated or reserved
        unsafe { core::mem::transmute::<u8, Self>(b) }
    }
}
