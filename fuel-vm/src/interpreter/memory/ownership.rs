use std::ops::Range;

use fuel_asm::RegId;
use fuel_types::Word;

use crate::{
    consts::VM_MAX_RAM,
    context::Context,
    prelude::{ExecutableTransaction, Interpreter},
};

use super::MemoryRange;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Return the registers used to determine ownership.
    pub(crate) fn ownership_registers(&self) -> OwnershipRegisters {
        OwnershipRegisters::new(self)
    }
}

pub struct OwnershipRegisters {
    pub(crate) sp: u64,
    pub(crate) ssp: u64,
    pub(crate) hp: u64,
    pub(crate) prev_hp: u64,
    pub(crate) context: Context,
}

impl OwnershipRegisters {
    pub(crate) fn new<S, Tx>(vm: &Interpreter<S, Tx>) -> Self {
        OwnershipRegisters {
            sp: vm.registers[RegId::SP],
            ssp: vm.registers[RegId::SSP],
            hp: vm.registers[RegId::HP],
            prev_hp: vm.frames.last().map(|frame| frame.registers()[RegId::HP]).unwrap_or(0),
            context: vm.context.clone(),
        }
    }
    pub(crate) fn has_ownership_range(&self, range: &MemoryRange) -> bool {
        let (start_incl, end_excl) = range.boundaries(self);
        let range = start_incl..end_excl;
        self.has_ownership_stack(&range) || self.has_ownership_heap(&range)
    }

    /// Empty range is owned iff the range.start is owned
    pub(crate) fn has_ownership_stack(&self, range: &Range<Word>) -> bool {
        if range.is_empty() && range.start == self.ssp {
            return true;
        }

        if !(self.ssp..self.sp).contains(&range.start) {
            return false;
        }

        if range.end > VM_MAX_RAM {
            return false;
        }

        (self.ssp..=self.sp).contains(&range.end)
    }

    /// Empty range is owned iff the range.start is owned
    pub(crate) fn has_ownership_heap(&self, range: &Range<Word>) -> bool {
        // TODO implement fp->hp and (addr, size) validations
        // fp->hp
        // it means $hp from the previous context, i.e. what's saved in the
        // "Saved registers from previous context" of the call frame at
        // $fp`
        if range.start < self.hp {
            return false;
        }

        let heap_end = if self.context.is_external() {
            VM_MAX_RAM
        } else {
            self.prev_hp
        };

        self.hp != heap_end && range.end <= heap_end
    }
}
