//! Utilities for accessing register values and proving at compile time that
//! the register index is valid.
//!
//! This module also provides utilities for mutably accessing multiple registers.
use std::ops::Deref;
use std::ops::DerefMut;

use fuel_asm::RegId;
use fuel_asm::Word;

use crate::consts::VM_REGISTER_COUNT;
use crate::consts::VM_REGISTER_WRITE_COUNT;

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

pub(crate) struct ReadRegisters<'a> {
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

pub(crate) struct ReadRegistersRef<'a> {
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
pub(crate) struct WriteRegisters<'a>(pub &'a mut [Word; VM_REGISTER_WRITE_COUNT]);
pub(crate) struct WriteRegistersRef<'a>(pub &'a [Word; VM_REGISTER_WRITE_COUNT]);

pub(crate) fn split_registers(registers: &mut [Word; VM_REGISTER_COUNT]) -> (ReadRegisters<'_>, WriteRegisters<'_>) {
    let [zero, one, of, pc, ssp, sp, fp, hp, err, ggas, cgas, bal, is, ret, retl, flag, rest @ ..] = registers;
    let r = ReadRegisters {
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
    (r, WriteRegisters(rest))
}

pub(crate) fn copy_registers(
    read_registers: &ReadRegistersRef<'_>,
    write_registers: &WriteRegistersRef<'_>,
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

impl<'a> From<&'a ReadRegisters<'_>> for ReadRegistersRef<'a> {
    fn from(value: &'a ReadRegisters<'_>) -> Self {
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

impl<'a> From<ReadRegisters<'a>> for ReadRegistersRef<'a> {
    fn from(value: ReadRegisters<'a>) -> Self {
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

impl<'a> From<&'a WriteRegisters<'_>> for WriteRegistersRef<'a> {
    fn from(value: &'a WriteRegisters<'_>) -> Self {
        Self(value.0)
    }
}

impl<'a> From<WriteRegisters<'a>> for WriteRegistersRef<'a> {
    fn from(value: WriteRegisters<'a>) -> Self {
        Self(value.0)
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

    assert_eq!(r.zero, RegMut::<ZERO>(&mut (ZERO as u64)));
    assert_eq!(r.one, RegMut::<ONE>(&mut (ONE as u64)));
    assert_eq!(r.of, RegMut::<OF>(&mut (OF as u64)));
    assert_eq!(r.pc, RegMut::<PC>(&mut (PC as u64)));
    assert_eq!(r.ssp, RegMut::<SSP>(&mut (SSP as u64)));
    assert_eq!(r.sp, RegMut::<SP>(&mut (SP as u64)));
    assert_eq!(r.fp, RegMut::<FP>(&mut (FP as u64)));
    assert_eq!(r.hp, RegMut::<HP>(&mut (HP as u64)));
    assert_eq!(r.err, RegMut::<ERR>(&mut (ERR as u64)));
    assert_eq!(r.ggas, RegMut::<GGAS>(&mut (GGAS as u64)));
    assert_eq!(r.cgas, RegMut::<CGAS>(&mut (CGAS as u64)));
    assert_eq!(r.bal, RegMut::<BAL>(&mut (BAL as u64)));
    assert_eq!(r.is, RegMut::<IS>(&mut (IS as u64)));
    assert_eq!(r.ret, RegMut::<RET>(&mut (RET as u64)));
    assert_eq!(r.retl, RegMut::<RETL>(&mut (RETL as u64)));
    assert_eq!(r.flag, RegMut::<FLAG>(&mut (FLAG as u64)));

    for i in 0..VM_REGISTER_WRITE_COUNT {
        assert_eq!(w.0[i], i as u64 + 16);
    }

    let reg = copy_registers(&(&r).into(), &(&w).into());

    assert_eq!(reg, expect);
}
