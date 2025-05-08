use alloc::{
    vec,
    vec::Vec,
};
use test_case::test_case;

use crate::prelude::*;
use fuel_asm::{
    Flags,
    Instruction,
    RegId,
    Word,
    narrowint::{
        MathArgs,
        MathOp,
        OpWidth,
    },
    op,
};
use fuel_tx::{
    PanicReason,
    Receipt,
};

use super::test_helpers::{
    RunResult,
    run_script,
    set_full_word,
};

fn alu_reserved(registers_init: &[(RegisterId, Word)], ins: Instruction) {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let script = registers_init
        .iter()
        .flat_map(|(r, v)| set_full_word(*r, *v))
        .chain([ins, op::ret(RegId::ONE)].iter().copied())
        .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    let result = receipts
        .iter()
        .find_map(Receipt::reason)
        .map(|r| *r.reason())
        .expect("Expected panic reason");

    assert_eq!(PanicReason::ReservedRegisterNotWritable, result);
}

#[test]
fn reserved_register() {
    alu_reserved(&[(0x10, 128)], op::add(RegId::ZERO, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::ONE, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::OF, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::PC, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::SSP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::SP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::FP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::HP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::ERR, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::GGAS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::CGAS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::BAL, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::IS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::RET, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::RETL, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(RegId::FLAG, 0x10, 0x11));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluOk {
    value: Word,
    of: Word,
    err: Word,
}

impl From<AluOk> for RunResult<AluOk> {
    fn from(a: AluOk) -> Self {
        Self::Success(a)
    }
}

type AluResult = RunResult<AluOk>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AluResultForFlags {
    normal: AluResult,
    wrapping: AluResult,
    unsafemath: AluResult,
    wrapping_unsafemath: AluResult,
}

impl AluResultForFlags {
    /// The operation isn't affected by the flags
    fn invariant_ok(value: Word) -> Self {
        let v = AluOk {
            value,
            of: 0,
            err: 0,
        }
        .into();
        Self {
            normal: v,
            wrapping: v,
            unsafemath: v,
            wrapping_unsafemath: v,
        }
    }

    /// The operation wraps around if wrapping is enabled, otherwise it panics.
    /// Computes the wrapping value for u64 oveflow. For narrow ints, use
    /// `wrapping_ok_narrow`.
    fn wrapping_ok(value: u128) -> Self {
        let wrapped = value as Word;
        let of = (value >> 64) as Word;
        let wrapping = AluOk {
            value: wrapped,
            of,
            err: 0,
        }
        .into();
        Self {
            normal: AluResult::Panic(PanicReason::ArithmeticOverflow),
            wrapping,
            unsafemath: AluResult::Panic(PanicReason::ArithmeticOverflow),
            wrapping_unsafemath: wrapping,
        }
    }

    /// The operation wraps around if wrapping is enabled, otherwise it panics.
    /// Computes the wrapping value for u64 oveflow. For narrow ints, use
    fn wrapping_ok_narrow(wrapped: Word, of: Word) -> Self {
        let wrapping = AluOk {
            value: wrapped,
            of,
            err: 0,
        }
        .into();
        Self {
            normal: AluResult::Panic(PanicReason::ArithmeticOverflow),
            wrapping,
            unsafemath: AluResult::Panic(PanicReason::ArithmeticOverflow),
            wrapping_unsafemath: wrapping,
        }
    }

    /// Like wrapping_ok, but value is 0 and overflow part is 1
    fn wrapping_fixed() -> Self {
        let wrapping = AluOk {
            value: 0,
            of: 1,
            err: 0,
        }
        .into();
        Self {
            normal: AluResult::Panic(PanicReason::ArithmeticOverflow),
            wrapping,
            unsafemath: AluResult::Panic(PanicReason::ArithmeticOverflow),
            wrapping_unsafemath: wrapping,
        }
    }

    /// The operation sets $err on error flag, and panics otherwise
    fn error() -> Self {
        let err = AluOk {
            value: 0,
            of: 0,
            err: 1,
        }
        .into();
        Self {
            normal: AluResult::Panic(PanicReason::ArithmeticError),
            wrapping: AluResult::Panic(PanicReason::ArithmeticError),
            unsafemath: err,
            wrapping_unsafemath: err,
        }
    }
}

fn run_alu_op(op: Instruction, reg_args: Vec<Word>, flags: Flags) -> AluResult {
    let mut code = vec![op::movi(0x10, flags.bits() as u32), op::flag(0x10)];
    for (i, &arg) in reg_args.iter().enumerate() {
        code.extend(set_full_word(0x10 + i, arg));
    }
    code.push(op);
    code.push(op::log(0x20, RegId::OF, RegId::ERR, RegId::ZERO));
    code.push(op::ret(RegId::ZERO));

    let receipts = run_script(code);

    AluResult::extract(&receipts, |receipts| {
        let Some(Receipt::Log { ra, rb, rc, .. }) = receipts.first() else {
            return None;
        };
        Some(AluOk {
            value: *ra,
            of: *rb,
            err: *rc,
        })
    })
}

const M64: u128 = u64::MAX as u128;

#[test_case(op::not, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::not, 0b1101 => AluResultForFlags::invariant_ok(u64::MAX ^ 0b1101))]
#[test_case(op::not, 1 => AluResultForFlags::invariant_ok(u64::MAX - 1))]
fn test_binary_op_reg(op: fn(u8, u8) -> Instruction, a: Word) -> AluResultForFlags {
    let run = |flags| run_alu_op(op(0x20, 0x10), vec![a], flags);
    AluResultForFlags {
        normal: run(Flags::empty()),
        wrapping: run(Flags::WRAPPING),
        unsafemath: run(Flags::UNSAFEMATH),
        wrapping_unsafemath: run(Flags::WRAPPING | Flags::UNSAFEMATH),
    }
}

#[test_case(op::add, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::add, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::add, 0, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::add, u64::MAX, 1 => AluResultForFlags::wrapping_ok(M64 + 1))]
#[test_case(op::add, 1, u64::MAX => AluResultForFlags::wrapping_ok(M64 + 1))]
#[test_case(op::add, u64::MAX, u64::MAX => AluResultForFlags::wrapping_ok(M64 * 2))]
#[test_case(op::sub, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::sub, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::sub, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::sub, 0, 1 => AluResultForFlags::wrapping_ok(0u128.wrapping_sub(1)))]
#[test_case(op::sub, 0, u64::MAX => AluResultForFlags::wrapping_ok(0u128.wrapping_sub(M64)))]
#[test_case(op::mul, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mul, 1, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::mul, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::mul, 1, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::mul, u64::MAX, 2 => AluResultForFlags::wrapping_ok(M64 * 2))]
#[test_case(op::div, 0, 0 => AluResultForFlags::error())]
#[test_case(op::div, 1, 0 => AluResultForFlags::error())]
#[test_case(op::div, u64::MAX, 0 => AluResultForFlags::error())]
#[test_case(op::div, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::div, 1, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::div, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::div, 0, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::div, 1, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::div, 2, 2 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::div, 8, 2 => AluResultForFlags::invariant_ok(4))]
#[test_case(op::div, u64::MAX, 2 => AluResultForFlags::invariant_ok(u64::MAX / 2))]
#[test_case(op::mod_, 0, 0 => AluResultForFlags::error())]
#[test_case(op::mod_, 1, 0 => AluResultForFlags::error())]
#[test_case(op::mod_, u64::MAX, 0 => AluResultForFlags::error())]
#[test_case(op::mod_, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mod_, 1, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mod_, u64::MAX, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mod_, 0, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mod_, 1, 2 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::mod_, 2, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mod_, 4, 3 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::mod_, 5, 3 => AluResultForFlags::invariant_ok(2))]
#[test_case(op::mod_, u64::MAX, 2 => AluResultForFlags::invariant_ok(u64::MAX % 2))]
#[test_case(op::exp, 0, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::exp, 2, 3 => AluResultForFlags::invariant_ok(8))]
#[test_case(op::exp, 3, 3 => AluResultForFlags::invariant_ok(27))]
#[test_case(op::exp, 1, u64::MAX => AluResultForFlags::invariant_ok(1))]
#[test_case(op::exp, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::exp, u32::MAX as u64, 2 => AluResultForFlags::invariant_ok((u32::MAX as u64).pow(2)))]
#[test_case(op::exp, 10, (u32::MAX as u64) + 1 => AluResultForFlags::wrapping_fixed())]
#[test_case(op::exp, u64::MAX, 2 => AluResultForFlags::wrapping_fixed())]
#[test_case(op::mlog, 0, 0 => AluResultForFlags::error())]
#[test_case(op::mlog, 1, 0 => AluResultForFlags::error())]
#[test_case(op::mlog, 1, 1 => AluResultForFlags::error())]
#[test_case(op::mlog, 1, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mlog, 8, 2 => AluResultForFlags::invariant_ok(3))]
#[test_case(op::mlog, u64::MAX, 2 => AluResultForFlags::invariant_ok(63))]
#[test_case(op::mlog, 2, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mlog, 999, 10 => AluResultForFlags::invariant_ok(2))]
#[test_case(op::mlog, 1000, 10 => AluResultForFlags::invariant_ok(3))]
#[test_case(op::mlog, 1001, 10 => AluResultForFlags::invariant_ok(3))]
#[test_case(op::mroo, 0, 0 => AluResultForFlags::error())]
#[test_case(op::mroo, 1, 0 => AluResultForFlags::error())]
#[test_case(op::mroo, 2, 0 => AluResultForFlags::error())]
#[test_case(op::mroo, u64::MAX, 0 => AluResultForFlags::error())]
#[test_case(op::mroo, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mroo, 1, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::mroo, 2, 1 => AluResultForFlags::invariant_ok(2))]
#[test_case(op::mroo, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::mroo, 4, 2 => AluResultForFlags::invariant_ok(2))]
#[test_case(op::mroo, 16, 2 => AluResultForFlags::invariant_ok(4))]
#[test_case(op::mroo, u64::MAX, 2 => AluResultForFlags::invariant_ok(4294967295))]
#[test_case(op::mroo, u64::MAX, 3 => AluResultForFlags::invariant_ok(2642245))]
#[test_case(op::mroo, u64::MAX, 10 => AluResultForFlags::invariant_ok(84))]
#[test_case(op::mroo, u64::MAX, 63 => AluResultForFlags::invariant_ok(2))]
#[test_case(op::sll, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::sll, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::sll, 1, 1 => AluResultForFlags::invariant_ok(1 << 1))]
#[test_case(op::sll, u64::MAX, 4 => AluResultForFlags::invariant_ok(u64::MAX << 4))]
#[test_case(op::sll, u64::MAX, 100 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::sll, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srl, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srl, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::srl, 1, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srl, 0b10, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::srl, u64::MAX, 4 => AluResultForFlags::invariant_ok(u64::MAX >> 4))]
#[test_case(op::srl, u64::MAX, 100 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srl, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::and, 0b1111, 0b1111 => AluResultForFlags::invariant_ok(0b1111))]
#[test_case(op::and, 0b1101, 0b0111 => AluResultForFlags::invariant_ok(0b0101))]
#[test_case(op::and, u64::MAX, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::and, u64::MAX, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::and, 0, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::and, 1, u64::MAX => AluResultForFlags::invariant_ok(1))]
#[test_case(op::and, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::or, 0b1111, 0b1111 => AluResultForFlags::invariant_ok(0b1111))]
#[test_case(op::or, 0b1101, 0b0111 => AluResultForFlags::invariant_ok(0b1111))]
#[test_case(op::or, 0b1101, 0b0101 => AluResultForFlags::invariant_ok(0b1101))]
#[test_case(op::or, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::or, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::or, 0, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::or, 1, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::or, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::xor, 0b1111, 0b1111 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::xor, 0b1101, 0b0111 => AluResultForFlags::invariant_ok(0b1010))]
#[test_case(op::xor, 0b1101, 0b0101 => AluResultForFlags::invariant_ok(0b1000))]
#[test_case(op::xor, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::xor, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX ^ 1))]
#[test_case(op::xor, 0, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::xor, 1, u64::MAX => AluResultForFlags::invariant_ok(u64::MAX ^ 1))]
#[test_case(op::xor, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::eq, 0, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::eq, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::eq, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(1))]
#[test_case(op::eq, u64::MAX, u64::MAX - 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::lt, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::lt, 0, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::lt, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::lt, u64::MAX, u64::MAX - 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::lt, u64::MAX - 1, u64::MAX => AluResultForFlags::invariant_ok(1))]
#[test_case(op::gt, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::gt, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::gt, u64::MAX, u64::MAX => AluResultForFlags::invariant_ok(0))]
#[test_case(op::gt, u64::MAX, u64::MAX - 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::gt, u64::MAX - 1, u64::MAX => AluResultForFlags::invariant_ok(0))]
fn test_binary_op_reg_reg(
    op: fn(u8, u8, u8) -> Instruction,
    lhs: Word,
    rhs: Word,
) -> AluResultForFlags {
    let run = |flags| run_alu_op(op(0x20, 0x10, 0x11), vec![lhs, rhs], flags);
    AluResultForFlags {
        normal: run(Flags::empty()),
        wrapping: run(Flags::WRAPPING),
        unsafemath: run(Flags::UNSAFEMATH),
        wrapping_unsafemath: run(Flags::WRAPPING | Flags::UNSAFEMATH),
    }
}

#[test_case(op::addi, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::addi, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::addi, u64::MAX, 1 => AluResultForFlags::wrapping_ok(M64 + 1))]
#[test_case(op::subi, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::subi, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::subi, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::subi, 0, 1 => AluResultForFlags::wrapping_ok(0u128.wrapping_sub(1)))]
#[test_case(op::muli, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::muli, 1, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::muli, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::muli, u64::MAX, 2 => AluResultForFlags::wrapping_ok(M64 * 2))]
#[test_case(op::divi, 0, 0 => AluResultForFlags::error())]
#[test_case(op::divi, 1, 0 => AluResultForFlags::error())]
#[test_case(op::divi, u64::MAX, 0 => AluResultForFlags::error())]
#[test_case(op::divi, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::divi, 1, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::divi, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::divi, 0, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::divi, 1, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::divi, 2, 2 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::divi, 8, 2 => AluResultForFlags::invariant_ok(4))]
#[test_case(op::divi, u64::MAX, 2 => AluResultForFlags::invariant_ok(u64::MAX / 2))]
#[test_case(op::modi, 0, 0 => AluResultForFlags::error())]
#[test_case(op::modi, 1, 0 => AluResultForFlags::error())]
#[test_case(op::modi, u64::MAX, 0 => AluResultForFlags::error())]
#[test_case(op::modi, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::modi, 1, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::modi, u64::MAX, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::modi, 0, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::modi, 1, 2 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::modi, 2, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::modi, 4, 3 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::modi, 5, 3 => AluResultForFlags::invariant_ok(2))]
#[test_case(op::modi, u64::MAX, 2 => AluResultForFlags::invariant_ok(u64::MAX % 2))]
#[test_case(op::expi, 0, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::expi, 2, 3 => AluResultForFlags::invariant_ok(8))]
#[test_case(op::expi, 3, 3 => AluResultForFlags::invariant_ok(27))]
#[test_case(op::expi, 1, 100 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::expi, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::expi, u32::MAX as u64, 2 => AluResultForFlags::invariant_ok((u32::MAX as u64).pow(2)))]
#[test_case(op::expi, u64::MAX, 2 => AluResultForFlags::wrapping_fixed())]
#[test_case(op::slli, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::slli, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::slli, 1, 1 => AluResultForFlags::invariant_ok(1 << 1))]
#[test_case(op::slli, u64::MAX, 4 => AluResultForFlags::invariant_ok(u64::MAX << 4))]
#[test_case(op::slli, u64::MAX, 100 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srli, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srli, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::srli, 1, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::srli, 0b10, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::srli, u64::MAX, 4 => AluResultForFlags::invariant_ok(u64::MAX >> 4))]
#[test_case(op::srli, u64::MAX, 100 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::andi, 0b1111, 0b1111 => AluResultForFlags::invariant_ok(0b1111))]
#[test_case(op::andi, 0b1101, 0b0111 => AluResultForFlags::invariant_ok(0b0101))]
#[test_case(op::andi, u64::MAX, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::andi, u64::MAX, 1 => AluResultForFlags::invariant_ok(1))]
#[test_case(op::ori, 0b1111, 0b1111 => AluResultForFlags::invariant_ok(0b1111))]
#[test_case(op::ori, 0b1101, 0b0111 => AluResultForFlags::invariant_ok(0b1111))]
#[test_case(op::ori, 0b1101, 0b0101 => AluResultForFlags::invariant_ok(0b1101))]
#[test_case(op::ori, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::ori, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::xori, 0b1111, 0b1111 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::xori, 0b1101, 0b0111 => AluResultForFlags::invariant_ok(0b1010))]
#[test_case(op::xori, 0b1101, 0b0101 => AluResultForFlags::invariant_ok(0b1000))]
#[test_case(op::xori, u64::MAX, 0 => AluResultForFlags::invariant_ok(u64::MAX))]
#[test_case(op::xori, u64::MAX, 1 => AluResultForFlags::invariant_ok(u64::MAX ^ 1))]
fn test_binary_op_reg_imm(
    op: fn(u8, u8, u16) -> Instruction,
    lhs: Word,
    rhs: u16,
) -> AluResultForFlags {
    let run = |flags| run_alu_op(op(0x20, 0x10, rhs), vec![lhs], flags);
    AluResultForFlags {
        normal: run(Flags::empty()),
        wrapping: run(Flags::WRAPPING),
        unsafemath: run(Flags::UNSAFEMATH),
        wrapping_unsafemath: run(Flags::WRAPPING | Flags::UNSAFEMATH),
    }
}

#[test_case(op::mldv, 0, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mldv, 5, 4, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(op::mldv, 3, 2, 1 => AluResultForFlags::invariant_ok(6))]
#[test_case(op::mldv, 5, 4, 2 => AluResultForFlags::invariant_ok(10))]
#[test_case(op::mldv, 5, 5, 3 => AluResultForFlags::invariant_ok(8))]
#[test_case(op::mldv, u64::MAX, 2, 0 => AluResultForFlags::invariant_ok(((M64 * 2) >> 64) as u64))]
#[test_case(op::mldv, u64::MAX, u64::MAX, 0 => AluResultForFlags::invariant_ok(((M64 * M64) >> 64) as u64))]
fn test_binary_op_reg_reg_reg(
    op: fn(u8, u8, u8, u8) -> Instruction,
    a: Word,
    b: Word,
    c: Word,
) -> AluResultForFlags {
    let run = |flags| run_alu_op(op(0x20, 0x10, 0x11, 0x12), vec![a, b, c], flags);
    AluResultForFlags {
        normal: run(Flags::empty()),
        wrapping: run(Flags::WRAPPING),
        unsafemath: run(Flags::UNSAFEMATH),
        wrapping_unsafemath: run(Flags::WRAPPING | Flags::UNSAFEMATH),
    }
}

#[test_case(MathOp::ADD, OpWidth::U8, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::ADD, OpWidth::U8, 1, 1 => AluResultForFlags::invariant_ok(2))]
#[test_case(MathOp::ADD, OpWidth::U8, u64::MAX, 0 => AluResultForFlags::invariant_ok(u8::MAX as u64))]
#[test_case(MathOp::ADD, OpWidth::U16, u64::MAX, 0 => AluResultForFlags::invariant_ok(u16::MAX as u64))]
#[test_case(MathOp::ADD, OpWidth::U32, u64::MAX, 0 => AluResultForFlags::invariant_ok(u32::MAX as u64))]
#[test_case(MathOp::ADD, OpWidth::U8, u8::MAX as u64, 1 => AluResultForFlags::wrapping_ok_narrow(0, 1))]
#[test_case(MathOp::ADD, OpWidth::U16, u16::MAX as u64, 1 => AluResultForFlags::wrapping_ok_narrow(0, 1))]
#[test_case(MathOp::ADD, OpWidth::U32, u32::MAX as u64, 1 => AluResultForFlags::wrapping_ok_narrow(0, 1))]
#[test_case(MathOp::ADD, OpWidth::U8, u8::MAX as u64, 2 => AluResultForFlags::wrapping_ok_narrow(1, 1))]
#[test_case(MathOp::ADD, OpWidth::U16, u8::MAX as u64, 1 => AluResultForFlags::invariant_ok(u8::MAX as u64 + 1))]
#[test_case(MathOp::ADD, OpWidth::U32, u8::MAX as u64, 1 => AluResultForFlags::invariant_ok(u8::MAX as u64 + 1))]
#[test_case(MathOp::ADD, OpWidth::U32, u16::MAX as u64, 1 => AluResultForFlags::invariant_ok(u16::MAX as u64 + 1))]
#[test_case(MathOp::ADD, OpWidth::U32, u32::MAX as u64, u32::MAX as u64 => AluResultForFlags::wrapping_ok_narrow(u32::MAX as u64 - 1, 1))]
#[test_case(MathOp::MUL, OpWidth::U8, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::MUL, OpWidth::U8, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::MUL, OpWidth::U8, 1, 2 => AluResultForFlags::invariant_ok(2))]
#[test_case(MathOp::MUL, OpWidth::U8, 1, u8::MAX as u64 => AluResultForFlags::invariant_ok(u8::MAX as u64))]
#[test_case(MathOp::MUL, OpWidth::U8, u8::MAX as u64, u8::MAX as u64 => AluResultForFlags::wrapping_ok_narrow(1, u8::MAX as u64 - 1))]
#[test_case(MathOp::MUL, OpWidth::U16, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::MUL, OpWidth::U16, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::MUL, OpWidth::U16, 1, 2 => AluResultForFlags::invariant_ok(2))]
#[test_case(MathOp::MUL, OpWidth::U16, 1, u16::MAX as u64 => AluResultForFlags::invariant_ok(u16::MAX as u64))]
#[test_case(MathOp::MUL, OpWidth::U16, u16::MAX as u64, u16::MAX as u64 => AluResultForFlags::wrapping_ok_narrow(1, u16::MAX as u64 - 1))]
#[test_case(MathOp::MUL, OpWidth::U32, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::MUL, OpWidth::U32, 0, 1 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::MUL, OpWidth::U32, 1, 2 => AluResultForFlags::invariant_ok(2))]
#[test_case(MathOp::MUL, OpWidth::U32, 1, u32::MAX as u64 => AluResultForFlags::invariant_ok(u32::MAX as u64))]
#[test_case(MathOp::MUL, OpWidth::U32, u32::MAX as u64, u32::MAX as u64 => AluResultForFlags::wrapping_ok_narrow(1, u32::MAX as u64 - 1))]
#[test_case(MathOp::EXP, OpWidth::U8, 0, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(MathOp::EXP, OpWidth::U8, 0, 2 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::EXP, OpWidth::U8, 2, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(MathOp::EXP, OpWidth::U8, 2, 3 => AluResultForFlags::invariant_ok(8))]
#[test_case(MathOp::EXP, OpWidth::U8, 2, 10 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::EXP, OpWidth::U8, 99, 99 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::EXP, OpWidth::U8, u8::MAX as u64, u8::MAX as u64 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::EXP, OpWidth::U16, 2, 10 => AluResultForFlags::invariant_ok(2u64.pow(10)))]
#[test_case(MathOp::EXP, OpWidth::U16, 2, 20 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::EXP, OpWidth::U16, u16::MAX as u64, u16::MAX as u64 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::EXP, OpWidth::U32, 2, 20 => AluResultForFlags::invariant_ok(2u64.pow(20)))]
#[test_case(MathOp::EXP, OpWidth::U32, 2, 40 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::EXP, OpWidth::U32, u32::MAX as u64, u32::MAX as u64 => AluResultForFlags::wrapping_fixed())]
#[test_case(MathOp::SLL, OpWidth::U8, 0, 0 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::SLL, OpWidth::U8, 1, 0 => AluResultForFlags::invariant_ok(1))]
#[test_case(MathOp::SLL, OpWidth::U8, 1, 3 => AluResultForFlags::invariant_ok(0b1000))]
#[test_case(MathOp::SLL, OpWidth::U8, 1, 10 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::SLL, OpWidth::U8, 1, 100 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::SLL, OpWidth::U8, 1, u8::MAX as u64 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::SLL, OpWidth::U16, 1, 15 => AluResultForFlags::invariant_ok(1 << 15))]
#[test_case(MathOp::SLL, OpWidth::U16, 1, u16::MAX as u64 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::SLL, OpWidth::U32, 1, 15 => AluResultForFlags::invariant_ok(1 << 15))]
#[test_case(MathOp::SLL, OpWidth::U32, 1, 31 => AluResultForFlags::invariant_ok(1 << 31))]
#[test_case(MathOp::SLL, OpWidth::U32, 1, 32 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::SLL, OpWidth::U32, 1, u32::MAX as u64 => AluResultForFlags::invariant_ok(0))]
#[test_case(MathOp::XNOR, OpWidth::U8, 0, 0 => AluResultForFlags::invariant_ok(u8::MAX as u64))]
#[test_case(MathOp::XNOR, OpWidth::U16, 0, 0 => AluResultForFlags::invariant_ok(u16::MAX as u64))]
#[test_case(MathOp::XNOR, OpWidth::U32, 0, 0 => AluResultForFlags::invariant_ok(u32::MAX as u64))]
#[test_case(MathOp::XNOR, OpWidth::U8, 0b_1111_0000, 1 => AluResultForFlags::invariant_ok(0b_0000_1110))]
#[test_case(MathOp::XNOR, OpWidth::U32, u32::MAX as u64, u32::MAX as u64 => AluResultForFlags::invariant_ok(u32::MAX as u64))]
fn test_niop(op: MathOp, width: OpWidth, lhs: Word, rhs: Word) -> AluResultForFlags {
    let args = MathArgs { op, width };
    let run = |flags| {
        run_alu_op(
            op::niop(0x20, 0x10, 0x11, args.to_imm().into()),
            vec![lhs, rhs],
            flags,
        )
    };
    AluResultForFlags {
        normal: run(Flags::empty()),
        wrapping: run(Flags::WRAPPING),
        unsafemath: run(Flags::UNSAFEMATH),
        wrapping_unsafemath: run(Flags::WRAPPING | Flags::UNSAFEMATH),
    }
}
