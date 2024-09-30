use alloc::{
    vec,
    vec::Vec,
};

use ethnum::U256;

use fuel_asm::{
    op,
    wideint::{
        CompareArgs,
        CompareMode,
        DivArgs,
        MathArgs,
        MathOp,
        MulArgs,
    },
    Flags,
    Instruction,
    PanicReason,
    RegId,
};
use fuel_tx::Receipt;

use super::test_helpers::{
    assert_panics,
    run_script,
};

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
        CompareMode::GTE,
        CompareMode::LZC
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
            CompareMode::EQ => (a == b) as u64,
            CompareMode::NE => (a != b) as u64,
            CompareMode::LT => (a < b) as u64,
            CompareMode::GT => (a > b) as u64,
            CompareMode::LTE => (a <= b) as u64,
            CompareMode::GTE => (a >= b) as u64,
            CompareMode::LZC => a.leading_zeros() as u64,
        };
        assert_eq!(*ra, expected);
    } else {
        panic!("Expected log receipt");
    }
}

#[test]
fn cmp_u128_resets_of() {
    // Given
    // Issue an overflowing operation first and log the value of $of
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.push(op::not(0x20, 0));
    ops.push(op::mul(0x20, 0x20, 0x20));
    ops.push(op::log(RegId::OF, RegId::ZERO, RegId::ZERO, RegId::ZERO));

    // Now push a cmp_u128 operation and log the value of $of again
    ops.extend(make_u128(0x20, 0));
    ops.extend(make_u128(0x21, 0));
    ops.push(op::wdcm_args(
        0x22,
        0x20,
        0x21,
        CompareArgs {
            indirect_rhs: true,
            mode: CompareMode::EQ,
        },
    ));
    ops.push(op::log(RegId::OF, RegId::ZERO, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));

    // When
    let receipts: Vec<Receipt> = run_script(ops);

    let Receipt::Log {
        ra: reg_of_before_cmp,
        ..
    } = receipts.first().unwrap()
    else {
        panic!("Expected log receipt");
    };

    let Receipt::Log {
        ra: reg_of_after_cmp,
        ..
    } = receipts.get(1).unwrap()
    else {
        panic!("Expected log receipt");
    };

    // Then
    assert!(*reg_of_before_cmp != 0);
    assert_eq!(*reg_of_after_cmp, 0);
}

#[test]
fn cmp_u128_resets_err() {
    // Given
    // Issue an erroring operation first and log the value of $err
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.push(op::div(0x10, RegId::ONE, RegId::ZERO));
    ops.push(op::log(RegId::ERR, RegId::ZERO, RegId::ZERO, RegId::ZERO));

    // Now push a cmp_u128 operation and log the value of $err again
    ops.extend(make_u128(0x20, 0));
    ops.extend(make_u128(0x21, 0));
    ops.push(op::wdcm_args(
        0x22,
        0x20,
        0x21,
        CompareArgs {
            indirect_rhs: true,
            mode: CompareMode::EQ,
        },
    ));
    ops.push(op::log(RegId::ERR, RegId::ZERO, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));

    // When
    let receipts: Vec<Receipt> = run_script(ops);

    let Receipt::Log {
        ra: reg_err_before_cmp,
        ..
    } = receipts.first().unwrap()
    else {
        panic!("Expected log receipt");
    };

    let Receipt::Log {
        ra: reg_err_after_cmp,
        ..
    } = receipts.get(1).unwrap()
    else {
        panic!("Expected log receipt");
    };

    // Then
    assert_eq!(*reg_err_before_cmp, 1);
    assert_eq!(*reg_err_after_cmp, 0);
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
        CompareMode::GTE,
        CompareMode::LZC
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

    let receipts: Vec<Receipt> = run_script(ops);

    if let Receipt::Log { ra, .. } = receipts.first().unwrap() {
        let expected = match mode {
            CompareMode::EQ => (a == b) as u64,
            CompareMode::NE => (a != b) as u64,
            CompareMode::LT => (a < b) as u64,
            CompareMode::GT => (a > b) as u64,
            CompareMode::LTE => (a <= b) as u64,
            CompareMode::GTE => (a >= b) as u64,
            CompareMode::LZC => a.leading_zeros() as u64,
        };
        assert_eq!(*ra, expected);
    } else {
        panic!("Expected log receipt");
    }
}

#[test]
fn cmp_u256_resets_of() {
    // Given
    // Issue an overflowing operation first and log the value of $of
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.push(op::not(0x20, 0));
    ops.push(op::mul(0x20, 0x20, 0x20));
    ops.push(op::log(RegId::OF, RegId::ZERO, RegId::ZERO, RegId::ZERO));

    // Now push a cmp_u256 operation and log the value of $of again
    ops.extend(make_u256(0x20, 0u64.into()));
    ops.extend(make_u256(0x21, 0u64.into()));
    ops.push(op::wqcm_args(
        0x22,
        0x20,
        0x21,
        CompareArgs {
            indirect_rhs: true,
            mode: CompareMode::EQ,
        },
    ));
    ops.push(op::log(RegId::OF, RegId::ZERO, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));

    // When
    let receipts: Vec<Receipt> = run_script(ops);

    let Receipt::Log {
        ra: reg_of_before_cmp,
        ..
    } = receipts.first().unwrap()
    else {
        panic!("Expected log receipt");
    };

    let Receipt::Log {
        ra: reg_of_after_cmp,
        ..
    } = receipts.get(1).unwrap()
    else {
        panic!("Expected log receipt");
    };

    // Then
    assert!(*reg_of_before_cmp != 0);
    assert_eq!(*reg_of_after_cmp, 0);
}

#[test]
fn cmp_u256_resets_err() {
    // Given
    // Issue an erroring operation first and log the value of $err
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.push(op::div(0x10, RegId::ONE, RegId::ZERO));
    ops.push(op::log(RegId::ERR, RegId::ZERO, RegId::ZERO, RegId::ZERO));

    // Now push a cmp_u256 operation and log the value of $err again
    ops.extend(make_u256(0x20, 0u64.into()));
    ops.extend(make_u256(0x21, 0u64.into()));
    ops.push(op::wqcm_args(
        0x22,
        0x20,
        0x21,
        CompareArgs {
            indirect_rhs: true,
            mode: CompareMode::EQ,
        },
    ));
    ops.push(op::log(RegId::ERR, RegId::ZERO, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));

    // When
    let receipts: Vec<Receipt> = run_script(ops);

    let Receipt::Log {
        ra: reg_err_before_cmp,
        ..
    } = receipts.first().unwrap()
    else {
        panic!("Expected log receipt");
    };

    let Receipt::Log {
        ra: reg_err_after_cmp,
        ..
    } = receipts.get(1).unwrap()
    else {
        panic!("Expected log receipt");
    };

    // Then
    assert_eq!(*reg_err_before_cmp, 1);
    assert_eq!(*reg_err_after_cmp, 0);
}

#[rstest::rstest]
fn incr_u128(
    #[values(0, 1, 2, u64::MAX as u128, (u64::MAX as u128) + 1, u128::MAX - 1)] v: u128,
) {
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
        let data = data.as_ref().unwrap();
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
        let data = data.as_ref().unwrap();
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
        let data = data.as_ref().unwrap();
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
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, 0);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn multiply_overflow_u128() {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, u128::MAX));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wdml_args(
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
fn multiply_overflow_wrapping_u128() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u128(0x20, u128::MAX));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wdml_args(
        0x22,
        0x20,
        0x20,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: true,
        },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, u128::MAX.wrapping_mul(u128::MAX));
    } else {
        panic!("Expected logd receipt");
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
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, U256::MAX.wrapping_mul(U256::MAX));
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn multiply_ok_u128(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into())] a: u128,
    #[values(1u64.into(), 2u64.into(), u64::MAX.into())] b: u128,
) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, a));
    ops.extend(make_u128(0x21, b));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wdml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: true,
        },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, a * b);
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
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, a * b);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn multiply_single_indirect_u128(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into())] a: u128,
    #[values(0, 1, 2, 5, 7)] b: u32,
) {
    let mut ops_lhs = Vec::new();
    ops_lhs.extend(make_u128(0x20, a));
    ops_lhs.push(op::movi(0x21, b));
    ops_lhs.extend(make_u128(0x22, 0u64.into()));
    ops_lhs.push(op::wdml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: false,
        },
    ));
    ops_lhs.push(op::movi(0x23, 16));
    ops_lhs.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops_lhs.push(op::ret(RegId::ONE));

    let mut ops_rhs = Vec::new();
    ops_rhs.push(op::movi(0x20, b));
    ops_rhs.extend(make_u128(0x21, a));
    ops_rhs.extend(make_u128(0x22, 0u64.into()));
    ops_rhs.push(op::wdml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: false,
            indirect_rhs: true,
        },
    ));
    ops_rhs.push(op::movi(0x23, 16));
    ops_rhs.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops_rhs.push(op::ret(RegId::ONE));

    let lhs_receipts = run_script(ops_lhs);
    let rhs_receipts = run_script(ops_rhs);

    let expected = a * u128::from(b);

    if let Receipt::LogData { data, .. } = lhs_receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, expected);
    } else {
        panic!("Expected logd receipt");
    }

    if let Receipt::LogData { data, .. } = rhs_receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn multiply_single_indirect_u256(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into(), u128::MAX.into())] a: U256,
    #[values(0, 1, 2, 5, 7)] b: u32,
) {
    let mut ops_lhs = Vec::new();
    ops_lhs.extend(make_u256(0x20, a));
    ops_lhs.push(op::movi(0x21, b));
    ops_lhs.extend(make_u256(0x22, 0u64.into()));
    ops_lhs.push(op::wqml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: true,
            indirect_rhs: false,
        },
    ));
    ops_lhs.push(op::movi(0x23, 32));
    ops_lhs.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops_lhs.push(op::ret(RegId::ONE));

    let mut ops_rhs = Vec::new();
    ops_rhs.push(op::movi(0x20, b));
    ops_rhs.extend(make_u256(0x21, a));
    ops_rhs.extend(make_u256(0x22, 0u64.into()));
    ops_rhs.push(op::wqml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: false,
            indirect_rhs: true,
        },
    ));
    ops_rhs.push(op::movi(0x23, 32));
    ops_rhs.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops_rhs.push(op::ret(RegId::ONE));

    let lhs_receipts = run_script(ops_lhs);
    let rhs_receipts = run_script(ops_rhs);

    let expected = a * U256::from(b);

    if let Receipt::LogData { data, .. } = lhs_receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, expected);
    } else {
        panic!("Expected logd receipt");
    }

    if let Receipt::LogData { data, .. } = rhs_receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn multiply_two_directs_u128(
    #[values(0, 1, 2, 5, 7)] a: u32,
    #[values(0, 1, 2, 5, 7)] b: u32,
) {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, a));
    ops.push(op::movi(0x21, b));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wdml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: false,
            indirect_rhs: false,
        },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, u128::from(a * b));
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn multiply_two_directs_u256(
    #[values(0, 1, 2, 5, 7)] a: u32,
    #[values(0, 1, 2, 5, 7)] b: u32,
) {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, a));
    ops.push(op::movi(0x21, b));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqml_args(
        0x22,
        0x20,
        0x21,
        MulArgs {
            indirect_lhs: false,
            indirect_rhs: false,
        },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, U256::from(a * b));
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn divide_by_zero_u128() {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, 1u64.into()));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wddv_args(
        0x22,
        0x20,
        0x22,
        DivArgs { indirect_rhs: true },
    ));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Panic { reason, .. } = receipts.first().unwrap() {
        assert_eq!(*reason.reason(), PanicReason::ArithmeticError);
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn divide_by_zero_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqdv_args(
        0x22,
        0x20,
        0x22,
        DivArgs { indirect_rhs: true },
    ));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Panic { reason, .. } = receipts.first().unwrap() {
        assert_eq!(*reason.reason(), PanicReason::ArithmeticError);
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn divide_by_zero_unsafemath_u128() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u128(0x20, 1u64.into()));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wddv_args(
        0x22,
        0x20,
        0x22,
        DivArgs { indirect_rhs: true },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, 0);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn divide_by_zero_unsafemath_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x22, 0u64.into()));
    ops.push(op::wqdv_args(
        0x22,
        0x20,
        0x22,
        DivArgs { indirect_rhs: true },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, 0);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
fn divide_ok_u128(
    #[values(0u64.into(), 1u64.into(), 2u64.into(), u64::MAX.into())] a: u128,
    #[values(1u64.into(), 2u64.into(), u64::MAX.into())] b: u128,
) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, a));
    ops.extend(make_u128(0x21, b));
    ops.extend(make_u128(0x22, 0u64.into()));
    ops.push(op::wddv_args(
        0x22,
        0x20,
        0x21,
        DivArgs { indirect_rhs: true },
    ));
    ops.push(op::movi(0x23, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    let recp = receipts.first().unwrap();

    if let &Receipt::LogData { data, .. } = &recp {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let result = u128::from_be_bytes(bytes);
        assert_eq!(result, a / b);
    } else {
        panic!("Expected logd receipt, found {:?}", recp);
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
    ops.push(op::wqdv_args(
        0x22,
        0x20,
        0x21,
        DivArgs { indirect_rhs: true },
    ));
    ops.push(op::movi(0x23, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x22, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let result = U256::from_be_bytes(bytes);
        assert_eq!(result, a / b);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
#[case(0u64.into(), 0u64.into(), 1u64.into(), 0u64.into())]
#[case(0u64.into(), 5u64.into(), 1u64.into(), 0u64.into())]
#[case(9u64.into(), 9u64.into(), 1u64.into(), 81u64.into())]
#[case(9u64.into(), 9u64.into(), 2u64.into(), 40u64.into())]
#[case(9u64.into(), 9u64.into(), 3u64.into(), 27u64.into())]
#[case(9u64.into(), 9u64.into(), 4u64.into(), 20u64.into())]
#[case(u128::MAX, 5u64.into(), 10u64.into(), u128::MAX / 2)]
#[case(u128::MAX, 2u64.into(), 6u64.into(), u128::MAX / 3)]
#[case(u128::MAX, u128::MAX, u128::MAX, u128::MAX)]
fn fused_mul_div_u128(
    #[case] lhs: u128,
    #[case] rhs: u128,
    #[case] divisor: u128,
    #[case] expected: u128,
) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, lhs));
    ops.extend(make_u128(0x21, rhs));
    ops.extend(make_u128(0x22, divisor));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdmd(0x23, 0x20, 0x21, 0x22));
    ops.push(op::movi(0x24, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x23, 0x24));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let v = u128::from_be_bytes(bytes);
        assert_eq!(v, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn fused_mul_div_overflow_u128() {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, u128::MAX));
    ops.extend(make_u128(0x21, 3u64.into()));
    ops.extend(make_u128(0x22, 2u64.into()));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdmd(0x23, 0x20, 0x21, 0x22));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::ArithmeticOverflow);
}

#[test]
fn fused_mul_div_overflow_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, U256::MAX));
    ops.extend(make_u256(0x21, 3u64.into()));
    ops.extend(make_u256(0x22, 2u64.into()));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqmd(0x23, 0x20, 0x21, 0x22));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::ArithmeticOverflow);
}

#[test]
fn fused_mul_div_overflow_wrapping_u128() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u128(0x20, u128::MAX));
    ops.extend(make_u128(0x21, 3u64.into()));
    ops.extend(make_u128(0x22, 2u64.into()));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdmd(0x23, 0x20, 0x21, 0x22));
    ops.push(op::log(RegId::OF, 0x00, 0x00, 0x00));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Log { ra, .. } = receipts.first().unwrap() {
        assert_eq!(*ra, 1);
    } else {
        panic!("Expected log receipt");
    }
}

#[test]
fn fused_mul_div_overflow_wrapping_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::WRAPPING.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, U256::MAX));
    ops.extend(make_u256(0x21, 3u64.into()));
    ops.extend(make_u256(0x22, 2u64.into()));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqmd(0x23, 0x20, 0x21, 0x22));
    ops.push(op::log(RegId::OF, 0x00, 0x00, 0x00));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::Log { ra, .. } = receipts.first().unwrap() {
        assert_eq!(*ra, 1);
    } else {
        panic!("Expected log receipt");
    }
}

#[rstest::rstest]
#[case(0u64.into(), 0u64.into(), 1u64.into(), 0u64.into())]
#[case(0u64.into(), 5u64.into(), 1u64.into(), 0u64.into())]
#[case(9u64.into(), 9u64.into(), 1u64.into(), 81u64.into())]
#[case(9u64.into(), 9u64.into(), 2u64.into(), 40u64.into())]
#[case(9u64.into(), 9u64.into(), 3u64.into(), 27u64.into())]
#[case(9u64.into(), 9u64.into(), 4u64.into(), 20u64.into())]
#[case(U256::MAX, 5u64.into(), 10u64.into(), U256::MAX / 2)]
#[case(U256::MAX, 2u64.into(), 6u64.into(), U256::MAX / 3)]
#[case(U256::MAX, U256::MAX, U256::MAX, U256::MAX)]
#[case(U256::MAX, u128::MAX.into(), 0u64.into(), 340282366920938463463374607431768211454u128.into())]
fn fused_mul_div_u256(
    #[case] lhs: U256,
    #[case] rhs: U256,
    #[case] divisor: U256,
    #[case] expected: U256,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, lhs));
    ops.extend(make_u256(0x21, rhs));
    ops.extend(make_u256(0x22, divisor));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqmd(0x23, 0x20, 0x21, 0x22));
    ops.push(op::movi(0x24, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x23, 0x24));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let v = U256::from_be_bytes(bytes);
        assert_eq!(v, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn addmod_by_zero_u128() {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, 1u64.into()));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdam(0x23, 0x20, 0x20, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::ArithmeticError);
}

#[test]
fn addmod_by_zero_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqam(0x23, 0x20, 0x20, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::ArithmeticError);
}

#[test]
fn addmod_by_zero_unsafemath_u128() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u128(0x20, 1u64.into()));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdam(0x23, 0x20, 0x20, 0x23));
    ops.push(op::log(RegId::OF, RegId::ERR, 0x00, 0x00));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    if let Receipt::Log { ra, rb, .. } = receipts.first().unwrap() {
        assert_eq!(*ra, 0); // of
        assert_eq!(*rb, 1); // err
    } else {
        panic!("Expected log receipt");
    }
}

#[test]
fn addmod_by_zero_unsafemath_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqam(0x23, 0x20, 0x20, 0x23));
    ops.push(op::log(RegId::OF, RegId::ERR, 0x00, 0x00));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    if let Receipt::Log { ra, rb, .. } = receipts.first().unwrap() {
        assert_eq!(*ra, 0); // of
        assert_eq!(*rb, 1); // err
    } else {
        panic!("Expected log receipt");
    }
}

#[rstest::rstest]
#[case(99u64.into(), 99u64.into(), 1u64.into(), 0u64.into())]
#[case(0u64.into(), 0u64.into(), 100u64.into(), 0u64.into())]
#[case(1u64.into(), 0u64.into(), 100u64.into(), 1u64.into())]
#[case(1u64.into(), 1u64.into(), 100u64.into(), 2u64.into())]
#[case(99u64.into(), 1u64.into(), 100u64.into(), 0u64.into())]
#[case(99u64.into(), 2u64.into(), 100u64.into(), 1u64.into())]
#[case(99u64.into(), 99u64.into(), 100u64.into(), 98u64.into())]
#[case(u128::MAX, u128::MAX, 7u64.into(), 6u64.into())]
#[case(u128::MAX, 2u64.into(), u128::MAX, 2u64.into())]
fn addmod_u128(
    #[case] lhs: u128,
    #[case] rhs: u128,
    #[case] modulus: u128,
    #[case] expected: u128,
) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, lhs));
    ops.extend(make_u128(0x21, rhs));
    ops.extend(make_u128(0x22, modulus));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdam(0x23, 0x20, 0x21, 0x22));
    ops.push(op::movi(0x24, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x23, 0x24));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let v = u128::from_be_bytes(bytes);
        assert_eq!(v, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
#[case(99u64.into(), 99u64.into(), 1u64.into(), 0u64.into())]
#[case(0u64.into(), 0u64.into(), 100u64.into(), 0u64.into())]
#[case(1u64.into(), 0u64.into(), 100u64.into(), 1u64.into())]
#[case(1u64.into(), 1u64.into(), 100u64.into(), 2u64.into())]
#[case(99u64.into(), 1u64.into(), 100u64.into(), 0u64.into())]
#[case(99u64.into(), 2u64.into(), 100u64.into(), 1u64.into())]
#[case(99u64.into(), 99u64.into(), 100u64.into(), 98u64.into())]
#[case(U256::MAX, U256::MAX, 7u64.into(), 2u64.into())]
#[case(5u64.into(), 2u64.into(), 5u64.into(), 2u64.into())]
#[case(U256::MAX, 2u64.into(), U256::MAX, 2u64.into())]
fn addmod_u256(
    #[case] lhs: U256,
    #[case] rhs: U256,
    #[case] modulus: U256,
    #[case] expected: U256,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, lhs));
    ops.extend(make_u256(0x21, rhs));
    ops.extend(make_u256(0x22, modulus));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqam(0x23, 0x20, 0x21, 0x22));
    ops.push(op::movi(0x24, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x23, 0x24));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let v = U256::from_be_bytes(bytes);
        assert_eq!(v, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[test]
fn mulmod_by_zero_u128() {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, 1u64.into()));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdmm(0x23, 0x20, 0x20, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::ArithmeticError);
}

#[test]
fn mulmod_by_zero_u256() {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqmm(0x23, 0x20, 0x20, 0x23));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::ArithmeticError);
}

#[test]
fn mulmod_by_zero_unsafemath_u128() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u128(0x20, 1u64.into()));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdmm(0x23, 0x20, 0x20, 0x23));
    ops.push(op::log(RegId::OF, RegId::ERR, 0x00, 0x00));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    if let Receipt::Log { ra, rb, .. } = receipts.first().unwrap() {
        assert_eq!(*ra, 0); // of
        assert_eq!(*rb, 1); // err
    } else {
        panic!("Expected log receipt");
    }
}

#[test]
fn mulmod_by_zero_unsafemath_u256() {
    let mut ops = Vec::new();
    ops.push(op::movi(0x20, Flags::UNSAFEMATH.bits() as u32));
    ops.push(op::flag(0x20));
    ops.extend(make_u256(0x20, 1u64.into()));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqmm(0x23, 0x20, 0x20, 0x23));
    ops.push(op::log(RegId::OF, RegId::ERR, 0x00, 0x00));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);
    if let Receipt::Log { ra, rb, .. } = receipts.first().unwrap() {
        assert_eq!(*ra, 0); // of
        assert_eq!(*rb, 1); // err
    } else {
        panic!("Expected log receipt");
    }
}

#[rstest::rstest]
#[case(99u64.into(), 99u64.into(), 1u64.into(), 0u64.into())]
#[case(0u64.into(), 0u64.into(), 100u64.into(), 0u64.into())]
#[case(1u64.into(), 1u64.into(), 100u64.into(), 1u64.into())]
#[case(1u64.into(), 2u64.into(), 100u64.into(), 2u64.into())]
#[case(50u64.into(), 2u64.into(), 100u64.into(), 0u64.into())]
#[case(50u64.into(), 3u64.into(), 100u64.into(), 50u64.into())]
#[case(99u64.into(), 99u64.into(), 100u64.into(), 1u64.into())]
#[case(1234u64.into(), 5678u64.into(), 100u64.into(), 52u64.into())]
// #[case(u128::MAX, u128::MAX, 7u64.into(), 1u64.into())]
// #[case(u128::MAX, u128::MAX, 100u64.into(), 25u64.into())]
// #[case(u128::MAX, 2u64.into(), u128::MAX, 0u64.into())]
// #[case(u128::MAX, 3u64.into(), u128::MAX, 0u64.into())]
fn mulmod_u128(
    #[case] lhs: u128,
    #[case] rhs: u128,
    #[case] modulus: u128,
    #[case] expected: u128,
) {
    let mut ops = Vec::new();
    ops.extend(make_u128(0x20, lhs));
    ops.extend(make_u128(0x21, rhs));
    ops.extend(make_u128(0x22, modulus));
    ops.extend(make_u128(0x23, 0u64.into()));
    ops.push(op::wdmm(0x23, 0x20, 0x21, 0x22));
    ops.push(op::movi(0x24, 16));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x23, 0x24));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 16] = data.clone().try_into().unwrap();
        let v = u128::from_be_bytes(bytes);
        assert_eq!(v, expected);
    } else {
        panic!("Expected logd receipt");
    }
}

#[rstest::rstest]
#[case(99u64.into(), 99u64.into(), 1u64.into(), 0u64.into())]
#[case(0u64.into(), 0u64.into(), 100u64.into(), 0u64.into())]
#[case(1u64.into(), 1u64.into(), 100u64.into(), 1u64.into())]
#[case(1u64.into(), 2u64.into(), 100u64.into(), 2u64.into())]
#[case(50u64.into(), 2u64.into(), 100u64.into(), 0u64.into())]
#[case(50u64.into(), 3u64.into(), 100u64.into(), 50u64.into())]
#[case(99u64.into(), 99u64.into(), 100u64.into(), 1u64.into())]
#[case(1234u64.into(), 5678u64.into(), 100u64.into(), 52u64.into())]
#[case(U256::MAX, U256::MAX, 7u64.into(), 1u64.into())]
#[case(U256::MAX, U256::MAX, 100u64.into(), 25u64.into())]
#[case(U256::MAX, 2u64.into(), U256::MAX, 0u64.into())]
#[case(U256::MAX, 3u64.into(), U256::MAX, 0u64.into())]
fn mulmod_u256(
    #[case] lhs: U256,
    #[case] rhs: U256,
    #[case] modulus: U256,
    #[case] expected: U256,
) {
    let mut ops = Vec::new();
    ops.extend(make_u256(0x20, lhs));
    ops.extend(make_u256(0x21, rhs));
    ops.extend(make_u256(0x22, modulus));
    ops.extend(make_u256(0x23, 0u64.into()));
    ops.push(op::wqmm(0x23, 0x20, 0x21, 0x22));
    ops.push(op::movi(0x24, 32));
    ops.push(op::logd(RegId::ZERO, RegId::ZERO, 0x23, 0x24));
    ops.push(op::ret(RegId::ONE));

    let receipts = run_script(ops);

    if let Receipt::LogData { data, .. } = receipts.first().unwrap() {
        let data = data.as_ref().unwrap();
        let bytes: [u8; 32] = data.clone().try_into().unwrap();
        let v = U256::from_be_bytes(bytes);
        assert_eq!(v, expected);
    } else {
        panic!("Expected logd receipt");
    }
}
