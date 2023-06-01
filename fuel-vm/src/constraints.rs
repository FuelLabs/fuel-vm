//! Types to help constrain inputs to functions to only what is used.

use fuel_asm::RegId;
use fuel_types::ContractId;

use crate::prelude::Interpreter;

pub mod reg_key;

/// Location of an instructing collected during runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionLocation {
    /// Context, i.e. current contract. None if running a script.
    pub context: Option<ContractId>,
    /// Offset from the IS register
    pub offset: u64,
}

impl InstructionLocation {
    /// New location from context and offset
    pub const fn new(context: Option<ContractId>, offset: u64) -> Self {
        Self { context, offset }
    }

    /// Context, i.e. current contract
    pub const fn context(&self) -> Option<ContractId> {
        self.context
    }

    /// Offset from the IS register
    pub const fn offset(&self) -> u64 {
        self.offset
    }
}

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) fn current_location(&self) -> InstructionLocation {
        InstructionLocation {
            context: self.contract_id(),
            offset: self.registers()[RegId::PC] - self.registers()[RegId::IS],
        }
    }
}
