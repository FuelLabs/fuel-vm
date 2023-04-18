use ethnum::U256;

use fuel_asm::{
    op,
    widemath::{CompareArgs, CompareMode, DivArgs, MathArgs, MathOp, MulArgs},
    Flags, Instruction, PanicReason, RegId,
};
use fuel_tx::Receipt;

use super::test_helpers::run_script;

/// Allocates a byte array from heap and initializes it. Then points `reg` to it.
fn aloc_bytearray<const S: usize>(reg: u8, v: [u8; S]) -> Vec<Instruction> {
    let mut ops = vec![op::movi(reg, S as u32), op::aloc(reg)];
    for (i, b) in v.iter().enumerate() {
        if *b != 0 {
            ops.push(op::movi(reg, *b as u32));
            ops.push(op::sb(RegId::HP, reg, i as u16));
        }
    }
    ops.push(op::move_(reg, RegId::HP));
    ops
}

fn make_u128(reg: u8, v: u128) -> Vec<Instruction> {
    aloc_bytearray(reg, v.to_be_bytes())
}

fn make_u256(reg: u8, v: U256) -> Vec<Instruction> {
    aloc_bytearray(reg, v.to_be_bytes())
}

#[rstest::rstest]
fn cmp_u128(
    #[values(0, 1, 2, u64::MAX as u128, (u64::MAX as u128) + 1, u128::MAX)] a: u128,
    #[values(0, 1, 2, u64::MAX as u128, (u64::MAX as u128) + 1, u128::MAX)] b: u128,
    #[values(
        CompareMode::EQ,
        CompareMode::NE,
        CompareMode::LT,
        CompareMode::GT,
        CompareMode::LTE,
        CompareMode::GTE
    )]
    mode: CompareMode,
) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, a));
    ops.extend(make_u128(0x21, b));
    ops.push(op::wdcm_args(
        0x22,
        0x20,
        0x21,
        CompareArgs {
            indirect_rhs: true,
            mode,
        },
    ));
    ops.push(op::log(0x22, RegId::ZERO, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Log { ra, .. } = receipts.first().unwrap() {
        let expected = match mode {
            CompareMode::EQ => a == b,
            CompareMode::NE => a != b,
            CompareMode::LT => a < b,
            CompareMode::GT => a > b,
            CompareMode::LTE => a <= b,
            CompareMode::GTE => a >= b,
        };
        assert_eq!(*ra, expected as u64);
    } else {
        panic!("Expected log receipt");
    }
}

#[rstest::rstest]
fn cmp_u256(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into(), ((u64::MAX as u128) + 1).into(), u128::MAX.into())]
    a: U256,
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into(), ((u64::MAX as u128) + 1).into(), u128::MAX.into())]
    b: U256,
    #[values(
        CompareMode::EQ,
        CompareMode::NE,
        CompareMode::LT,
        CompareMode::GT,
        CompareMode::LTE,
        CompareMode::GTE
    )]
    mode: CompareMode,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, a));
    ops.extend(make_u256(0x21, b));
    ops.push(op::wqcm_args(
        0x22,
        0x20,
        0x21,
        CompareArgs {
            indirect_rhs: true,
            mode,
        },
    ));
    ops.push(op::log(0x22, RegId::ZERO, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Log { ra, .. } = receipts.first().unwrap() {
        let expected = match mode {
            CompareMode::EQ => a == b,
            CompareMode::NE => a != b,
            CompareMode::LT => a < b,
            CompareMode::GT => a > b,
            CompareMode::LTE => a <= b,
            CompareMode::GTE => a >= b,
        };
        assert_eq!(*ra, expected as u64);
    } else {
        panic!("Expected log receipt");
    }
}

#[rstest::rstest]
fn incr_u128(#[values(0, 1, 2, u64::MAX as u128, (u64::MAX as u128) + 1, u128::MAX - 1)] v: u128) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, v));
    ops.extend(make_u128(0x22, 0));
    ops.push(op::wdop_args(
        0x22,
        0x20,
        RegId::ONE,
        MathArgs {
            indirect_rhs: false,
            op: MathOp::ADD,
        },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let expected = v + 1;
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn incr_u256(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into(), ((u64::MAX as u128) + 1).into(), u128::MAX.into())]
    v: U256,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, v));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqop_args(
        0x22,
        0x20,
        RegId::ONE,
        MathArgs {
            indirect_rhs: false,
            op: MathOp::ADD,
        },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let expected = v + 1;
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn incr_overflow_u128() {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, u128::MAX));
    ops.extend(make_u128(0x22, 0));
    ops.push(op::wdop_args(
        0x22,
        0x20,
        RegId::ONE,
        MathArgs {
            indirect_rhs: false,
            op: MathOp::ADD,
        },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Panic { reason, .. } = receipts.first().unwrap() {
        assert_eq!(*reason.reason(), PanicReason::ArithmeticOverflow);
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn incr_overflow_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, U256::MAX));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wdop_args(
        0x22,
        0x20,
        RegId::ONE,
        MathArgs {
            indirect_rhs: false,
            op: MathOp::ADD,
        },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Panic { reason, .. } = receipts.first().unwrap() {
        assert_eq!(*reason.reason(), PanicReason::ArithmeticOverflow);
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn incr_wrapping_u128() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u128(0x20, u128::MAX));
    ops.extend(make_u128(0x22, 0));
    ops.push(op::wdop_args(
        0x22,
        0x20,
        RegId::ONE,
        MathArgs {
            indirect_rhs: false,
            op: MathOp::ADD,
        },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, 0);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn incr_wrapping_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, U256::MAX));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wdop_args(
        0x22,
        0x20,
        RegId::ONE,
        MathArgs {
            indirect_rhs: false,
            op: MathOp::ADD,
        },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, 0);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn multiply_overflow_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, U256::MAX));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqml_args(
        0x22,
        0x20,
        0x20,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: true,
        },
    ));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Panic { reason, .. } = receipts.first().unwrap() {
        assert_eq!(*reason.reason(), PanicReason::ArithmeticOverflow);
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn multiply_overflow_wrapping_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, U256::MAX));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqml_args(
        0x22,
        0x20,
        0x20,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: true,
        },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, U256::MAX.wrapping_mul(U256::MAX));
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn multiply_ok_u256(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into(), u128::MAX.into())] a: U256,
    #[values(1u64.into(), 2u64.into(), u64::MAX.into(), u128::MAX.into())] b: U256,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, a));
    ops.extend(make_u256(0x21, b));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: true,
        },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, a * b);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn divide_by_zero_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqdv_args(0x22, 0x20, 0x22, DivArgs { indirect_rhs: true }));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Panic { reason, .. } = receipts.first().unwrap() {
        assert_eq!(*reason.reason(), PanicReason::ErrorFlag);
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn divide_by_zero_unsafemath_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqdv_args(0x22, 0x20, 0x22, DivArgs { indirect_rhs: true }));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, 0);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn divide_ok_u256(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into())] a: U256,
    #[values(1u64.into(), 2u64.into(), u64::MAX.into())] b: U256,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, a));
    ops.extend(make_u256(0x21, b));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqdv_args(0x22, 0x20, 0x21, DivArgs { indirect_rhs: true }));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, a / b);
    } else {
        panic!("Expected logd receipt");
    }
}
