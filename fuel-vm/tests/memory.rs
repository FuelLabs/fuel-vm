use fuel_vm::consts::*;
use fuel_vm::prelude::*;

fn setup(ops: Vec<Opcode>) -> Transactor<MemoryStorage, Script> {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let script = ops.into_iter().collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params)
        .expect("failed to check tx");

    let mut vm = Transactor::new(storage, Default::default(), Default::default());
    vm.transact(tx);
    vm
}

#[test]
fn test_lw() {
    let ops = vec![
        Opcode::MOVI(0x10, 8),
        Opcode::MOVI(0x11, 1),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x10, REG_HP, 1),
        Opcode::SW(0x10, 0x11, 0),
        Opcode::LW(0x13, 0x10, 0),
        Opcode::RET(REG_ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize];
    assert_eq!(1, result);
}

#[test]
fn test_lb() {
    let ops = vec![
        Opcode::MOVI(0x10, 8),
        Opcode::MOVI(0x11, 1),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x10, REG_HP, 1),
        Opcode::SB(0x10, 0x11, 0),
        Opcode::LB(0x13, 0x10, 0),
        Opcode::RET(REG_ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize] as u8;
    assert_eq!(1, result);
}
