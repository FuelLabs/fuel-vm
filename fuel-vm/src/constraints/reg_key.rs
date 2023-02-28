//! Utilities for accessing register values and proving at compile time that
//! the register index is valid.
//!
//! This module also provides utilities for mutably accessing multiple registers.
use std::ops::Deref;
use std::ops::DerefMut;

use fuel_asm::PanicReason;
use fuel_asm::RegId;
use fuel_asm::RegisterId;
use fuel_asm::Word;

use crate::consts::VM_REGISTER_COUNT;
use crate::consts::VM_REGISTER_PROGRAM_COUNT;
use crate::consts::VM_REGISTER_SYSTEM_COUNT;
use crate::prelude::RuntimeError;

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
pub struct WriteRegKey(usize);

impl WriteRegKey {
    /// Create a new writable register key if the index is within the bounds
    /// of the writable registers.
    pub fn new(k: impl Into<usize>) -> Result<Self, RuntimeError> {
        let k = k.into();
        is_register_writable(&k)?;
        Ok(Self(k))
    }

    fn translate(self) -> usize {
        self.0 - VM_REGISTER_SYSTEM_COUNT
    }
}

pub(crate) fn is_register_writable(ra: &RegisterId) -> Result<(), RuntimeError> {
    const W_USIZE: usize = RegId::WRITABLE.to_u8() as usize;
    const RANGE: core::ops::Range<usize> = W_USIZE..(W_USIZE + VM_REGISTER_PROGRAM_COUNT);
    if RANGE.contains(ra) {
        Ok(())
    } else {
        Err(RuntimeError::Recoverable(PanicReason::ReservedRegisterNotWritable))
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
impl<'r, const INDEX: u8> RegMut<'r, INDEX> {
    /// Re-borrow the register as an immutable reference.
    pub fn as_ref(&self) -> Reg<INDEX> {
        Reg(self.0)
    }
}

impl<'r, const INDEX: u8> RegMut<'r, INDEX> {
    /// Re-borrow the register as a mutable reference.
    pub fn as_mut(&mut self) -> RegMut<INDEX> {
        RegMut(self.0)
    }
}

macro_rules! impl_keys {
    ( $($i:ident, $f:ident, $f_mut:ident)* ) => {
        $(
            #[doc = "Register index key for use with Reg and RegMut."]
            pub const $i: u8 = RegId::$i.to_u8();
        )*
        #[doc = "Get register reference by name."]
        pub trait GetReg {
        $(
            #[doc = "Get register reference for this key."]
            fn $f(&self) -> Reg<'_, $i>;

            #[doc = "Get mutable register reference for this key."]
            fn $f_mut(&mut self) -> RegMut<'_, $i>;
        )*
        }
        impl GetReg for [Word; VM_REGISTER_COUNT] {
        $(
            fn $f(&self) -> Reg<'_, $i> {
                Reg(&self[$i as usize])
            }

            fn $f_mut(&mut self) -> RegMut<'_, $i> {
                RegMut(&mut self[$i as usize])
            }
        )*
        }
    };
}

impl_keys! {
    ZERO, zero, zero_mut
    ONE, one, one_mut
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
pub(crate) struct ProgramRegisters<'a>(pub &'a mut [Word; VM_REGISTER_PROGRAM_COUNT]);
pub(crate) struct ProgramRegistersRef<'a>(pub &'a [Word; VM_REGISTER_PROGRAM_COUNT]);

pub(crate) fn split_registers(
    registers: &mut [Word; VM_REGISTER_COUNT],
) -> (SystemRegisters<'_>, ProgramRegisters<'_>) {
    let [zero, one, of, pc, ssp, sp, fp, hp, err, ggas, cgas, bal, is, ret, retl, flag, rest @ ..] = registers;
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

pub(crate) fn copy_registers(
    read_registers: &SystemRegistersRef<'_>,
    write_registers: &ProgramRegistersRef<'_>,
) -> [Word; VM_REGISTER_COUNT] {
    [
        *read_registers.zero,
        *read_registers.one,
        *read_registers.of,
        *read_registers.pc,
        *read_registers.ssp,
        *read_registers.sp,
        *read_registers.fp,
        *read_registers.hp,
        *read_registers.err,
        *read_registers.ggas,
        *read_registers.cgas,
        *read_registers.bal,
        *read_registers.is,
        *read_registers.ret,
        *read_registers.retl,
        *read_registers.flag,
        write_registers.0[0],
        write_registers.0[1],
        write_registers.0[2],
        write_registers.0[3],
        write_registers.0[4],
        write_registers.0[5],
        write_registers.0[6],
        write_registers.0[7],
        write_registers.0[8],
        write_registers.0[9],
        write_registers.0[10],
        write_registers.0[11],
        write_registers.0[12],
        write_registers.0[13],
        write_registers.0[14],
        write_registers.0[15],
        write_registers.0[16],
        write_registers.0[17],
        write_registers.0[18],
        write_registers.0[19],
        write_registers.0[20],
        write_registers.0[21],
        write_registers.0[22],
        write_registers.0[23],
        write_registers.0[24],
        write_registers.0[25],
        write_registers.0[26],
        write_registers.0[27],
        write_registers.0[28],
        write_registers.0[29],
        write_registers.0[30],
        write_registers.0[31],
        write_registers.0[32],
        write_registers.0[33],
        write_registers.0[34],
        write_registers.0[35],
        write_registers.0[36],
        write_registers.0[37],
        write_registers.0[38],
        write_registers.0[39],
        write_registers.0[40],
        write_registers.0[41],
        write_registers.0[42],
        write_registers.0[43],
        write_registers.0[44],
        write_registers.0[45],
        write_registers.0[46],
        write_registers.0[47],
    ]
}

impl<'r> ProgramRegisters<'r> {
    pub fn split(&mut self, a: WriteRegKey, b: WriteRegKey) -> Option<(&mut Word, &mut Word)> {
        match a.cmp(&b) {
            std::cmp::Ordering::Less => {
                let a = a.translate();
                let [i, rest @ ..] = &mut self.0[a..] else { return None };
                let b = b.translate() - 1 - a;
                let j = &mut rest[b];
                Some((i, j))
            }
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => {
                let b = b.translate();
                let [i, rest @ ..] = &mut self.0[b..] else { return None };
                let a = a.translate() - 1 - b;
                let j = &mut rest[a];
                Some((j, i))
            }
        }
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

impl TryFrom<RegisterId> for WriteRegKey {
    type Error = RuntimeError;
    fn try_from(ra: RegisterId) -> Result<Self, Self::Error> {
        Self::new(ra)
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
