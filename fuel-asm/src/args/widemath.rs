//! Immediate value arguments for wide-math instructions

use crate::Imm06;

/// Comparison mode used by WDCM and WQCM instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::FromRepr)]
#[repr(u8)]
pub enum CompareMode {
    /// Equality (`==`)
    EQ = 0,
    /// Inequality (`!=`)
    NE = 1,
    /// Less than (`<`)
    LT = 2,
    /// Greater than (`>`)
    GT = 3,
    /// Less than or equals (`>=`)
    LTE = 4,
    /// Greater than or equals (`>=`)
    GTE = 5,
}

/// Arguments for WDCM and WQCM instructions.
#[derive(Debug, Clone, Copy)]
pub struct CompareArgs {
    /// Comparison mode
    pub mode: CompareMode,
    /// Load RHS from register if true, otherwise zero-extend register value
    pub indirect_rhs: bool,
}

impl CompareArgs {
    /// Convert to immediate value.
    pub fn to_imm(self) -> Imm06 {
        let mut bits = self.mode as u8;
        bits |= (self.indirect_rhs as u8) << 5;
        Imm06(bits)
    }

    /// Construct from `Imm06`. Returns `None` if the value has reserved flags set.
    pub fn from_imm(bits: Imm06) -> Option<Self> {
        let indirect_rhs = ((bits.0 >> 5) & 1) == 1;
        let reserved = (bits.0 >> 3) & 0b11;
        if reserved != 0 {
            return None;
        }
        let mode = CompareMode::from_repr(bits.0 & 0b111)?;
        Some(Self { mode, indirect_rhs })
    }
}
/// The operation performed by WDOP and WQOP instructions, determined as
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::FromRepr)]
#[repr(u8)]
pub enum MathOp {
    /// Add
    ADD = 0,
    /// Subtract
    SUB = 1,
    /// Invert bits (discards rhs)
    NOT = 2,
    /// Bitwise or
    OR = 3,
    /// Bitwise exclusive or
    XOR = 4,
    /// Bitwise and
    AND = 5,
    /// Shift left
    SHL = 6,
    /// Shift right
    SHR = 7,
}

/// Additional arguments for WDOP and WQOP instructions.
#[derive(Debug, Clone, Copy)]
pub struct MathArgs {
    /// The operation to perform
    pub op: MathOp,
    /// Load RHS from register if true, otherwise zero-extend register value
    pub indirect_rhs: bool,
}

impl MathArgs {
    /// Convert to immediate value.
    pub fn to_imm(self) -> Imm06 {
        let mut bits = self.op as u8;
        bits |= (self.indirect_rhs as u8) << 5;
        Imm06(bits)
    }

    /// Construct from `Imm06`. Returns `None` if the value has reserved flags set.
    pub fn from_imm(bits: Imm06) -> Option<Self> {
        let indirect_rhs = ((bits.0 >> 5) & 1) == 1;
        let op = MathOp::from_repr(bits.0 & 0b11111)?;
        Some(Self { op, indirect_rhs })
    }
}

/// Additional arguments for WDML and WQML instructions.
#[derive(Debug, Clone, Copy)]
pub struct MulArgs {
    /// Load LHSS from register if true, otherwise zero-extend register value
    pub indirect_lhs: bool,
    /// Load RHS from register if true, otherwise zero-extend register value
    pub indirect_rhs: bool,
}

impl MulArgs {
    /// Convert to immediate value.
    pub fn to_imm(self) -> Imm06 {
        let mut bits = 0u8;
        bits |= (self.indirect_lhs as u8) << 4;
        bits |= (self.indirect_rhs as u8) << 5;
        Imm06(bits)
    }

    /// Construct from `Imm06`. Returns `None` if the value has reserved flags set.
    pub fn from_imm(bits: Imm06) -> Option<Self> {
        let indirect_lhs = ((bits.0 >> 4) & 1) == 1;
        let indirect_rhs = ((bits.0 >> 5) & 1) == 1;
        if (bits.0 & 0b1111) != 0 {
            return None;
        }
        Some(Self {
            indirect_lhs,
            indirect_rhs,
        })
    }
}

/// Additional arguments for WMDV and WDDV instructions.
#[derive(Debug, Clone, Copy)]
pub struct DivArgs {
    /// Load RHS from register if true, otherwise zero-extend register value
    pub indirect_rhs: bool,
}

impl DivArgs {
    /// Convert to immediate value.
    pub fn to_imm(self) -> Imm06 {
        let mut bits = 0u8;
        bits |= (self.indirect_rhs as u8) << 5;
        Imm06(bits)
    }

    /// Construct from `Imm06`. Returns `None` if the value has reserved flags set.
    pub fn from_imm(bits: Imm06) -> Option<Self> {
        let indirect_rhs = ((bits.0 >> 5) & 1) == 1;
        if (bits.0 & 0b11111) != 0 {
            return None;
        }
        Some(Self { indirect_rhs })
    }
}
