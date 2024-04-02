//! Types to help constrain inputs to functions to only what is used.

use fuel_types::ContractId;

pub mod reg_key;

/// Location of an instructing collected during runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionLocation {
    /// Context, i.e. current contract. None if running a script.
    pub context: Option<ContractId>,
    /// Offset from the IS register
    pub offset: u64,
}

// impl<const LEN: usize> Deref for CheckedMemConstLen<LEN> {
//     type Target = MemoryRange;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl<const LEN: usize> DerefMut for CheckedMemConstLen<LEN> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }
