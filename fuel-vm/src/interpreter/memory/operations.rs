use super::super::internal::inc_pc;
use super::super::{ExecutableTransaction, Interpreter};
use super::{MemoryRange, ToAddr};
use crate::constraints::reg_key::*;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::{PanicReason, RegId};
use fuel_types::{RegisterId, Word};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Modify stack pointer using an overflowing-style arithmetic operation.
    pub(crate) fn stack_pointer_overflow<F>(&mut self, f: F, v: Word) -> Result<(), RuntimeError>
    where
        F: FnOnce(Word, Word) -> (Word, bool),
    {
        let (SystemRegisters { mut sp, ssp, hp, .. }, _) = split_registers(&mut self.registers);
        let (result, overflow) = f(*sp, v);

        if overflow || result >= *hp || result < *ssp {
            Err(PanicReason::OutOfMemory.into())
        } else {
            *sp = result;

            let pages = self
                .memory
                .update_allocations(*sp, *hp)
                .map_err(|_| PanicReason::OutOfMemory)?;

            if let Some(charge) = pages.maybe_cost(self.gas_costs.memory_page) {
                self.gas_charge(charge)?;
            }

            inc_pc(self.registers.pc_mut())
        }
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        let addr = b.checked_add(c).ok_or(PanicReason::MemoryAccess)?;
        let bytes: [u8; 1] = self.mem_read_bytes(addr)?;
        self.registers[ra] = bytes[0] as Word;
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
        let addr = b.checked_add(c * 8).ok_or(PanicReason::MemoryAccess)?;
        self.registers[ra] = Word::from_be_bytes(self.mem_read_bytes(addr)?);
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (ac, overflow) = a.overflowing_add(c);
        if overflow {
            return Err(PanicReason::MemoryAccess.into());
        }

        self.mem_write_bytes(ac, &[b as u8])?;

        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
        let addr = a.checked_add(c * 8).ok_or(PanicReason::MemoryAccess)?;
        self.mem_write_bytes(addr, &b.to_be_bytes())?;
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn malloc(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { mut hp, sp, .. }, _) = split_registers(&mut self.registers);
        let (result, overflow) = hp.overflowing_sub(a);
        if overflow || result < *sp {
            Err(PanicReason::OutOfMemory.into())
        } else {
            *hp = result;

            self.update_allocations()?;
            inc_pc(self.registers.pc_mut())
        }
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        self.mem_write(a, b)?.fill(0);
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let dst_range = MemoryRange::try_new(a, c)?;
        let src_range = MemoryRange::try_new(b, c)?;

        self.check_mem_owned(&dst_range)?;
        self.check_mem_access(&src_range)?;

        self.memory.try_copy_within(&dst_range, &src_range)?;
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn memeq(&mut self, ra: RegisterId, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let range0 = MemoryRange::try_new(b, d)?;
        let range1 = MemoryRange::try_new(c, d)?;

        if range0.len() > MEM_MAX_ACCESS_SIZE {
            return Err(PanicReason::MemoryAccessSize.into());
        }

        let eq = self.memory.read(&range0) == self.memory.read(&range1);
        self.registers[ra] = eq as Word;

        inc_pc(self.registers.pc_mut())
    }
}

impl<S, Tx> Interpreter<S, Tx> {
    /// Lower level check for memory access rights
    pub fn check_mem_access(&self, range: &MemoryRange) -> Result<(), RuntimeError> {
        let stack = MemoryRange::try_new(0, self.registers[RegId::SP]).expect("Invalid SP value");
        let heap = MemoryRange::try_new(self.registers[RegId::HP], VM_MAX_RAM - self.registers[RegId::HP])
            .expect("Invalid HP value");

        if range.len() > MEM_MAX_ACCESS_SIZE {
            return Err(PanicReason::MemoryAccessSize.into());
        }

        if stack.contains_range(range) && heap.contains_range(range) {
            Ok(())
        } else {
            Err(PanicReason::MemoryAccess.into())
        }
    }

    /// Lower level check for memory write rights
    pub fn check_mem_owned(&self, range: &MemoryRange) -> Result<(), RuntimeError> {
        self.check_mem_access(range)?;
        if self.ownership_registers().has_ownership_range(range) {
            Ok(())
        } else {
            Err(PanicReason::MemoryOwnership.into())
        }
    }

    /// Get a read-only acccess to a range of memory, performing access checks
    pub fn mem_read_range(&self, range: &MemoryRange) -> Result<&[u8], RuntimeError> {
        self.check_mem_access(range)?;
        Ok(self.memory.read(range))
    }
    /// Get a read-only acccess to a range of memory, performing access checks
    pub fn mem_read<A: ToAddr, B: ToAddr>(&self, addr: A, len: B) -> Result<&[u8], RuntimeError> {
        let range = MemoryRange::try_new(addr, len)?;
        self.mem_read_range(&range)
    }

    /// Read a fixed-size byte array of memory, performing access checks
    pub fn mem_read_bytes<A: ToAddr, const LEN: usize>(&self, addr: A) -> Result<[u8; LEN], RuntimeError> {
        let mut buf = [0u8; LEN];
        buf.copy_from_slice(self.mem_read(addr, LEN)?);
        Ok(buf)
    }

    /// Get a write access to a range of memory, performing ownership checks
    pub fn mem_write_range(&mut self, range: &MemoryRange) -> Result<&mut [u8], RuntimeError> {
        self.check_mem_owned(range)?;
        Ok(self.memory.write(range))
    }

    /// Get a write access to a range of memory, performing ownership checks
    pub fn mem_write<A: ToAddr, B: ToAddr>(&mut self, addr: A, len: B) -> Result<&mut [u8], RuntimeError> {
        let range = MemoryRange::try_new(addr, len)?;
        self.mem_write_range(&range)
    }

    /// Write a fixed-size byte array of memory, performing ownership checks
    pub fn mem_write_bytes<A: ToAddr, const LEN: usize>(
        &mut self,
        addr: A,
        data: &[u8; LEN],
    ) -> Result<(), RuntimeError> {
        self.mem_write(addr, data.len())?.copy_from_slice(data);
        Ok(())
    }

    /// Write a slice to memory, performing ownership checks
    pub fn mem_write_slice<A: ToAddr>(&mut self, addr: A, data: &[u8]) -> Result<(), RuntimeError> {
        self.mem_write(addr, data.len())?.copy_from_slice(data);
        Ok(())
    }

    /// Update the memory allocations based on the current SP and HP values.
    /// This must be called after every time new memory is allocated.
    pub fn update_allocations(&mut self) -> Result<(), RuntimeError> {
        let pages = self
            .memory
            .update_allocations(self.registers[RegId::SP], self.registers[RegId::HP])
            .map_err(|_| PanicReason::OutOfMemory)?;

        if let Some(charge) = pages.maybe_cost(self.gas_costs.memory_page) {
            self.gas_charge(charge)?;
        }

        Ok(())
    }
}
