//! Definitions and implementations for each unique instruction type, one for each
//! unique `Opcode` variant.

use super::{
    widemath::{CompareArgs, MathArgs, MulArgs, DivArgs},
    CheckRegId, GMArgs, GTFArgs, Imm12, Imm18, Instruction, RegId,
};

// Here we re-export the generated instruction types and constructors, but extend them with
// `gm_args` and `gtf_args` short-hand constructors below to take their `GMArgs` and `GTFArgs`
// values respectively.
#[doc(inline)]
pub use super::_op::*;

impl GM {
    /// Construct a `GM` instruction from its arguments.
    pub fn from_args(ra: RegId, args: GMArgs) -> Self {
        Self::new(ra, Imm18::new(args as _))
    }
}

impl GTF {
    /// Construct a `GTF` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, args: GTFArgs) -> Self {
        Self::new(ra, rb, Imm12::new(args as _))
    }
}

/// Construct a `GM` instruction from its arguments.
pub fn gm_args<A: CheckRegId>(ra: A, args: GMArgs) -> Instruction {
    Instruction::GM(GM::from_args(ra.check(), args))
}

/// Construct a `GM` instruction from its arguments.
pub fn gtf_args<A: CheckRegId, B: CheckRegId>(ra: A, rb: B, args: GTFArgs) -> Instruction {
    Instruction::GTF(GTF::from_args(ra.check(), rb.check(), args))
}

impl WDCM {
    /// Construct a `WDCM` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: CompareArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WQCM {
    /// Construct a `WQCM` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: CompareArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WDOP {
    /// Construct a `WDOP` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: MathArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WQOP {
    /// Construct a `WQOP` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: MathArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WDML {
    /// Construct a `WDML` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: MulArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WQML {
    /// Construct a `WQML` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: MulArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WDDV {
    /// Construct a `WDDV` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: DivArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

impl WQDV {
    /// Construct a `WQDV` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: DivArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

/// Construct a `WDCM` instruction from its arguments.
pub fn wdcm_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: CompareArgs) -> Instruction {
    Instruction::WDCM(WDCM::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WQCM` instruction from its arguments.
pub fn wqcm_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: CompareArgs) -> Instruction {
    Instruction::WQCM(WQCM::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WDOP` instruction from its arguments.
pub fn wdop_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: MathArgs) -> Instruction {
    Instruction::WDOP(WDOP::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WQOP` instruction from its arguments.
pub fn wqop_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: MathArgs) -> Instruction {
    Instruction::WQOP(WQOP::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WDML` instruction from its arguments.
pub fn wdml_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: MulArgs) -> Instruction {
    Instruction::WDML(WDML::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WQML` instruction from its arguments.
pub fn wqml_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: MulArgs) -> Instruction {
    Instruction::WQML(WQML::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WDDV` instruction from its arguments.
pub fn wddv_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: DivArgs) -> Instruction {
    Instruction::WDDV(WDDV::from_args(ra.check(), rb.check(), rc.check(), args))
}

/// Construct a `WQDV` instruction from its arguments.
pub fn wqdv_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(ra: A, rb: B, rc: C, args: DivArgs) -> Instruction {
    Instruction::WQDV(WQDV::from_args(ra.check(), rb.check(), rc.check(), args))
}
