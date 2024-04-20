#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::vec::Vec;

use super::*;
use core::ops::Div;
use fuel_asm::Imm12;
use test_case::test_case;

use crate::error::PanicOrBug;
#[derive(Debug, PartialEq, Eq)]
struct CommonInput {
    of: Word,
    err: Word,
    pc: Word,
}

#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0, 0, 0 => Ok((0, CommonInput { of: 0, err: 0, pc: 4 }));
    "add 0 0"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0, 1, 1 => Ok((2, CommonInput { of: 0, err: 0, pc: 4 }));
    "add 1 1"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b10, u64::MAX as u128, 1 => Ok((0, CommonInput { of: 1, err: 0, pc: 4 }));
    "add u64::MAX 1 wrapping"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b00, u64::MAX as u128, 1 => Err(PanicOrBug::Panic(PanicReason::ArithmeticOverflow));
    "add u64::MAX 1 not wrapping"
)]
#[test_case(
    CommonInput { of: 0, err: 1, pc: 0 },
    0, 0, 0 => Ok((0, CommonInput { of: 0, err: 0, pc: 4 }));
    "err is cleared"
)]
fn test_add(
    CommonInput {
        mut of,
        mut err,
        mut pc,
    }: CommonInput,
    flag: Word,
    b: u128,
    c: u128,
) -> SimpleResult<(Word, CommonInput)> {
    let common = AluCommonReg {
        of: RegMut::new(&mut of),
        err: RegMut::new(&mut err),
        pc: RegMut::new(&mut pc),
    };
    let mut dest = 0;
    alu_capture_overflow(
        &mut dest,
        Reg::new(&flag),
        common,
        u128::overflowing_add,
        b,
        c,
    )
    .map(|_| (dest, CommonInput { of, err, pc }))
}

#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b0, 1, 1, false => Ok((1, CommonInput { of: 0, err: 0, pc: 4 }));
    "div 1 1"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b0, 10, 2, false => Ok((5, CommonInput { of: 0, err: 0, pc: 4 }));
    "div 10 2"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b0, 10, 0, true => Err(PanicOrBug::Panic(PanicReason::ArithmeticError));
    "div 10 0 error flag"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b1, 10, 0, true => Ok((0, CommonInput { of: 0, err: 1, pc: 4 }));
    "div 10 0 unsafe math"
)]
fn test_div(
    CommonInput {
        mut of,
        mut err,
        mut pc,
    }: CommonInput,
    flag: Word,
    b: u64,
    c: u64,
    err_bool: bool,
) -> SimpleResult<(Word, CommonInput)> {
    let common = AluCommonReg {
        of: RegMut::new(&mut of),
        err: RegMut::new(&mut err),
        pc: RegMut::new(&mut pc),
    };
    let mut dest = 0;
    alu_error(
        &mut dest,
        Reg::new(&flag),
        common,
        Word::div,
        b,
        c,
        err_bool,
    )
    .map(|_| (dest, CommonInput { of, err, pc }))
}

#[test_case(
    CommonInput { of: 1, err: 1, pc: 0 },
    0, 0, 0 => Ok((1, CommonInput { of: 0, err: 0, pc: 4 }));
    "of and err is cleared"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b00, 10, u64::MAX - 100 => Err(PanicOrBug::Panic(PanicReason::ArithmeticOverflow));
    "larger than 32 bit exp"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b00, 10, (u32::MAX as u64)+ 1 => Err(PanicOrBug::Panic(PanicReason::ArithmeticOverflow));
    "just larger than 32 bit exp"
)]
#[test_case(
    CommonInput { of: 0, err: 0, pc: 0 },
    0b10, 10, (u32::MAX as u64) + 3 => Ok((0, CommonInput { of: 1, err: 0, pc: 4 }));
    "just larger than 32 bit exp with wrapping"
)]
fn test_exp(
    CommonInput {
        mut of,
        mut err,
        mut pc,
    }: CommonInput,
    flag: Word,
    b: u64,
    c: u64,
) -> SimpleResult<(Word, CommonInput)> {
    let common = AluCommonReg {
        of: RegMut::new(&mut of),
        err: RegMut::new(&mut err),
        pc: RegMut::new(&mut pc),
    };
    let mut dest = 0;
    alu_boolean_overflow(&mut dest, Reg::new(&flag), common, super::exp, b, c)
        .map(|_| (dest, CommonInput { of, err, pc }))
}

#[test]
fn test_add_can_do_big_int_math() {
    let b: u128 = u64::MAX as u128 + 20;
    let c: u128 = 10;
    let expected = b + c;
    let mut of = 0;
    let mut err = 0;
    let mut pc = 0;
    let flag = 0b10;
    let common = AluCommonReg {
        of: RegMut::new(&mut of),
        err: RegMut::new(&mut err),
        pc: RegMut::new(&mut pc),
    };
    let mut dest = 0;
    alu_capture_overflow(
        &mut dest,
        Reg::new(&flag),
        common,
        u128::overflowing_add,
        b,
        c,
    )
    .unwrap();
    let result = u128::from_be_bytes(
        of.to_be_bytes()
            .into_iter()
            .chain(dest.to_be_bytes())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    );
    assert_eq!(result, expected);
}

#[test]
fn test_alu_set_clears_of_and_err() {
    let mut of = 1;
    let mut err = 1;
    let mut pc = 4;
    let common = AluCommonReg {
        of: RegMut::new(&mut of),
        err: RegMut::new(&mut err),
        pc: RegMut::new(&mut pc),
    };
    let mut dest = 1;
    alu_set(&mut dest, common, 10).unwrap();
    assert_eq!(of, 0);
    assert_eq!(err, 0);
    assert_eq!(pc, 8);
    assert_eq!(dest, 10);
}

#[test]
fn test_word_from_imm_sets_zero() {
    let imm: Imm12 = 10.into();
    let word = Word::from(imm);
    assert_eq!(word.to_be_bytes(), [0, 0, 0, 0, 0, 0, 0, 10]);
}
