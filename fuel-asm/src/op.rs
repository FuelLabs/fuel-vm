//! Definitions and implementations for each unique instruction type, one for each
//! unique `Opcode` variant.
use super::{GMArgs, GTFArgs, Instruction, RegId, Imm12, Imm18};

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
pub fn gm_args(ra: u8, args: GMArgs) -> Instruction {
    Instruction::GM(GM::from_args(RegId::from(ra), args))
}

/// Construct a `GM` instruction from its arguments.
pub fn gtf_args(ra: u8, rb: u8, args: GTFArgs) -> Instruction {
    Instruction::GTF(GTF::from_args(RegId::from(ra), RegId::from(rb), args))
}
