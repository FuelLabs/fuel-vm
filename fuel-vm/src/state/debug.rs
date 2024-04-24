use fuel_asm::Instruction;
use fuel_types::{
    ContractId,
    Word,
};

use crate::consts::VM_MAX_RAM;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Breakpoint description that binds a tuple `(contract, $pc)` to a debugger
/// implementation.
///
/// Breakpoints should be context-sensitive; hence, should target contract Ids.
///
/// For script/predicate verification, the contract id should be zero.
pub struct Breakpoint {
    contract: ContractId,
    pc: Word,
}

impl Breakpoint {
    pub(crate) const fn raw(contract: ContractId, pc: Word) -> Self {
        Self { contract, pc }
    }

    /// Create a new contract breakpoint
    ///
    /// The `$pc` is provided in op count and internally is multiplied by the op
    /// size. Also, the op count is always relative to `$is` so it should
    /// consider only the bytecode of the contract.
    ///
    /// Panics if the `pc` cannot ever fit into the VM memory.
    pub const fn new(contract: ContractId, pc: Word) -> Self {
        let pc = pc.saturating_mul(Instruction::SIZE as Word);
        assert!(pc <= VM_MAX_RAM, "Breakpoint cannot fit into vm memory");
        Self::raw(contract, pc)
    }

    /// Create a new script breakpoint
    ///
    /// The `$pc` is provided in op count and internally is multiplied by the op
    /// size
    pub fn script(pc: Word) -> Self {
        let contract = Default::default();

        Self::new(contract, pc)
    }

    /// Contract that will trigger the breakpoint.
    pub const fn contract(&self) -> &ContractId {
        &self.contract
    }

    /// Program counter that will trigger the breakpoint.
    pub const fn pc(&self) -> Word {
        self.pc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// State evaluation of the interpreter that will describe if a program should
/// break or continue.
pub enum DebugEval {
    /// This evaluation should break the program in the location described in
    /// `Breakpoint`.
    Breakpoint(Breakpoint),
    /// This evaluation should not break the program.
    Continue,
}

impl Default for DebugEval {
    fn default() -> Self {
        Self::Continue
    }
}

impl From<Breakpoint> for DebugEval {
    fn from(b: Breakpoint) -> Self {
        Self::Breakpoint(b)
    }
}

impl DebugEval {
    /// Flag whether the program execution should break.
    pub const fn should_continue(&self) -> bool {
        matches!(self, Self::Continue)
    }

    /// Return a breakpoint description if the current evaluation should break;
    /// return `None` otherwise.
    pub const fn breakpoint(&self) -> Option<&Breakpoint> {
        match self {
            Self::Breakpoint(b) => Some(b),
            _ => None,
        }
    }
}
