use core::ops::{
    Index,
    IndexMut,
};

use fuel_asm::{
    RegId,
    RegR,
    RegW,
    Word,
};
use fuel_types::canonical::{
    Deserialize,
    Serialize,
};

use super::VM_REGISTER_COUNT;

/// Registers of the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Registers(pub [Word; VM_REGISTER_COUNT]);

impl Registers {
    /// All registers set to zero, even the `ONE` register.
    pub const ALL_ZERO: Self = Self([0; VM_REGISTER_COUNT]);
}

impl Index<RegId> for Registers {
    type Output = Word;

    fn index(&self, index: RegId) -> &Self::Output {
        &self.0[index.to_u8() as usize]
    }
}

impl Index<RegR> for Registers {
    type Output = Word;

    fn index(&self, index: RegR) -> &Self::Output {
        &self.0[index.to_u8() as usize]
    }
}

impl Index<RegW> for Registers {
    type Output = Word;

    fn index(&self, index: RegW) -> &Self::Output {
        &self.0[index.to_u8() as usize]
    }
}

impl IndexMut<RegId> for Registers {
    fn index_mut(&mut self, index: RegId) -> &mut Self::Output {
        &mut self.0[index.to_u8() as usize]
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<usize> for Registers {
    type Output = Word;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl IndexMut<usize> for Registers {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
