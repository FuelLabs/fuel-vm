use fuel_vm::consts::*;
use fuel_vm::prelude::*;

fn alu(registers_init: &[(RegisterId, Immediate18)], op: Opcode, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let script = registers_init
        .iter()
        .map(|(r, v)| Opcode::MOVI(*r, *v))
        .chain([op, Opcode::LOG(reg, 0, 0, 0), Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    assert_eq!(
        receipts.first().expect("Receipt not found").ra().expect("$ra expected"),
        expected
    );
}

fn alu_overflow(program: &[Opcode], reg: RegisterId, expected: u128) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let script = program
        .iter()
        .copied()
        .chain([Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage.clone())
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

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    let lo_value = receipts.first().expect("Receipt not found").ra().expect("$ra expected");
    let hi_value = receipts.first().expect("Receipt not found").rb().expect("$rb expected");

    let overflow_value = lo_value as u128 + ((hi_value as u128) << 64);

    assert_eq!(overflow_value, expected);
}

fn alu_err(registers_init: &[(RegisterId, Immediate18)], op: Opcode, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let script = registers_init
        .iter()
        .map(|(r, v)| Opcode::MOVI(*r, *v))
        .chain([op, Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage.clone())
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

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage)
        .transact(tx)
        .receipts()
        .expect("Failed to execute ALU script!")
        .to_owned();

    assert_eq!(
        receipts.first().expect("Receipt not found").ra().expect("$ra expected"),
        expected
    );
}

fn alu_reserved(registers_init: &[(RegisterId, Immediate18)], op: Opcode) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let script = registers_init
        .iter()
        .map(|(r, v)| Opcode::MOVI(*r, *v))
        .chain([op, Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    let receipts = Transactor::new(storage)
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
    );
}

#[test]
fn sll() {
    alu(&[(0x10, 128), (0x11, 2)], Opcode::SLL(0x12, 0x10, 0x11), 0x12, 512);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::MOVI(0x11, 2),
            Opcode::NOT(0x10, 0x10),
            Opcode::SLL(0x10, 0x10, 0x11),
        ],
        0x10,
        (Word::MAX as u128) << 2,
    );
}

#[test]
fn slli() {
    alu(&[(0x10, 128)], Opcode::SLLI(0x11, 0x10, 2), 0x11, 512);
    alu_overflow(
        &[
            Opcode::MOVE(0x10, REG_ZERO),
            Opcode::NOT(0x10, 0x10),
            Opcode::SLLI(0x10, 0x10, 2),
        ],
        0x10,
        (Word::MAX as u128) << 2,
    );
}

#[test]
fn srl() {
    alu(&[(0x10, 128), (0x11, 2)], Opcode::SRL(0x12, 0x10, 0x11), 0x12, 32);
    // TODO: unsure what the expected overflow behavior is for shift right, seems like it shouldn't
    //       interact with REG_OF at all really
    // alu_overflow(
    //     &[
    //         Opcode::MOVE(0x10, REG_ZERO),
    //         Opcode::MOVI(0x11, 2),
    //         Opcode::SRL(0x10, 0x10, 0x11),
    //     ],
    //     0x10,
    //     (0 as u128).overflowing_shr(2).0,
    // );
}

#[test]
fn srli() {
    alu(&[(0x10, 128)], Opcode::SRLI(0x11, 0x10, 2), 0x11, 32);
    // TODO: unsure what the expected overflow behavior is for shift right, seems like it shouldn't
    //       interact with REG_OF at all really
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
    );
}

#[test]
fn subi() {
    alu(&[(0x10, 128)], Opcode::SUBI(0x11, 0x10, 25), 0x11, 103);
    alu_overflow(
        &[Opcode::MOVE(0x10, REG_ZERO), Opcode::SUBI(0x10, 0x10, 10)],
        0x10,
        (0 as u128).wrapping_sub(10),
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
    alu(&[(0x10, 6), (0x11, 3)], Opcode::EXP(0x12, 0x10, 0x11), 0x12, 216);
    alu(&[(0x10, 6)], Opcode::EXPI(0x12, 0x10, 3), 0x12, 216);
}

#[test]
fn gt() {
    alu(&[(0x10, 6), (0x11, 3)], Opcode::GT(0x12, 0x10, 0x11), 0x12, 1);
    alu(&[(0x10, 3), (0x11, 3)], Opcode::GT(0x12, 0x10, 0x11), 0x12, 0);
    alu(&[(0x10, 1), (0x11, 3)], Opcode::GT(0x12, 0x10, 0x11), 0x12, 0);
}
