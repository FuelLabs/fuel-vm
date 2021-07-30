use fuel_vm::consts::*;
use fuel_vm::prelude::*;

fn alu(registers_init: &[(RegisterId, Immediate12)], op: Opcode, reg: RegisterId, expected: Word) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let script = registers_init
        .iter()
        .map(|(r, v)| Opcode::ADDI(*r, REG_ZERO, *v))
        .chain([op, Opcode::LOG(reg, 0, 0, 0), Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![]);
    let state = Interpreter::transition(storage, tx).expect("Failed to execute ALU script!");

    assert!(
        matches!(state.log().first(), Some(LogEvent::Register { register, value, .. }) if *register == reg && *value == expected)
    );
}

fn alu_err(registers_init: &[(RegisterId, Immediate12)], op: Opcode) {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let script = registers_init
        .iter()
        .map(|(r, v)| Opcode::ADDI(*r, REG_ZERO, *v))
        .chain([op, Opcode::RET(REG_ONE)].iter().copied())
        .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![]);
    let result = Interpreter::transition(storage, tx);

    assert!(matches!(result, Err(ExecuteError::OpcodeFailure(o)) if o == op));
}

#[test]
fn reserved_register() {
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_ZERO, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_ONE, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_OF, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_PC, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_SSP, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_SP, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_FP, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_HP, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_ERR, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_GGAS, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_CGAS, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_BAL, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_IS, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_RET, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_RETL, 0x10, 0x11));
    alu_err(&[(0x10, 128)], Opcode::ADD(REG_FLAG, 0x10, 0x11));
}

#[test]
fn add() {
    alu(&[(0x10, 128), (0x11, 25)], Opcode::ADD(0x12, 0x10, 0x11), 0x12, 153);
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
