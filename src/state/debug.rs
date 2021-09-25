use fuel_asm::Opcode;
use fuel_data::{ContractId, Word};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
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
    pub const fn new(contract: ContractId, pc: Word) -> Self {
        let pc = pc * (Opcode::BYTES_SIZE as Word);

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

    pub const fn contract(&self) -> &ContractId {
        &self.contract
    }

    pub const fn pc(&self) -> Word {
        self.pc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum DebugEval {
    Breakpoint(Breakpoint),
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
    pub const fn should_continue(&self) -> bool {
        match self {
            Self::Continue => true,
            _ => false,
        }
    }

    pub const fn breakpoint(&self) -> Option<&Breakpoint> {
        match self {
            Self::Breakpoint(b) => Some(b),
            _ => None,
        }
    }
}
