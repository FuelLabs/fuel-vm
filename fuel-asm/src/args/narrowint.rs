//! Immediate value arguments for subword math instruction NIOP

use crate::Imm06;

/// The operation performed by the NIOP instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::FromRepr)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen(js_name = NarrowMathOp))]
#[repr(u8)]
#[must_use]
pub enum MathOp {
    /// Add
    ADD = 0,
    /// Subtract
    SUB = 1,
    /// Multiply
    MUL = 2,
    /// Exponentiate
    EXP = 3,
    /// Bit shift left
    SLL = 4,
    /// XNOR
    XNOR = 5,
}

/// Operation width
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::FromRepr)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[repr(u8)]
#[must_use]
pub enum OpWidth {
    /// 8-bit
    U8 = 0,
    /// 16-bit
    U16 = 1,
    /// 32-bit
    U32 = 2,
}

/// Immediate value arguments for the NIOP instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen(js_name = NarrowMathArgs))]
#[must_use]
pub struct MathArgs {
    /// The operation to perform
    pub op: MathOp,
    /// Width of the operation
    pub width: OpWidth,
}

impl MathArgs {
    /// Convert to immediate value.
    pub fn to_imm(self) -> Imm06 {
        let mut bits = self.op as u8;
        bits |= (self.width as u8) << 4;
        Imm06(bits)
    }

    /// Construct from `Imm06`. Returns `None` if the value has reserved flags set.
    pub fn from_imm(bits: Imm06) -> Option<Self> {
        let op = MathOp::from_repr(bits.0 & 0b_1111)?;
        let width = OpWidth::from_repr((bits.0 >> 4) & 0b11)?;
        Some(Self { op, width })
    }
}

#[cfg(feature = "typescript")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl MathArgs {
    /// Create a new `MathArgs` instance from operation and width.
    #[wasm_bindgen(constructor)]
    pub fn new(op: MathOp, width: OpWidth) -> Self {
        Self { op, width }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    fn encode_decode_mathop(
        #[values(MathOp::ADD, MathOp::MUL, MathOp::EXP, MathOp::SLL, MathOp::XNOR)]
        op: MathOp,
        #[values(OpWidth::U8, OpWidth::U16, OpWidth::U32)] width: OpWidth,
    ) {
        let orig = MathArgs { op, width };
        let decoded = MathArgs::from_imm(orig.to_imm()).expect("decode error");
        assert_eq!(orig, decoded);
    }

    #[test]
    fn decode_encode_mathop() {
        for imm in 0..Imm06::MAX.0 {
            let bits = Imm06::from(imm);
            if let Some(decoded) = MathArgs::from_imm(bits) {
                assert_eq!(decoded.to_imm().0, imm);
            }
        }
    }
}
