use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn external_balance() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let script = vec![Opcode::RET(0x01)].iter().copied().collect();
    let balances = vec![(Color::random(&mut rng), 100), (Color::random(&mut rng), 500)];

    let inputs = balances
        .iter()
        .map(|(color, amount)| {
            Input::coin(
                Bytes32::random(&mut rng),
                Address::random(&mut rng),
                *amount,
                *color,
                0,
                maturity,
                vec![],
                vec![],
            )
        })
        .collect();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        vec![],
        inputs,
        vec![],
        vec![vec![].into()],
    );

    vm.init(tx).expect("Failed to init VM!");

    for (color, amount) in balances {
        assert!(vm.external_color_balance_sub(&color, amount + 1).is_err());
        vm.external_color_balance_sub(&color, amount - 10).unwrap();
        assert!(vm.external_color_balance_sub(&color, 11).is_err());
        vm.external_color_balance_sub(&color, 10).unwrap();
        assert!(vm.external_color_balance_sub(&color, 1).is_err());
    }
}
