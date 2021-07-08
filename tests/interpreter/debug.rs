use fuel_vm::consts::*;
use fuel_vm::prelude::*;

#[test]
fn breakpoint_script() {
    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let script = vec![
        Opcode::ADDI(0x10, REG_ZERO, 8),
        Opcode::ADDI(0x11, REG_ZERO, 16),
        Opcode::ADDI(0x12, REG_ZERO, 32),
        Opcode::ADDI(0x13, REG_ZERO, 64),
        Opcode::ADDI(0x14, REG_ZERO, 128),
        Opcode::RET(0x10),
    ]
    .iter()
    .copied()
    .collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![]);

    vm.init(tx).expect("Failed to init VM!");

    let suite = vec![
        (
            Breakpoint::script(0),
            vec![(0x10, 0), (0x11, 0), (0x12, 0), (0x13, 0), (0x14, 0)],
        ),
        (
            Breakpoint::script(2),
            vec![(0x10, 8), (0x11, 16), (0x12, 0), (0x13, 0), (0x14, 0)],
        ),
        (
            Breakpoint::script(3),
            vec![(0x10, 8), (0x11, 16), (0x12, 32), (0x13, 0), (0x14, 0)],
        ),
        (
            Breakpoint::script(5),
            vec![(0x10, 8), (0x11, 16), (0x12, 32), (0x13, 64), (0x14, 128)],
        ),
    ];

    suite.iter().for_each(|(b, _)| vm.set_breakpoint(*b));
    let state = vm.run().expect("Failed to execute script!");

    suite.into_iter().fold(state, |state, (breakpoint, registers)| {
        let debug = state.debug_ref().expect("Expected breakpoint");
        let b = debug.breakpoint().expect("State without expected breakpoint");

        assert_eq!(&breakpoint, b);
        registers.into_iter().for_each(|(r, w)| {
            assert_eq!(w, vm.registers()[r]);
        });

        vm.resume().expect("Failed to resume")
    });
}
