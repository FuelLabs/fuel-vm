use super::{
    internal::inc_pc,
    is_unsafe_math,
    is_wrapping,
    ExecutableTransaction,
    Interpreter,
};
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
};

use fuel_asm::PanicReason;
use fuel_types::{
    RegisterId,
    Word,
};

mod muldiv;
mod wideint;

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
{
    /// Stores the overflowed wrapped value into RegId::OF
    pub(crate) fn alu_capture_overflow<F, B, C>(
        &mut self,
        ra: RegisterId,
        f: F,
        b: B,
        c: C,
    ) -> SimpleResult<()>
    where
        F: FnOnce(B, C) -> (u128, bool),
    {
        let (
            SystemRegisters {
                flag, of, err, pc, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let dest = &mut w[ra.try_into()?];
        let common = AluCommonReg { of, err, pc };
        alu_capture_overflow(dest, flag.as_ref(), common, f, b, c)
    }

    /// Set RegId::OF to true and zero the result register if overflow occurred.
    pub(crate) fn alu_boolean_overflow<F, B, C>(
        &mut self,
        ra: RegisterId,
        f: F,
        b: B,
        c: C,
    ) -> SimpleResult<()>
    where
        F: FnOnce(B, C) -> (Word, bool),
    {
        let (
            SystemRegisters {
                flag, of, err, pc, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let dest = &mut w[ra.try_into()?];
        let common = AluCommonReg { of, err, pc };
        alu_boolean_overflow(dest, flag.as_ref(), common, f, b, c)
    }

    pub(crate) fn alu_error<F, B, C>(
        &mut self,
        ra: RegisterId,
        f: F,
        b: B,
        c: C,
        err_bool: bool,
    ) -> SimpleResult<()>
    where
        F: FnOnce(B, C) -> Word,
    {
        let (
            SystemRegisters {
                flag, of, err, pc, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let dest = &mut w[ra.try_into()?];
        let common = AluCommonReg { of, err, pc };
        alu_error(dest, flag.as_ref(), common, f, b, c, err_bool)
    }

    pub(crate) fn alu_set(&mut self, ra: RegisterId, b: Word) -> SimpleResult<()> {
        let (SystemRegisters { of, err, pc, .. }, mut w) =
            split_registers(&mut self.registers);
        let dest = &mut w[ra.try_into()?];
        let common = AluCommonReg { of, err, pc };
        alu_set(dest, common, b)
    }

    pub(crate) fn alu_clear(&mut self) -> SimpleResult<()> {
        let (SystemRegisters { of, err, pc, .. }, _) =
            split_registers(&mut self.registers);
        let common = AluCommonReg { of, err, pc };
        alu_clear(common)
    }
}

     if let Ok(expo) = u32::try_from(c) {
         Word::pow(b, expo).unwrap()
     } else if b < 2 {
         (b, false)
     } else {
         (0, true)
     }

pub(crate) struct AluCommonReg<'a> {
    pub of: RegMut<'a, OF>,
    pub err: RegMut<'a, ERR>,
    pub pc: RegMut<'a, PC>,
}

/// Stores the overflowed wrapped value into RegId::OF
pub(crate) fn alu_capture_overflow<F, B, C>(
    dest: &mut Word,
    flag: Reg<FLAG>,
    mut common: AluCommonReg,
    f: F,
    b: B,
    c: C,
) -> SimpleResult<()>
where
    F: FnOnce(B, C) -> (u128, bool),
{
    let (result, _overflow) = f(b, c);

    if result > Word::MAX as u128 && !is_wrapping(flag) {
        return Err(PanicReason::ArithmeticOverflow.into())
    }

    // set the OF register to high bits of the u128 result
    *common.of = (result >> 64) as u64;
    *common.err = 0;

    // set the return value to the low bits of the u128 result
    *dest = u64::try_from(result & Word::MAX as u128)
        .expect("We already truncated the result");

    Ok(inc_pc(common.pc)?)
}

/// Set RegId::OF to true and zero the result register if overflow occurred.
pub(crate) fn alu_boolean_overflow<F, B, C>(
    dest: &mut Word,
    flag: Reg<FLAG>,
    mut common: AluCommonReg,
    f: F,
    b: B,
    c: C,
) -> SimpleResult<()>
where
    F: FnOnce(B, C) -> (Word, bool),
{
    let (result, overflow) = f(b, c);

    if overflow && !is_wrapping(flag) {
        return Err(PanicReason::ArithmeticOverflow.into())
    }

    // set the OF register to 1 if an overflow occurred
    *common.of = overflow as Word;
    *common.err = 0;

    *dest = if overflow { 0 } else { result };

    Ok(inc_pc(common.pc)?)
}

pub(crate) fn alu_error<F, B, C>(
    dest: &mut Word,
    flag: Reg<FLAG>,
    mut common: AluCommonReg,
    f: F,
    b: B,
    c: C,
    err_bool: bool,
) -> SimpleResult<()>
where
    F: FnOnce(B, C) -> Word,
{
    if err_bool && !is_unsafe_math(flag) {
        return Err(PanicReason::ArithmeticError.into())
    }

    *common.of = 0;
    *common.err = err_bool as Word;

    *dest = if err_bool { 0 } else { f(b, c) };

    Ok(inc_pc(common.pc)?)
}

pub(crate) fn alu_set(
    dest: &mut Word,
    mut common: AluCommonReg,
    b: Word,
) -> SimpleResult<()> {
    *common.of = 0;
    *common.err = 0;

    *dest = b;

    Ok(inc_pc(common.pc)?)
}

pub(crate) fn alu_clear(mut common: AluCommonReg) -> SimpleResult<()> {
    *common.of = 0;
    *common.err = 0;

    Ok(inc_pc(common.pc)?)
}
