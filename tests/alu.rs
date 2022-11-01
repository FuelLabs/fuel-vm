use fuel_vm::consts::*;
use fuel_vm::prelude::*;

/// Set a register `r` to a Word-sized number value using left-shifts
fn set_full_word(r: RegisterId, v: Word) -> Vec<Opcode> {
    let mut ops = vec![Opcode::MOVI(r, 0)];
    for byte in v.to_be_bytes() {
        ops.push(Opcode::ORI(r, r, byte as Immediate12));
        ops.push(Opcode::SLLI(r, r, 8));
    }
    ops.pop().unwrap(); // Remove last shift
    ops
}

fn alu(registers_init: &[(RegisterId, Word)], op: Opcode, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let script = registers_init
        .iter()
        .map(|(r, v)| set_full_word(*r, *v))
        .flatten()
        .chain([op, Opcode::LOG(reg, 0, 0, 0), Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default())
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    assert_eq!(
        receipts.first().expect("Receipt not found").ra().expect("$ra expected"),
        expected
    );
}


fn alu_overflow(program: &[Opcode], reg: RegisterId, expected: u128, boolean: bool) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let script = program
        .iter()
        .copied()
        .chain([Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage.clone(), Default::default())
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
    let script = [Opcode::MOVI(0x10, 0x02), Opcode::FLAG(0x10)]
        .into_iter()
        .chain(program.iter().copied())
        .chain([Opcode::LOG(reg, REG_OF, 0, 0), Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default())
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
    op: Opcode,
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

    let set_regs = registers_init.iter().map(|(r, v)| set_full_word(*r, *v)).flatten();

    let script = [
        // TODO avoid magic constants
        // https://github.com/FuelLabs/fuel-asm/issues/60
        Opcode::MOVI(REG_WRITABLE, 0x2),
        Opcode::FLAG(REG_WRITABLE),
    ]
    .iter()
    .copied()
    .chain(set_regs)
    .chain(
        [op, Opcode::LOG(reg, REG_OF, 0, 0), Opcode::RET(REG_ONE)]
            .iter()
            .copied(),
    )
    .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default())
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    let log_receipt = receipts.first().expect("Receipt not found");

    assert_eq!(log_receipt.ra().expect("$ra expected"), expected);

    let expected_of: u64 = expected_of.try_into().unwrap();
    assert_eq!(log_receipt.rb().expect("$rb (value of REG_OF) expected"), expected_of);
}

fn alu_err(registers_init: &[(RegisterId, Immediate18)], op: Opcode, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let script = registers_init
        .iter()
        .map(|(r, v)| Opcode::MOVI(*r, *v))
        .chain([op, Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage.clone(), Default::default())
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
    let script = [Opcode::MOVI(0x10, 0x01), Opcode::FLAG(0x10)]
        .into_iter()
        .chain(registers_init.iter().map(|(r, v)| Opcode::MOVI(*r, *v)))
        .chain([op, Opcode::LOG(reg, 0, 0, 0), Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default())
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    assert_eq!(
        receipts.first().expect("Receipt not found").ra().expect("$ra expected"),
        expected
    );
}

fn alu_reserved(registers_init: &[(RegisterId, Word)], op: Opcode) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let script = registers_init
        .iter()
        .map(|(r, v)| set_full_word(*r, *v))
        .flatten()
        .chain([op, Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let receipts = Transactor::new(storage, Default::default())
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
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_ZERO, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_ONE, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_OF, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_PC, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_SSP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_SP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_FP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_HP, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_ERR, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_GGAS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_CGAS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_BAL, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_IS, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_RET, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_RETL, 0x10, 0x11));
    alu_reserved(&[(0x10, 128)], Opcode::ADD(REG_FLAG, 0x10, 0x11));
}

#[test]
fn add() {
    alu(&[(0x10, 128), (0x11, 25)], Opcode::ADD(0x12, 0x10, 0x11), 0x12, 153);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::MOVI(0x11, 10),
            Opcode::NOT(0x10, 0x10),
            Opcode::ADD(0x10, 0x10, 0x11),
        ],
        0x10,
        Word::MAX as u128 + 10,
        false,
    );
}

#[test]
fn addi() {
    alu(&[(0x10, 128)], Opcode::ADDI(0x11, 0x10, 25), 0x11, 153);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::NOT(0x10, 0x10),
            Opcode::ADDI(0x10, 0x10, 10),
        ],
        0x10,
        Word::MAX as u128 + 10,
        false,
    );
}

#[test]
fn mul() {
    alu(&[(0x10, 128), (0x11, 25)], Opcode::MUL(0x12, 0x10, 0x11), 0x12, 3200);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::MOVI(0x11, 2),
            Opcode::NOT(0x10, 0x10),
            Opcode::MUL(0x10, 0x10, 0x11),
        ],
        0x10,
        Word::MAX as u128 * 2,
        false,
    );
}

#[test]
fn muli() {
    alu(&[(0x10, 128)], Opcode::MULI(0x11, 0x10, 25), 0x11, 3200);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::NOT(0x10, 0x10),
            Opcode::MULI(0x10, 0x10, 2),
        ],
        0x10,
        Word::MAX as u128 * 2,
        false,
    );
}

#[test]
fn sll() {
    alu(&[(0x10, 128), (0x11, 2)], Opcode::SLL(0x12, 0x10, 0x11), 0x12, 512);
    // test boundary 1<<63 == Word::MAX
    alu(&[(0x10, 1), (0x11, 63)], Opcode::SLL(0x12, 0x10, 0x11), 0x12, 1 << 63);
    // test overflow 1<<64 == 0
    alu(&[(0x10, 1), (0x11, 64)], Opcode::SLL(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn slli() {
    alu(&[(0x10, 128)], Opcode::SLLI(0x11, 0x10, 2), 0x11, 512);
    // test boundary 1<<63 == 1<<63
    alu(&[(0x10, 1)], Opcode::SLLI(0x11, 0x10, 63), 0x11, 1 << 63);
    // test overflow 1<<64 == 0
    alu(&[(0x10, 1)], Opcode::SLLI(0x11, 0x10, 64), 0x11, 0);
}

#[test]
fn srl() {
    alu(&[(0x10, 128), (0x11, 2)], Opcode::SRL(0x12, 0x10, 0x11), 0x12, 32);
    // test boundary 2>>1 == 1
    alu(&[(0x10, 2), (0x11, 1)], Opcode::SRL(0x12, 0x10, 0x11), 0x12, 1);
    // test overflow 1>>1 == 0
    alu(&[(0x10, 1), (0x11, 1)], Opcode::SRL(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn srli() {
    alu(&[(0x10, 128)], Opcode::SRLI(0x11, 0x10, 2), 0x11, 32);
    // test boundary 2>>1 == 1
    alu(&[(0x10, 2)], Opcode::SRLI(0x11, 0x10, 1), 0x11, 1);
    // test overflow 1>>1 == 0
    alu(&[(0x10, 1)], Opcode::SRLI(0x11, 0x10, 1), 0x11, 0);
}

#[test]
fn sub() {
    alu(&[(0x10, 128), (0x11, 25)], Opcode::SUB(0x12, 0x10, 0x11), 0x12, 103);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::MOVI(0x11, 10),
            Opcode::SUB(0x10, 0x10, 0x11),
        ],
        0x10,
        (0 as u128).wrapping_sub(10),
        false,
    );
}

#[test]
fn subi() {
    alu(&[(0x10, 128)], Opcode::SUBI(0x11, 0x10, 25), 0x11, 103);
    alu_overflow(
        &[Opcode::MOVE(0x10, REG_ZERO), Opcode::SUBI(0x10, 0x10, 10)],
        0x10,
        (0 as u128).wrapping_sub(10),
        false,
    );
}

#[test]
fn and() {
    alu(&[(0x10, 0xcc), (0x11, 0xaa)], Opcode::AND(0x12, 0x10, 0x11), 0x12, 0x88);
    alu(&[(0x10, 0xcc)], Opcode::ANDI(0x12, 0x10, 0xaa), 0x12, 0x88);
}

#[test]
fn div() {
    alu(&[(0x10, 59), (0x11, 10)], Opcode::DIV(0x12, 0x10, 0x11), 0x12, 5);
    alu(&[(0x10, 59)], Opcode::DIVI(0x12, 0x10, 10), 0x12, 5);
    alu_err(&[], Opcode::DIVI(0x10, REG_ONE, REG_ZERO as Immediate12), 0x10, 0x00);
}

#[test]
fn eq() {
    alu(&[(0x10, 10), (0x11, 10)], Opcode::EQ(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 11), (0x11, 10)], Opcode::EQ(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn exp() {
    // EXP
    alu(&[(0x10, 6), (0x11, 3)], Opcode::EXP(0x12, 0x10, 0x11), 0x12, 216);
    alu_overflow(
        &[
            Opcode::MOVI(0x10, 2),
            Opcode::MOVI(0x11, 64),
            Opcode::EXP(0x10, 0x10, 0x11),
        ],
        0x10,
        true as u128,
        true,
    );
    alu_wrapping(
        &[(0x10, 2), (0x11, 32)],
        Opcode::EXP(0x10, 0x10, 0x11),
        0x10,
        2u64.pow(32),
        false,
    );
    alu_wrapping(&[(0x10, 2), (0x11, 64)], Opcode::EXP(0x10, 0x10, 0x11), 0x10, 0, true);

    // EXPI
    alu(&[(0x10, 6)], Opcode::EXPI(0x12, 0x10, 3), 0x12, 216);
    alu_overflow(
        &[Opcode::MOVI(0x10, 2), Opcode::EXPI(0x10, 0x10, 64)],
        0x10,
        true as u128,
        true,
    );
    alu_wrapping(&[(0x10, 2)], Opcode::EXPI(0x10, 0x10, 32), 0x10, 2u64.pow(32), false);
    alu_wrapping(&[(0x10, 2)], Opcode::EXPI(0x10, 0x10, 64), 0x10, 0, true);
}

#[test]
fn mlog() {
    alu(&[(0x10, 1), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 10), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 100), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 2);
    alu(&[(0x10, 999), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 2);
    alu(&[(0x10, 1000), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 3);
    alu(&[(0x10, 1001), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 3);

    alu(&[(0x10, 1), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 2), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 4), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 2);
    
    alu(&[(0x10, 2u64.pow(32)), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 32);
    alu(&[(0x10, Word::MAX), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 63);
    alu(&[(0x10, 10u64.pow(10)), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 10);
    alu(&[(0x10, 10u64.pow(11)), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 11);

    alu_err(&[(0x10, 0), (0x11, 10)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 0);
    alu_err(&[(0x10, 0), (0x11, 2)], Opcode::MLOG(0x12, 0x10, 0x11), 0x12, 0);
}

#[test]
fn gt() {
    alu(&[(0x10, 6), (0x11, 3)], Opcode::GT(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 3)], Opcode::GT(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 1), (0x11, 3)], Opcode::GT(0x12, 0x10, 0x11), 0x12, 0);
}
