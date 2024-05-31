use alloc::vec;

use crate::prelude::*;
use fuel_asm::{
    op,
    Imm18,
    Instruction,
    RegId,
};

use super::test_helpers::set_full_word;

fn alu(
    registers_init: &[(RegisterId, Word)],
    ins: Instruction,
    reg: RegisterId,
    expected: Word,
) {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;
    let reg = u8::try_from(reg).unwrap();

    let script = registers_init
        .iter()
        .flat_map(|(r, v)| set_full_word(*r, *v))
        .chain([ins, op::log(reg, 0, 0, 0), op::ret(RegId::ONE)])
        .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    assert_eq!(
        receipts
            .first()
            .expect("Receipt not found")
            .ra()
            .expect("$ra expected"),
        expected
    );
}

fn alu_overflow(program: &[Instruction], reg: RegisterId, expected: u128, boolean: bool) {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let script = program
        .iter()
        .copied()
        .chain([op::ret(RegId::ONE)])
        .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    // TODO rename reason method
    // https://github.com/FuelLabs/fuel-tx/issues/120
    let result = receipts
        .first()
        .expect("Failed to fetch receipt")
        .reason()
        .expect("Failed to fetch instruction result");

    assert_eq!(&PanicReason::ArithmeticOverflow, result.reason());

    // TODO avoid magic constants
    // https://github.com/FuelLabs/fuel-asm/issues/60
    let script = [op::movi(0x10, 0x02), op::flag(0x10)]
        .into_iter()
        .chain(program.iter().copied())
        .chain([
            op::log(u8::try_from(reg).unwrap(), RegId::OF, 0, 0),
            op::ret(RegId::ONE),
        ])
        .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    if !boolean {
        let lo_value = receipts
            .first()
            .expect("Receipt not found")
            .ra()
            .expect("$ra expected");
        let hi_value = receipts
            .first()
            .expect("Receipt not found")
            .rb()
            .expect("$rb expected");

        let overflow_value = lo_value as u128 + ((hi_value as u128) << 64);

        assert_eq!(overflow_value, expected);
    } else {
        let overflow = receipts
            .first()
            .expect("Receipt not found")
            .rb()
            .expect("$ra expected");
        assert_eq!(overflow, expected as u64);
    }
}

fn alu_wrapping(
    registers_init: &[(RegisterId, Word)],
    ins: Instruction,
    reg: RegisterId,
    expected: Word,
    expected_of: bool,
) {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;
    let set_regs = registers_init
        .iter()
        .flat_map(|(r, v)| set_full_word(*r, *v));

    let script = vec![
        // TODO avoid magic constants
        // https://github.com/FuelLabs/fuel-asm/issues/60
        op::movi(RegId::WRITABLE, 0x2),
        op::flag(RegId::WRITABLE),
    ]
    .into_iter()
    .chain(set_regs)
    .chain([
        ins,
        op::log(u8::try_from(reg).unwrap(), RegId::OF, 0, 0),
        op::ret(RegId::ONE),
    ])
    .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    let log_receipt = receipts.first().expect("Receipt not found");

    assert_eq!(log_receipt.ra().expect("$ra expected"), expected);

    let expected_of: u64 = expected_of.into();
    assert_eq!(
        log_receipt.rb().expect("$rb (value of RegId::OF) expected"),
        expected_of
    );
}

fn alu_err(
    registers_init: &[(RegisterId, Immediate18)],
    ins: Instruction,
    reg: RegisterId,
    expected: Word,
) {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;
    let reg = u8::try_from(reg).unwrap();

    let script = registers_init
        .iter()
        .map(|(r, v)| op::movi(u8::try_from(*r).unwrap(), *v))
        .chain([ins, op::ret(RegId::ONE)])
        .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    // TODO rename reason method
    // https://github.com/FuelLabs/fuel-tx/issues/120
    let result = receipts
        .first()
        .expect("Failed to fetch receipt")
        .reason()
        .expect("Failed to fetch instruction result");

    assert_eq!(&PanicReason::ArithmeticError, result.reason());

    // TODO avoid magic constants
    // https://github.com/FuelLabs/fuel-asm/issues/60
    let script = [op::movi(0x10, 0x01), op::flag(0x10)]
        .into_iter()
        .chain(
            registers_init
                .iter()
                .map(|(r, v)| op::movi(u8::try_from(*r).unwrap(), *v)),
        )
        .chain([ins, op::log(reg, 0, 0, 0), op::ret(RegId::ONE)])
        .collect();

    let result = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    assert_eq!(
        receipts
            .first()
            .expect("Receipt not found")
            .ra()
            .expect("$ra expected"),
        expected
    );
}

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

#[test]
fn add() {
    alu(
        &[(0x10, 128), (0x11, 25)],
        op::add(0x12, 0x10, 0x11),
        0x12,
        153,
    );
    alu_overflow(
        &[
            op::move_(0x10, RegId::ZERO),
            op::movi(0x11, 10),
            op::not(0x10, 0x10),
            op::add(0x10, 0x10, 0x11),
        ],
        0x10,
        Word::MAX as u128 + 10,
        false,
    );
}

#[test]
fn addi() {
    alu(&[(0x10, 128)], op::addi(0x11, 0x10, 25), 0x11, 153);
    alu_overflow(
        &[
            op::move_(0x10, RegId::ZERO),
            op::not(0x10, 0x10),
            op::addi(0x10, 0x10, 10),
        ],
        0x10,
        Word::MAX as u128 + 10,
        false,
    );
}

#[test]
fn mul() {
    alu(
        &[(0x10, 128), (0x11, 25)],
        op::mul(0x12, 0x10, 0x11),
        0x12,
        3200,
    );
    alu_overflow(
        &[
            op::move_(0x10, RegId::ZERO),
            op::movi(0x11, 2),
            op::not(0x10, 0x10),
            op::mul(0x10, 0x10, 0x11),
        ],
        0x10,
        Word::MAX as u128 * 2,
        false,
    );
}

#[test]
fn muli() {
    alu(&[(0x10, 128)], op::muli(0x11, 0x10, 25), 0x11, 3200);
    alu_overflow(
        &[
            op::move_(0x10, RegId::ZERO),
            op::not(0x10, 0x10),
            op::muli(0x10, 0x10, 2),
        ],
        0x10,
        Word::MAX as u128 * 2,
        false,
    );
}
#[test]
fn mldv() {
    alu(
        &[(0x10, u64::MAX), (0x11, 3), (0x12, 6)],
        op::mldv(0x13, 0x10, 0x11, 0x12),
        0x13,
        u64::MAX / 2,
    );
}

#[test]
fn sll() {
    alu(
        &[(0x10, 128), (0x11, 2)],
        op::sll(0x12, 0x10, 0x11),
        0x12,
        512,
    );
    // test boundary 1<<63 == Word::MAX
    alu(
        &[(0x10, 1), (0x11, 63)],
        op::sll(0x12, 0x10, 0x11),
        0x12,
        1 << 63,
    );
    // test overflow 1<<64 == 0
    alu(&[(0x10, 1), (0x11, 64)], op::sll(0x12, 0x10, 0x11), 0x12, 0);
    // test too large shift
    alu(
        &[(0x10, 1), (0x11, Word::MAX)],
        op::sll(0x12, 0x10, 0x11),
        0x12,
        0,
    );
}

#[test]
fn slli() {
    alu(&[(0x10, 128)], op::slli(0x11, 0x10, 2), 0x11, 512);
    // test boundary 1<<63 == 1<<63
    alu(&[(0x10, 1)], op::slli(0x11, 0x10, 63), 0x11, 1 << 63);
    // test overflow 1<<64 == 0
    alu(&[(0x10, 1)], op::slli(0x11, 0x10, 64), 0x11, 0);
}

#[test]
fn srl() {
    alu(
        &[(0x10, 128), (0x11, 2)],
        op::srl(0x12, 0x10, 0x11),
        0x12,
        32,
    );
    // test boundary 2>>1 == 1
    alu(&[(0x10, 2), (0x11, 1)], op::srl(0x12, 0x10, 0x11), 0x12, 1);
    // test overflow 1>>1 == 0
    alu(&[(0x10, 1), (0x11, 1)], op::srl(0x12, 0x10, 0x11), 0x12, 0);
    // test too large shift
    alu(
        &[(0x10, 1), (0x11, Word::MAX)],
        op::srl(0x12, 0x10, 0x11),
        0x12,
        0,
    );
}

#[test]
fn srli() {
    alu(&[(0x10, 128)], op::srli(0x11, 0x10, 2), 0x11, 32);
    // test boundary 2>>1 == 1
    alu(&[(0x10, 2)], op::srli(0x11, 0x10, 1), 0x11, 1);
    // test overflow 1>>1 == 0
    alu(&[(0x10, 1)], op::srli(0x11, 0x10, 1), 0x11, 0);
}

#[test]
fn sub() {
    alu(
        &[(0x10, 128), (0x11, 25)],
        op::sub(0x12, 0x10, 0x11),
        0x12,
        103,
    );
    alu_overflow(
        &[
            op::move_(0x10, RegId::ZERO),
            op::movi(0x11, 10),
            op::sub(0x10, 0x10, 0x11),
        ],
        0x10,
        (0_u128).wrapping_sub(10),
        false,
    );
}

#[test]
fn subi() {
    alu(&[(0x10, 128)], op::subi(0x11, 0x10, 25), 0x11, 103);
    alu_overflow(
        &[op::move_(0x10, RegId::ZERO), op::subi(0x10, 0x10, 10)],
        0x10,
        (0_u128).wrapping_sub(10),
        false,
    );
}

#[test]
fn div() {
    alu(
        &[(0x10, 59), (0x11, 10)],
        op::div(0x12, 0x10, 0x11),
        0x12,
        5,
    );
    alu(&[(0x10, 59)], op::divi(0x12, 0x10, 10), 0x12, 5);
    alu_err(&[], op::div(0x10, RegId::ONE, RegId::ZERO), 0x10, 0x00);
    alu_err(&[], op::divi(0x10, RegId::ONE, 0), 0x10, 0x00);
}

#[test]
fn mod_() {
    alu(
        &[(0x10, 59), (0x11, 10)],
        op::mod_(0x12, 0x10, 0x11),
        0x12,
        9,
    );
    alu(&[(0x10, 59)], op::modi(0x12, 0x10, 10), 0x12, 9);
    alu_err(&[], op::mod_(0x10, RegId::ONE, RegId::ZERO), 0x10, 0x00);
    alu_err(&[], op::modi(0x10, RegId::ONE, 0), 0x10, 0x00);
}

#[test]
fn eq() {
    alu(&[(0x10, 10), (0x11, 10)], op::eq(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 11), (0x11, 10)], op::eq(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn exp() {
    // EXP
    alu(
        &[(0x10, 6), (0x11, 3)],
        op::exp(0x12, 0x10, 0x11),
        0x12,
        216,
    );
    alu_overflow(
        &[
            op::movi(0x10, 2),
            op::movi(0x11, 64),
            op::exp(0x10, 0x10, 0x11),
        ],
        0x10,
        true as u128,
        true,
    );
    alu_wrapping(
        &[(0x10, 2), (0x11, 32)],
        op::exp(0x10, 0x10, 0x11),
        0x10,
        2u64.pow(32),
        false,
    );
    alu_wrapping(
        &[(0x10, 2), (0x11, 64)],
        op::exp(0x10, 0x10, 0x11),
        0x10,
        0,
        true,
    );

    // EXPI
    alu(&[(0x10, 6)], op::expi(0x12, 0x10, 3), 0x12, 216);
    alu_overflow(
        &[op::movi(0x10, 2), op::expi(0x10, 0x10, 64)],
        0x10,
        true as u128,
        true,
    );
    alu_wrapping(
        &[(0x10, 2)],
        op::expi(0x10, 0x10, 32),
        0x10,
        2u64.pow(32),
        false,
    );
    alu_wrapping(&[(0x10, 2)], op::expi(0x10, 0x10, 64), 0x10, 0, true);
}

#[test]
fn mroo() {
    alu(&[(0x10, 0), (0x11, 1)], op::mroo(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 1)], op::mroo(0x12, 0x10, 0x11), 0x12, 2);
    alu(
        &[(0x10, 1234), (0x11, 1)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        1234,
    );

    alu(&[(0x10, 0), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 1);
    alu(
        &[(0x10, 16), (0x11, 2)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        4,
    );
    alu(
        &[(0x10, 17), (0x11, 2)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        4,
    );
    alu(
        &[(0x10, 24), (0x11, 2)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        4,
    );
    alu(
        &[(0x10, 25), (0x11, 2)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        5,
    );
    alu(
        &[(0x10, 26), (0x11, 2)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        5,
    );

    alu(
        &[(0x10, 26), (0x11, 3)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        2,
    );
    alu(
        &[(0x10, 27), (0x11, 3)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        3,
    );

    alu(
        &[(0x10, 2441), (0x11, 12)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        1,
    );
    alu(
        &[(0x10, 4327279578356147249), (0x11, 7)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        459,
    );
    alu(
        &[(0x10, 2567305238000531939), (0x11, 16)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        14,
    );
    alu(
        &[(0x10, 15455138536657945190), (0x11, 2)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        3931302396,
    );
    alu(
        &[(0x10, 11875230360893570326), (0x11, 27)],
        op::mroo(0x12, 0x10, 0x11),
        0x12,
        5,
    );

    alu_err(&[(0x10, 2), (0x11, 0)], op::mroo(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn mlog() {
    alu(
        &[(0x10, 1), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        0,
    );
    alu(
        &[(0x10, 10), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        1,
    );
    alu(
        &[(0x10, 100), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        2,
    );
    alu(
        &[(0x10, 999), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        2,
    );
    alu(
        &[(0x10, 1000), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        3,
    );
    alu(
        &[(0x10, 1001), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        3,
    );

    alu(&[(0x10, 1), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 4), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 2);

    alu(
        &[(0x10, 2u64.pow(32)), (0x11, 2)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        32,
    );
    alu(
        &[(0x10, Word::MAX), (0x11, 2)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        63,
    );
    alu(
        &[(0x10, 10u64.pow(10)), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        10,
    );
    alu(
        &[(0x10, 10u64.pow(11)), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        11,
    );

    alu_err(
        &[(0x10, 0), (0x11, 10)],
        op::mlog(0x12, 0x10, 0x11),
        0x12,
        0,
    );
    alu_err(&[(0x10, 0), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn campare_lt_gt() {
    alu(&[(0x10, 6), (0x11, 3)], op::lt(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 3), (0x11, 3)], op::lt(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 1), (0x11, 3)], op::lt(0x12, 0x10, 0x11), 0x12, 1);

    alu(&[(0x10, 6), (0x11, 3)], op::gt(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 3)], op::gt(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 1), (0x11, 3)], op::gt(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn move_and_movi() {
    alu(&[], op::move_(0x12, 0x10), 0x12, 0);
    alu(&[(0x10, 6)], op::move_(0x12, 0x10), 0x12, 6);
    alu(&[(0x10, Word::MAX)], op::move_(0x12, 0x10), 0x12, Word::MAX);

    alu(&[], op::movi(0x12, 0), 0x12, 0);
    alu(
        &[],
        op::movi(0x12, Imm18::MAX.into()),
        0x12,
        Imm18::MAX.into(),
    );
}

#[test]
fn not() {
    alu(&[], op::not(0x12, 0x10), 0x12, Word::MAX);
    alu(&[(0x10, 3)], op::not(0x12, 0x10), 0x12, Word::MAX ^ 3);
}

#[test]
fn bitwise_and_or_xor() {
    alu(
        &[(0x10, 0xcc), (0x11, 0xaa)],
        op::and(0x12, 0x10, 0x11),
        0x12,
        0x88,
    );
    alu(&[(0x10, 0xcc)], op::andi(0x12, 0x10, 0xaa), 0x12, 0x88);

    alu(
        &[(0x10, 0x11), (0x11, 0x22)],
        op::or(0x12, 0x10, 0x11),
        0x12,
        0x33,
    );
    alu(&[(0x10, 0x11)], op::ori(0x12, 0x10, 0x22), 0x12, 0x33);

    alu(
        &[(0x10, 0x33), (0x11, 0x22)],
        op::xor(0x12, 0x10, 0x11),
        0x12,
        0x11,
    );
    alu(&[(0x10, 0x33)], op::xori(0x12, 0x10, 0x22), 0x12, 0x11);
}
