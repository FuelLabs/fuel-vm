//! Utilities for accessing register values and proving at compile time that
//! the register index is valid.
//!
//! This module also provides utilities for mutably accessing multiple registers.
use core::ops::{
    Deref,
    DerefMut,
};

use fuel_asm::{
    PanicReason,
    RegId,
    Word,
};

use crate::consts::{
    VM_REGISTER_COUNT,
    VM_REGISTER_PROGRAM_COUNT,
    VM_REGISTER_SYSTEM_COUNT,
};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq)]
/// Mutable reference to a register value at a given index.
pub struct RegMut<'r, const INDEX: u8>(&'r mut Word);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Immutable reference to a register value at a given index.
pub struct Reg<'r, const INDEX: u8>(&'r Word);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A key to a writable register that is within
/// the bounds of the writable registers.
pub struct WriteRegKey(RegId);

impl WriteRegKey {
    /// Create a new writable register key if the index is within the bounds
    /// of the writable registers.
    pub fn new(k: impl Into<RegId>) -> Result<Self, PanicReason> {
        let k = k.into();

        if k >= RegId::WRITABLE {
            Ok(Self(k))
        } else {
            Err(PanicReason::ReservedRegisterNotWritable)
        }
    }

    /// Translate this key from an absolute register index
    /// to a program register index.
    ///
    /// This subtracts the number of system registers from the key.
    #[allow(clippy::arithmetic_side_effects)] // Safety: checked in constructor
    fn translate(self) -> usize {
        self.0.to_u8() as usize - VM_REGISTER_SYSTEM_COUNT
    }
}

impl<'r, const INDEX: u8> RegMut<'r, INDEX> {
    /// Create a new mutable register reference.
    pub fn new(reg: &'r mut Word) -> Self {
        Self(reg)
    }
}

impl<'r, const INDEX: u8> Reg<'r, INDEX> {
    /// Create a new immutable register reference.
    pub fn new(reg: &'r Word) -> Self {
        Self(reg)
    }
}

impl<const INDEX: u8> Deref for Reg<'_, INDEX> {
    type Target = Word;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<const INDEX: u8> Deref for RegMut<'_, INDEX> {
    type Target = Word;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<const INDEX: u8> DerefMut for RegMut<'_, INDEX> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a, const INDEX: u8> From<RegMut<'a, INDEX>> for Reg<'a, INDEX> {
    fn from(reg: RegMut<'a, INDEX>) -> Self {
        Self(reg.0)
    }
}

impl<const INDEX: u8> RegMut<'_, INDEX> {
    /// Re-borrow the register as an immutable reference.
    pub fn as_ref(&self) -> Reg<INDEX> {
        Reg(self.0)
    }
}

impl<const INDEX: u8> RegMut<'_, INDEX> {
    /// Re-borrow the register as a mutable reference.
    pub fn as_mut(&mut self) -> RegMut<INDEX> {
        RegMut(self.0)
    }
}

macro_rules! impl_keys {
    ( $($i:ident, $f:ident $(,$f_mut:ident)?)* ) => {
        $(
            #[doc = "Register index key for use with Reg and RegMut."]
            pub const $i: u8 = RegId::$i.to_u8();
        )*
        #[doc = "Get register reference by name."]
        pub trait GetReg {
        $(
            #[doc = "Get register reference for this key."]
            fn $f(&self) -> Reg<'_, $i>;
        )*
        }
        #[doc = "Get register mutable reference by name."]
        pub trait GetRegMut {
        $(
            $(
            #[doc = "Get mutable register reference for this key."]
            fn $f_mut(&mut self) -> RegMut<'_, $i>;
            )?
        )*
        }
        impl GetReg for [Word; VM_REGISTER_COUNT] {
        $(
            fn $f(&self) -> Reg<'_, $i> {
                Reg(&self[$i as usize])
            }
        )*
        }
        impl GetRegMut for [Word; VM_REGISTER_COUNT] {
        $(
            $(
            fn $f_mut(&mut self) -> RegMut<'_, $i> {
                RegMut(&mut self[$i as usize])
            }
            )?
        )*
        }
    };
}

impl_keys! {
    ZERO, zero
    ONE, one
    OF, of, of_mut
    PC, pc, pc_mut
    SSP, ssp, ssp_mut
    SP, sp, sp_mut
    FP, fp, fp_mut
    HP, hp, hp_mut
    ERR, err, err_mut
    GGAS, ggas, ggas_mut
    CGAS, cgas, cgas_mut
    BAL, bal, bal_mut
    IS, is, is_mut
    RET, ret, ret_mut
    RETL, retl, retl_mut
    FLAG, flag, flag_mut
}

/// The set of system registers split into
/// individual mutable references.
pub(crate) struct SystemRegisters<'a> {
    pub(crate) zero: RegMut<'a, ZERO>,
    pub(crate) one: RegMut<'a, ONE>,
    pub(crate) of: RegMut<'a, OF>,
    pub(crate) pc: RegMut<'a, PC>,
    pub(crate) ssp: RegMut<'a, SSP>,
    pub(crate) sp: RegMut<'a, SP>,
    pub(crate) fp: RegMut<'a, FP>,
    pub(crate) hp: RegMut<'a, HP>,
    pub(crate) err: RegMut<'a, ERR>,
    pub(crate) ggas: RegMut<'a, GGAS>,
    pub(crate) cgas: RegMut<'a, CGAS>,
    pub(crate) bal: RegMut<'a, BAL>,
    pub(crate) is: RegMut<'a, IS>,
    pub(crate) ret: RegMut<'a, RET>,
    pub(crate) retl: RegMut<'a, RETL>,
    pub(crate) flag: RegMut<'a, FLAG>,
}

/// Same as `SystemRegisters` but with immutable references.
pub(crate) struct SystemRegistersRef<'a> {
    pub(crate) zero: Reg<'a, ZERO>,
    pub(crate) one: Reg<'a, ONE>,
    pub(crate) of: Reg<'a, OF>,
    pub(crate) pc: Reg<'a, PC>,
    pub(crate) ssp: Reg<'a, SSP>,
    pub(crate) sp: Reg<'a, SP>,
    pub(crate) fp: Reg<'a, FP>,
    pub(crate) hp: Reg<'a, HP>,
    pub(crate) err: Reg<'a, ERR>,
    pub(crate) ggas: Reg<'a, GGAS>,
    pub(crate) cgas: Reg<'a, CGAS>,
    pub(crate) bal: Reg<'a, BAL>,
    pub(crate) is: Reg<'a, IS>,
    pub(crate) ret: Reg<'a, RET>,
    pub(crate) retl: Reg<'a, RETL>,
    pub(crate) flag: Reg<'a, FLAG>,
}

/// The set of program registers split from the system registers.
pub(crate) struct ProgramRegisters<'a>(pub &'a mut [Word; VM_REGISTER_PROGRAM_COUNT]);

/// Same as `ProgramRegisters` but with immutable references.
pub(crate) struct ProgramRegistersRef<'a>(pub &'a [Word; VM_REGISTER_PROGRAM_COUNT]);

/// Split the registers into system and program registers.
///
/// This allows multiple mutable references to registers.
pub(crate) fn split_registers(
    registers: &mut [Word; VM_REGISTER_COUNT],
) -> (SystemRegisters<'_>, ProgramRegisters<'_>) {
    let [zero, one, of, pc, ssp, sp, fp, hp, err, ggas, cgas, bal, is, ret, retl, flag, rest @ ..] =
        registers;
    let r = SystemRegisters {
        zero: RegMut(zero),
        one: RegMut(one),
        of: RegMut(of),
        pc: RegMut(pc),
        ssp: RegMut(ssp),
        sp: RegMut(sp),
        fp: RegMut(fp),
        hp: RegMut(hp),
        err: RegMut(err),
        ggas: RegMut(ggas),
        cgas: RegMut(cgas),
        bal: RegMut(bal),
        is: RegMut(is),
        ret: RegMut(ret),
        retl: RegMut(retl),
        flag: RegMut(flag),
    };
    (r, ProgramRegisters(rest))
}

/// Copy the system and program registers into a single array.
pub(crate) fn copy_registers(
    system_registers: &SystemRegistersRef<'_>,
    program_registers: &ProgramRegistersRef<'_>,
) -> [Word; VM_REGISTER_COUNT] {
    let mut out = [0u64; VM_REGISTER_COUNT];
    out[..VM_REGISTER_SYSTEM_COUNT]
        .copy_from_slice(&<[Word; VM_REGISTER_SYSTEM_COUNT]>::from(system_registers));
    out[VM_REGISTER_SYSTEM_COUNT..].copy_from_slice(program_registers.0);
    out
}

impl ProgramRegisters<'_> {
    /// Get two mutable references to program registers.
    /// Note they cannot be the same register.
    pub fn get_mut_two(
        &mut self,
        a: WriteRegKey,
        b: WriteRegKey,
    ) -> Option<(&mut Word, &mut Word)> {
        if a == b {
            // Cannot mutably borrow the same register twice.
            return None
        }

        // Order registers
        let swap = a > b;
        let (a, b) = if swap { (b, a) } else { (a, b) };

        // Translate the absolute register indices to a program register indeces.
        let a = a.translate();

        // Subtract a + 1 because because we split the array at `a`.
        let b = b
            .translate()
            .checked_sub(a.saturating_add(1))
            .expect("Cannot underflow as the values are ordered");

        // Split the array at the first register which is a.
        let [i, rest @ ..] = &mut self.0[a..] else {
            return None
        };

        // Translate the higher absolute register index to a program register index.
        // Get the `b` register.
        let j = &mut rest[b];

        Some(if swap { (j, i) } else { (i, j) })
    }
}

impl<'a> From<&'a SystemRegisters<'_>> for SystemRegistersRef<'a> {
    fn from(value: &'a SystemRegisters<'_>) -> Self {
        Self {
            zero: Reg(value.zero.0),
            one: Reg(value.one.0),
            of: Reg(value.of.0),
            pc: Reg(value.pc.0),
            ssp: Reg(value.ssp.0),
            sp: Reg(value.sp.0),
            fp: Reg(value.fp.0),
            hp: Reg(value.hp.0),
            err: Reg(value.err.0),
            ggas: Reg(value.ggas.0),
            cgas: Reg(value.cgas.0),
            bal: Reg(value.bal.0),
            is: Reg(value.is.0),
            ret: Reg(value.ret.0),
            retl: Reg(value.retl.0),
            flag: Reg(value.flag.0),
        }
    }
}

impl<'a> From<SystemRegisters<'a>> for SystemRegistersRef<'a> {
    fn from(value: SystemRegisters<'a>) -> Self {
        Self {
            zero: Reg(value.zero.0),
            one: Reg(value.one.0),
            of: Reg(value.of.0),
            pc: Reg(value.pc.0),
            ssp: Reg(value.ssp.0),
            sp: Reg(value.sp.0),
            fp: Reg(value.fp.0),
            hp: Reg(value.hp.0),
            err: Reg(value.err.0),
            ggas: Reg(value.ggas.0),
            cgas: Reg(value.cgas.0),
            bal: Reg(value.bal.0),
            is: Reg(value.is.0),
            ret: Reg(value.ret.0),
            retl: Reg(value.retl.0),
            flag: Reg(value.flag.0),
        }
    }
}

impl<'a> From<&'a ProgramRegisters<'_>> for ProgramRegistersRef<'a> {
    fn from(value: &'a ProgramRegisters<'_>) -> Self {
        Self(value.0)
    }
}

impl<'a> From<ProgramRegisters<'a>> for ProgramRegistersRef<'a> {
    fn from(value: ProgramRegisters<'a>) -> Self {
        Self(value.0)
    }
}

impl TryFrom<RegId> for WriteRegKey {
    type Error = PanicReason;

    fn try_from(r: RegId) -> Result<Self, Self::Error> {
        Self::new(r)
    }
}

impl core::ops::Index<WriteRegKey> for ProgramRegisters<'_> {
    type Output = Word;

    fn index(&self, index: WriteRegKey) -> &Self::Output {
        &self.0[index.translate()]
    }
}

impl core::ops::IndexMut<WriteRegKey> for ProgramRegisters<'_> {
    fn index_mut(&mut self, index: WriteRegKey) -> &mut Self::Output {
        &mut self.0[index.translate()]
    }
}

impl<'a> From<&SystemRegistersRef<'a>> for [Word; VM_REGISTER_SYSTEM_COUNT] {
    fn from(value: &SystemRegistersRef<'a>) -> Self {
        let SystemRegistersRef {
            zero,
            one,
            of,
            pc,
            ssp,
            sp,
            fp,
            hp,
            err,
            ggas,
            cgas,
            bal,
            is,
            ret,
            retl,
            flag,
        } = value;
        [
            *zero.0, *one.0, *of.0, *pc.0, *ssp.0, *sp.0, *fp.0, *hp.0, *err.0, *ggas.0,
            *cgas.0, *bal.0, *is.0, *ret.0, *retl.0, *flag.0,
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ProgramRegistersSegment {
    /// Registers 16..40
    Low,
    /// Registers 40..64
    High,
}

impl ProgramRegisters<'_> {
    /// Returns the registers corresponding to the segment, always 24 elements.
    pub(crate) fn segment(&self, segment: ProgramRegistersSegment) -> &[Word] {
        match segment {
            ProgramRegistersSegment::Low => &self.0[..24],
            ProgramRegistersSegment::High => &self.0[24..],
        }
    }

    /// Returns the registers corresponding to the segment, always 24 elements.
    pub(crate) fn segment_mut(
        &mut self,
        segment: ProgramRegistersSegment,
    ) -> &mut [Word] {
        match segment {
            ProgramRegistersSegment::Low => &mut self.0[..24],
            ProgramRegistersSegment::High => &mut self.0[24..],
        }
    }
}
