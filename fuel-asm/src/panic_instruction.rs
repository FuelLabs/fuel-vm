use core::fmt;

use crate::{
    Instruction,
    PanicReason,
    RawInstruction,
    Word,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
/// Describe a panic reason with the instruction that generated it
pub struct PanicInstruction {
    #[canonical(skip)]
    reason: PanicReason,
    instruction: RawInstruction,
}

impl PanicInstruction {
    /// Represents an error described by a reason and an instruction.
    pub const fn error(reason: PanicReason, instruction: RawInstruction) -> Self {
        Self {
            reason,
            instruction,
        }
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

/// Helper struct to debug-format a `RawInstruction` in `PanicInstruction::fmt`.
struct InstructionDbg(RawInstruction);
impl fmt::Debug for InstructionDbg {
    /// Formats like this: `MOVI { dst: 32, val: 32 } (bytes: 72 80 00 20)`}`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match Instruction::try_from(self.0) {
            Ok(instr) => write!(f, "{:?}", instr)?,
            Err(_) => write!(f, "Unknown")?,
        };
        write!(f, " (bytes: ")?;
        for (i, byte) in self.0.to_be_bytes().iter().enumerate() {
            if i != 0 {
                write!(f, " ")?;
            }
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

impl fmt::Debug for PanicInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PanicInstruction")
            .field("reason", &self.reason)
            .field("instruction", &InstructionDbg(self.instruction))
            .finish()
    }
}

#[cfg(feature = "typescript")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl PanicInstruction {
    /// Represents an error described by a reason and an instruction.
    #[wasm_bindgen(constructor)]
    pub fn error_typescript(reason: PanicReason, instruction: RawInstruction) -> Self {
        Self::error(reason, instruction)
    }

    /// Underlying panic reason
    #[wasm_bindgen(js_name = reason)]
    pub fn reason_typescript(&self) -> PanicReason {
        *self.reason()
    }

    /// Underlying instruction
    #[wasm_bindgen(js_name = instruction)]
    pub fn instruction_typescript(&self) -> RawInstruction {
        *self.instruction()
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

impl From<Word> for PanicInstruction {
    #[allow(clippy::cast_possible_truncation)]
    fn from(val: Word) -> Self {
        // Safe to cast as we've shifted the 8 MSB.
        let reason_u8 = (val >> REASON_OFFSET) as u8;
        // Cast to truncate in order to remove the `reason` bits.
        let instruction = (val >> INSTR_OFFSET) as u32;
        let reason = PanicReason::from(reason_u8);
        Self {
            reason,
            instruction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op;
    use fuel_types::canonical::Serialize;

    #[test]
    fn canonical_serialization_ignores_panic_reason() {
        let revert_panic_instruction =
            PanicInstruction::error(PanicReason::Revert, op::noop().into());
        let out_of_gas_panic_instruction =
            PanicInstruction::error(PanicReason::OutOfGas, op::noop().into());
        assert_eq!(
            revert_panic_instruction.to_bytes(),
            out_of_gas_panic_instruction.to_bytes()
        );
    }
}
