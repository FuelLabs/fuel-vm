//! Utilities for accessing register values and proving at compile time that
//! the register index is valid.
//!
//! This module also provides utilities for mutably accessing multiple registers.
use std::ops::Deref;
use std::ops::DerefMut;

use fuel_asm::RegId;
use fuel_asm::Word;

use crate::consts::VM_REGISTER_COUNT;
use crate::consts::VM_REGISTER_PROGRAM_COUNT;
use crate::consts::VM_REGISTER_SYSTEM_COUNT;

#[derive(Debug, PartialEq, Eq)]
/// Mutable reference to a register value at a given index.
pub struct RegMut<'r, const INDEX: u8>(&'r mut Word);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Immutable reference to a register value at a given index.
pub struct Reg<'r, const INDEX: u8>(&'r Word);

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
    ( $($i:ident, $f:ident)* ) => {
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
        impl GetReg for [Word; VM_REGISTER_COUNT] {
        $(
            fn $f(&self) -> Reg<'_, $i> {
                Reg(&self[$i as usize])
            }
        )*
        }
    };
}

impl_keys! {
    ZERO, zero
    ONE, one
    OF, of
    PC, pc
    SSP, ssp
    SP, sp
    FP, fp
    HP, hp
    ERR, err
    GGAS, ggas
    CGAS, cgas
    BAL, bal
    IS, is
    RET, ret
    RETL, retl
    FLAG, flag
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
    let mut out = [0u64; VM_REGISTER_COUNT];
    out[..VM_REGISTER_SYSTEM_COUNT].copy_from_slice(&<[Word; VM_REGISTER_SYSTEM_COUNT]>::from(read_registers));
    out[VM_REGISTER_SYSTEM_COUNT..].copy_from_slice(write_registers.0);
    out
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
            *zero.0, *one.0, *of.0, *pc.0, *ssp.0, *sp.0, *fp.0, *hp.0, *err.0, *ggas.0, *cgas.0, *bal.0, *is.0,
            *ret.0, *retl.0, *flag.0,
        ]
    }
}

#[test]
fn can_split() {
    let mut reg: [Word; VM_REGISTER_COUNT] = std::iter::successors(Some(0), |x| Some(x + 1))
        .take(VM_REGISTER_COUNT)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let expect = reg;

    let (r, w) = split_registers(&mut reg);

    let SystemRegisters {
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
    } = &r;

    assert_eq!(*zero, RegMut::<ZERO>(&mut (ZERO as u64)));
    assert_eq!(*one, RegMut::<ONE>(&mut (ONE as u64)));
    assert_eq!(*of, RegMut::<OF>(&mut (OF as u64)));
    assert_eq!(*pc, RegMut::<PC>(&mut (PC as u64)));
    assert_eq!(*ssp, RegMut::<SSP>(&mut (SSP as u64)));
    assert_eq!(*sp, RegMut::<SP>(&mut (SP as u64)));
    assert_eq!(*fp, RegMut::<FP>(&mut (FP as u64)));
    assert_eq!(*hp, RegMut::<HP>(&mut (HP as u64)));
    assert_eq!(*err, RegMut::<ERR>(&mut (ERR as u64)));
    assert_eq!(*ggas, RegMut::<GGAS>(&mut (GGAS as u64)));
    assert_eq!(*cgas, RegMut::<CGAS>(&mut (CGAS as u64)));
    assert_eq!(*bal, RegMut::<BAL>(&mut (BAL as u64)));
    assert_eq!(*is, RegMut::<IS>(&mut (IS as u64)));
    assert_eq!(*ret, RegMut::<RET>(&mut (RET as u64)));
    assert_eq!(*retl, RegMut::<RETL>(&mut (RETL as u64)));
    assert_eq!(*flag, RegMut::<FLAG>(&mut (FLAG as u64)));

    for i in 0..VM_REGISTER_PROGRAM_COUNT {
        assert_eq!(w.0[i], i as u64 + 16);
    }

    let reg = copy_registers(&(&r).into(), &(&w).into());

    assert_eq!(reg, expect);
}
