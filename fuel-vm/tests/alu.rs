use fuel_asm::{op, Instruction};
use fuel_vm::consts::*;
use fuel_vm::prelude::*;

/// Set a register `r` to a Word-sized number value using left-shifts
fn set_full_word(r: RegisterId, v: Word) -> Vec<Instruction> {
    let r = u8::try_from(r).unwrap();
    let mut ops = vec![op::movi(r, 0)];
    for byte in v.to_be_bytes() {
        ops.push(op::ori(r, r, byte as Immediate12));
        ops.push(op::slli(r, r, 8));
    }
    ops.pop().unwrap(); // Remove last shift
    ops
}

fn alu(registers_init: &[(RegisterId, Word)], ins: Instruction, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let reg = u8::try_from(reg).unwrap();
    let gas_costs = GasCosts::default();

    let script = registers_init
        .iter()
        .flat_map(|(r, v)| set_full_word(*r, *v))
        .chain([ins, op::log(reg, 0, 0, 0), op::ret(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default(), gas_costs)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    assert_eq!(
        receipts.first().expect("Receipt not found").ra().expect("$ra expected"),
        expected
    );
}

fn alu_overflow(program: &[Instruction], reg: RegisterId, expected: u128, boolean: bool) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let script = program
        .iter()
        .copied()
        .chain([op::ret(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage.clone(), Default::default(), gas_costs.clone())
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

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
        .chain(
            [op::log(u8::try_from(reg).unwrap(), REG_OF, 0, 0), op::ret(REG_ONE)]
                .iter()
                .copied(),
        )
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default(), gas_costs)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    if !boolean {
        let lo_value = receipts.first().expect("Receipt not found").ra().expect("$ra expected");
        let hi_value = receipts.first().expect("Receipt not found").rb().expect("$rb expected");

        let overflow_value = lo_value as u128 + ((hi_value as u128) << 64);

        assert_eq!(overflow_value, expected);
    } else {
        let overflow = receipts.first().expect("Receipt not found").rb().expect("$ra expected");
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
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let set_regs = registers_init.iter().flat_map(|(r, v)| set_full_word(*r, *v));

    let script = [
        // TODO avoid magic constants
        // https://github.com/FuelLabs/fuel-asm/issues/60
        op::movi(REG_WRITABLE, 0x2),
        op::flag(REG_WRITABLE),
    ]
    .iter()
    .copied()
    .chain(set_regs)
    .chain(
        [ins, op::log(u8::try_from(reg).unwrap(), REG_OF, 0, 0), op::ret(REG_ONE)]
            .iter()
            .copied(),
    )
    .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default(), gas_costs)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    let log_receipt = receipts.first().expect("Receipt not found");

    assert_eq!(log_receipt.ra().expect("$ra expected"), expected);

    let expected_of: u64 = expected_of.try_into().unwrap();
    assert_eq!(log_receipt.rb().expect("$rb (value of REG_OF) expected"), expected_of);
}

fn alu_err(registers_init: &[(RegisterId, Immediate18)], ins: Instruction, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let reg = u8::try_from(reg).unwrap();
    let gas_costs = GasCosts::default();

    let script = registers_init
        .iter()
        .map(|(r, v)| op::movi(u8::try_from(*r).unwrap(), *v))
        .chain([ins, op::ret(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage.clone(), Default::default(), gas_costs.clone())
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    // TODO rename reason method
    // https://github.com/FuelLabs/fuel-tx/issues/120
    let result = receipts
        .first()
        .expect("Failed to fetch receipt")
        .reason()
        .expect("Failed to fetch instruction result");

    assert_eq!(&PanicReason::ErrorFlag, result.reason());

    // TODO avoid magic constants
    // https://github.com/FuelLabs/fuel-asm/issues/60
    let script = [op::movi(0x10, 0x01), op::flag(0x10)]
        .into_iter()
        .chain(
            registers_init
                .iter()
                .map(|(r, v)| op::movi(u8::try_from(*r).unwrap(), *v)),
        )
        .chain([ins, op::log(reg, 0, 0, 0), op::ret(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default(), gas_costs)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    assert_eq!(
        receipts.first().expect("Receipt not found").ra().expect("$ra expected"),
        expected
    );
}

fn alu_reserved(registers_init: &[(RegisterId, Word)], ins: Instruction) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let script = registers_init
        .iter()
        .flat_map(|(r, v)| set_full_word(*r, *v))
        .chain([ins, op::ret(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default(), gas_costs)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    let result = receipts
        .iter()
        .find_map(Receipt::reason)
        .map(|r| *r.reason())
        .expect("Expected panic reason");

    assert_eq!(PanicReason::ReservedRegisterNotWritable, result);
}

#[test]
fn reserved_register() {
    alu_reserved(&[(0x10, 128)], op::add(REG_ZERO, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_ONE, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_OF, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_PC, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_SSP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_SP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_FP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_HP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_ERR, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_GGAS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_CGAS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_BAL, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_IS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_RET, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_RETL, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], op::add(REG_FLAG, 0x10, 0x11));
}

#[test]
fn add() {
    alu(&[(0x10, 128), (0x11, 25)], op::add(0x12, 0x10, 0x11), 0x12, 153);
    alu_overflow(
        &[
            op::move_(0x10, REG_ZERO),
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
        &[op::move_(0x10, REG_ZERO), op::not(0x10, 0x10), op::addi(0x10, 0x10, 10)],
        0x10,
        Word::MAX as u128 + 10,
        false,
    );
}

#[test]
fn mul() {
    alu(&[(0x10, 128), (0x11, 25)], op::mul(0x12, 0x10, 0x11), 0x12, 3200);
    alu_overflow(
        &[
            op::move_(0x10, REG_ZERO),
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
        &[op::move_(0x10, REG_ZERO), op::not(0x10, 0x10), op::muli(0x10, 0x10, 2)],
        0x10,
        Word::MAX as u128 * 2,
        false,
    );
}

#[test]
fn sll() {
    alu(&[(0x10, 128), (0x11, 2)], op::sll(0x12, 0x10, 0x11), 0x12, 512);
    // test boundary 1<<63 == Word::MAX
    alu(&[(0x10, 1), (0x11, 63)], op::sll(0x12, 0x10, 0x11), 0x12, 1 << 63);
    // test overflow 1<<64 == 0
    alu(&[(0x10, 1), (0x11, 64)], op::sll(0x12, 0x10, 0x11), 0x12, 0);
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
    alu(&[(0x10, 128), (0x11, 2)], op::srl(0x12, 0x10, 0x11), 0x12, 32);
    // test boundary 2>>1 == 1
    alu(&[(0x10, 2), (0x11, 1)], op::srl(0x12, 0x10, 0x11), 0x12, 1);
    // test overflow 1>>1 == 0
    alu(&[(0x10, 1), (0x11, 1)], op::srl(0x12, 0x10, 0x11), 0x12, 0);
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
    alu(&[(0x10, 128), (0x11, 25)], op::sub(0x12, 0x10, 0x11), 0x12, 103);
    alu_overflow(
        &[op::move_(0x10, REG_ZERO), op::movi(0x11, 10), op::sub(0x10, 0x10, 0x11)],
        0x10,
        (0_u128).wrapping_sub(10),
        false,
    );
}

#[test]
fn subi() {
    alu(&[(0x10, 128)], op::subi(0x11, 0x10, 25), 0x11, 103);
    alu_overflow(
        &[op::move_(0x10, REG_ZERO), op::subi(0x10, 0x10, 10)],
        0x10,
        (0_u128).wrapping_sub(10),
        false,
    );
}

#[test]
fn and() {
    alu(&[(0x10, 0xcc), (0x11, 0xaa)], op::and(0x12, 0x10, 0x11), 0x12, 0x88);
    alu(&[(0x10, 0xcc)], op::andi(0x12, 0x10, 0xaa), 0x12, 0x88);
}

#[test]
fn div() {
    alu(&[(0x10, 59), (0x11, 10)], op::div(0x12, 0x10, 0x11), 0x12, 5);
    alu(&[(0x10, 59)], op::divi(0x12, 0x10, 10), 0x12, 5);
    alu_err(&[], op::divi(0x10, REG_ONE, 0), 0x10, 0x00);
}

#[test]
fn eq() {
    alu(&[(0x10, 10), (0x11, 10)], op::eq(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 11), (0x11, 10)], op::eq(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn exp() {
    // EXP
    alu(&[(0x10, 6), (0x11, 3)], op::exp(0x12, 0x10, 0x11), 0x12, 216);
    alu_overflow(
        &[op::movi(0x10, 2), op::movi(0x11, 64), op::exp(0x10, 0x10, 0x11)],
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
    alu_wrapping(&[(0x10, 2), (0x11, 64)], op::exp(0x10, 0x10, 0x11), 0x10, 0, true);

    // EXPI
    alu(&[(0x10, 6)], op::expi(0x12, 0x10, 3), 0x12, 216);
    alu_overflow(&[op::movi(0x10, 2), op::expi(0x10, 0x10, 64)], 0x10, true as u128, true);
    alu_wrapping(&[(0x10, 2)], op::expi(0x10, 0x10, 32), 0x10, 2u64.pow(32), false);
    alu_wrapping(&[(0x10, 2)], op::expi(0x10, 0x10, 64), 0x10, 0, true);
}

#[test]
fn mroo() {
    alu(&[(0x10, 0), (0x11, 1)], op::mroo(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 1)], op::mroo(0x12, 0x10, 0x11), 0x12, 2);
    alu(&[(0x10, 1234), (0x11, 1)], op::mroo(0x12, 0x10, 0x11), 0x12, 1234);

    alu(&[(0x10, 0), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 16), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 4);
    alu(&[(0x10, 17), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 4);
    alu(&[(0x10, 24), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 4);
    alu(&[(0x10, 25), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 5);
    alu(&[(0x10, 26), (0x11, 2)], op::mroo(0x12, 0x10, 0x11), 0x12, 5);

    alu(&[(0x10, 26), (0x11, 3)], op::mroo(0x12, 0x10, 0x11), 0x12, 2);
    alu(&[(0x10, 27), (0x11, 3)], op::mroo(0x12, 0x10, 0x11), 0x12, 3);

    alu(&[(0x10, 2441), (0x11, 12)], op::mroo(0x12, 0x10, 0x11), 0x12, 1);
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
    alu(&[(0x10, 1), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 10), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 100), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 2);
    alu(&[(0x10, 999), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 2);
    alu(&[(0x10, 1000), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 3);
    alu(&[(0x10, 1001), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 3);

    alu(&[(0x10, 1), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 4), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 2);

    alu(&[(0x10, 2u64.pow(32)), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 32);
    alu(&[(0x10, Word::MAX), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 63);
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

    alu_err(&[(0x10, 0), (0x11, 10)], op::mlog(0x12, 0x10, 0x11), 0x12, 0);
    alu_err(&[(0x10, 0), (0x11, 2)], op::mlog(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn gt() {
    alu(&[(0x10, 6), (0x11, 3)], op::gt(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 3)], op::gt(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 1), (0x11, 3)], op::gt(0x12, 0x10, 0x11), 0x12, 0);
}
