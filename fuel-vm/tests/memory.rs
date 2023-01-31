use fuel_asm::op;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;

fn setup(program: Vec<Instruction>) -> Transactor<MemoryStorage, Script> {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let script = program.into_iter().collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let mut vm = Transactor::new(storage, Default::default(), Default::default());
    vm.transact(tx);
    vm
}

#[test]
fn test_lw() {
    let ops = vec![
        op::movi(0x10, 8),
        op::movi(0x11, 1),
        op::aloc(0x10),
        op::addi(0x10, REG_HP.into(), 1),
        op::sw(0x10, 0x11, 0),
        op::lw(0x13, 0x10, 0),
        op::ret(REG_ONE.into()),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize];
    assert_eq!(1, result);
}

#[test]
fn test_lb() {
    let ops = vec![
        op::movi(0x10, 8),
        op::movi(0x11, 1),
        op::aloc(0x10),
        op::addi(0x10, REG_HP.into(), 1),
        op::sb(0x10, 0x11, 0),
        op::lb(0x13, 0x10, 0),
        op::ret(REG_ONE.into()),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize] as u8;
    assert_eq!(1, result);
}
