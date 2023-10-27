use alloc::{
    vec,
    vec::Vec,
};

use fuel_asm::*;
use fuel_tx::Receipt;
use fuel_types::Immediate18;

use super::test_helpers;
use rstest::rstest;

use test_helpers::{
    assert_panics,
    assert_success,
    run_script,
};

/// Setup some useful values
/// * 0x31 to all ones, i.e. max word
/// * 0x32 to two
fn common_setup() -> Vec<Instruction> {
    vec![op::not(0x31, RegId::ZERO), op::movi(0x32, 2)]
}

#[rstest]
fn spec_unsafemath_flag(
    #[values(
        op::div(0x10, RegId::ZERO, RegId::ZERO),
        op::divi(0x10, RegId::ZERO, 0),
        op::mlog(0x10, 0x31, RegId::ZERO),
        op::mod_(0x10, 0x31, RegId::ZERO),
        op::modi(0x10, 0x31, 0),
        op::mroo(0x10, 0x31, RegId::ZERO)
    )]
    case: Instruction,
    #[values(true, false)] flag: bool,
) {
    let mut script = common_setup();
    if flag {
        script.extend(&[op::addi(0x10, RegId::ZERO, 0x01), op::flag(0x10)]);
    }
    script.push(case);
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script);

    if flag {
        if let Receipt::Log { rd: err, .. } = receipts[0] {
            assert_eq!(err, 1);
        } else {
            panic!("No log data");
        }
    } else {
        assert_panics(&receipts, PanicReason::ArithmeticError);
    }
}

#[rstest]
fn spec_wrapping_flag(
    #[values(
        op::add(0x10, 0x31, 0x31),
        op::addi(0x10, 0x31, 0x31),
        op::exp(0x10, 0x31, 0x31),
        op::expi(0x10, 0x31, 0x31),
        op::mul(0x10, 0x31, 0x31),
        op::muli(0x10, 0x31, 0x31),
        op::sub(0x10, RegId::ZERO, RegId::ONE),
        op::subi(0x10, RegId::ZERO, 1)
    )]
    case: Instruction,
    #[values(true, false)] flag: bool,
) {
    let mut script = common_setup();
    if flag {
        script.extend(&[op::addi(0x10, RegId::ZERO, 0x02), op::flag(0x10)]);
    }
    script.push(case);
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if flag {
        if let Receipt::Log { rc: of, .. } = receipts[0] {
            assert_ne!(of, 0);
        } else {
            panic!("No log data");
        }
    } else {
        assert_panics(&receipts, PanicReason::ArithmeticOverflow);
    }
}

#[rstest]
fn spec_logic_ops_clear_of(
    #[values(
        op::and(0x10, RegId::ZERO, RegId::ONE),
        op::andi(0x10, RegId::ZERO, 1),
        op::eq(0x10, RegId::ZERO, RegId::ONE),
        op::gt(0x10, RegId::ZERO, RegId::ONE),
        op::lt(0x10, RegId::ZERO, RegId::ONE),
        op::move_(0x10, RegId::ONE),
        op::movi(0x10, 1),
        op::noop(),
        op::or(0x10, RegId::ZERO, RegId::ONE),
        op::ori(0x10, RegId::ZERO, 1),
        op::sll(0x10, RegId::ZERO, RegId::ONE),
        op::slli(0x10, RegId::ZERO, 1),
        op::srl(0x10, RegId::ZERO, RegId::ONE),
        op::srli(0x10, RegId::ZERO, 1),
        op::xor(0x10, RegId::ZERO, RegId::ONE),
        op::xori(0x10, RegId::ZERO, 1)
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    script.extend(&[op::addi(0x10, RegId::ZERO, 0x02), op::flag(0x10)]);
    script.push(op::add(0x10, 0x31, 0x31)); // Set $of to nonzero
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(case); // Check that the logic op clears it
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { rc: of, .. } = receipts[0] {
        assert_ne!(of, 0);
    } else {
        panic!("No log data");
    }

    if let Receipt::Log { rc: of, .. } = receipts[1] {
        assert_eq!(of, 0);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_alu_immediates_are_zero_extended(
    #[values(
        (op::addi(0x10, RegId::ZERO, Imm12::MAX.into()), Imm12::MAX.into()),
        (op::andi(0x10, 0x31, Imm12::MAX.into()), Imm12::MAX.into()),
        (op::divi(0x10, 0x31, Imm12::MAX.into()), u64::MAX / (Imm12::MAX.to_u16() as u64)),
        (op::expi(0x10, 1, Imm12::MAX.into()), 1), // pow(Imm12::MAX, 2) would overflow
        (op::modi(0x10, 0x31, Imm12::MAX.into()), u64::MAX % (Imm12::MAX.to_u16() as u64)),
        (op::movi(0x10, Imm18::MAX.into()), Imm18::MAX.into()),
        (op::muli(0x10, 0x32, Imm12::MAX.into()), 8190),
        (op::ori(0x10, RegId::ZERO, Imm12::MAX.into()), Imm12::MAX.into()),
        (op::slli(0x10, 0x31, Imm12::MAX.into()), 0), // These cases don't make much sense, since
        (op::srli(0x10, 0x31, Imm12::MAX.into()), 0), // shifting more than 64 bits is meaningless
        (op::subi(0x10, 0x31, Imm12::MAX.into()), u64::MAX - (Imm12::MAX.to_u16() as u64)),
        (op::xori(0x10, 0x31, Imm12::MAX.into()), u64::MAX ^ (Imm12::MAX.to_u16() as u64))
    )]
    case: (Instruction, u64),
) {
    let (op, expected) = case;

    let mut script = common_setup();
    script.push(op);
    script.push(op::log(0x10, RegId::ZERO, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { ra, .. } = receipts[0] {
        assert_eq!(ra, expected);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_logic_ops_clear_err(
    #[values(
        op::and(0x10, RegId::ZERO, RegId::ONE),
        op::andi(0x10, RegId::ZERO, 1),
        op::eq(0x10, RegId::ZERO, RegId::ONE),
        op::gt(0x10, RegId::ZERO, RegId::ONE),
        op::lt(0x10, RegId::ZERO, RegId::ONE),
        op::move_(0x10, RegId::ONE),
        op::movi(0x10, 1),
        op::noop(),
        op::or(0x10, RegId::ZERO, RegId::ONE),
        op::ori(0x10, RegId::ZERO, 1),
        op::sll(0x10, RegId::ZERO, RegId::ONE),
        op::slli(0x10, RegId::ZERO, 1),
        op::srl(0x10, RegId::ZERO, RegId::ONE),
        op::srli(0x10, RegId::ZERO, 1),
        op::xor(0x10, RegId::ZERO, RegId::ONE),
        op::xori(0x10, RegId::ZERO, 1)
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    script.extend(&[op::addi(0x10, RegId::ZERO, 0x01), op::flag(0x10)]);
    script.push(op::div(0x10, RegId::ZERO, RegId::ZERO)); // Set $err to nonzero
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(case); // Check that the logic op clears it
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { rd: err, .. } = receipts[0] {
        assert_ne!(err, 0);
    } else {
        panic!("No log data");
    }

    if let Receipt::Log { rd: err, .. } = receipts[1] {
        assert_eq!(err, 0);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_reserved_reg_write(
    #[values(
        op::add(0, 0, 0),
        op::addi(0, 0, 0),
        op::and(0, 0, 0),
        op::andi(0, 0, 0),
        op::div(0, 0, 0),
        op::divi(0, 0, 0),
        op::eq(0, 0, 0),
        op::exp(0, 0, 0),
        op::expi(0, 0, 0),
        op::gt(0, 0, 0),
        op::lt(0, 0, 0),
        op::mlog(0, 0, 0),
        op::mod_(0, 0, 0),
        op::modi(0, 0, 0),
        op::move_(0, 0),
        op::movi(0, 0),
        op::mroo(0, 0, 0),
        op::mul(0, 0, 0),
        op::muli(0, 0, 0),
        op::not(0, 0),
        op::or(0, 0, 0),
        op::ori(0, 0, 0),
        op::sll(0, 0, 0),
        op::slli(0, 0, 0),
        op::srl(0, 0, 0),
        op::srli(0, 0, 0),
        op::sub(0, 0, 0),
        op::subi(0, 0, 0),
        op::xor(0, 0, 0),
        op::xori(0, 0, 0)
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    script.push(case);
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::ReservedRegisterNotWritable);
}

#[rstest]
fn spec_incr_pc_by_four(
    #[values(
        op::add(RegId::WRITABLE, 0, 0),
        op::addi(RegId::WRITABLE, 0, 0),
        op::and(RegId::WRITABLE, 0, 0),
        op::andi(RegId::WRITABLE, 0, 0),
        op::div(RegId::WRITABLE, 0, 1),
        op::divi(RegId::WRITABLE, 0, 1),
        op::eq(RegId::WRITABLE, 0, 0),
        op::exp(RegId::WRITABLE, 0, 0),
        op::expi(RegId::WRITABLE, 0, 0),
        op::gt(RegId::WRITABLE, 0, 0),
        op::lt(RegId::WRITABLE, 0, 0),
        op::mlog(RegId::WRITABLE, 1, 0x32),
        op::mod_(RegId::WRITABLE, 0, 1),
        op::modi(RegId::WRITABLE, 0, 1),
        op::move_(RegId::WRITABLE, 0),
        op::movi(RegId::WRITABLE, 0),
        op::mroo(RegId::WRITABLE, 0, 0x32),
        op::mul(RegId::WRITABLE, 0, 0),
        op::muli(RegId::WRITABLE, 0, 0),
        op::not(RegId::WRITABLE, 0),
        op::or(RegId::WRITABLE, 0, 0),
        op::ori(RegId::WRITABLE, 0, 0),
        op::sll(RegId::WRITABLE, 0, 0),
        op::slli(RegId::WRITABLE, 0, 0),
        op::srl(RegId::WRITABLE, 0, 0),
        op::srli(RegId::WRITABLE, 0, 0),
        op::sub(RegId::WRITABLE, 0, 0),
        op::subi(RegId::WRITABLE, 0, 0),
        op::xor(RegId::WRITABLE, 0, 0),
        op::xori(RegId::WRITABLE, 0, 0)
    )]
    case: Instruction,
    #[values(0, 1, 2)] offset: usize,
) {
    let mut script = common_setup();
    for _ in 0..offset {
        script.push(op::noop());
    }
    let setup_size = (script.len() as Word) * 4;
    script.push(case);
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { ra: is, rb: pc, .. } = receipts[0] {
        assert_eq!(is + setup_size + 4, pc);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_can_write_allowed_flag_combinations(
    #[values(0b00, 0b01, 0b10, 0b11)] flags: Immediate18,
) {
    let mut script = common_setup();
    script.push(op::movi(0x20, flags));
    script.push(op::flag(0x20));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_success(&receipts);
}

#[rstest]
fn spec_cannot_write_reserved_flags(#[values(0b100, 0b111)] flags: Immediate18) {
    let mut script = common_setup();
    script.push(op::movi(0x20, flags));
    script.push(op::flag(0x20));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::InvalidFlags);
}
