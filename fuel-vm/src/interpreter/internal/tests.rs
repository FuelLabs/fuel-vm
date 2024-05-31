use alloc::vec;

use crate::{
    constraints::reg_key::RegMut,
    interpreter::{
        internal::{
            external_asset_id_balance_sub,
            set_variable_output,
        },
        InterpreterParams,
    },
    prelude::*,
};
use fuel_asm::op;
use fuel_tx::{
    field::Outputs,
    ConsensusParameters,
    TransactionBuilder,
};
use fuel_types::canonical::Deserialize;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

use super::inc_pc;

#[test]
fn external_balance() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let mut vm = Interpreter::<_, _>::with_memory_storage();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    let script = op::ret(0x01).to_bytes().to_vec();
    let balances = vec![(rng.gen(), 100), (rng.gen(), 500)];

    let mut tx = TransactionBuilder::script(script, Default::default());

    balances.iter().copied().for_each(|(asset, amount)| {
        tx.add_unsigned_coin_input(
            SecretKey::random(&mut rng),
            rng.gen(),
            amount,
            asset,
            rng.gen(),
            maturity,
        );
    });

    let tx = tx
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .script_gas_limit(100)
        .maturity(maturity)
        .finalize_checked(height);

    vm.init_script(tx).expect("Failed to init VM!");

    for (asset_id, amount) in balances {
        assert!(external_asset_id_balance_sub(
            &mut vm.balances,
            &mut vm.memory,
            &asset_id,
            amount + 1
        )
        .is_err());
        external_asset_id_balance_sub(
            &mut vm.balances,
            &mut vm.memory,
            &asset_id,
            amount - 10,
        )
        .unwrap();
        assert!(external_asset_id_balance_sub(
            &mut vm.balances,
            &mut vm.memory,
            &asset_id,
            11
        )
        .is_err());
        external_asset_id_balance_sub(&mut vm.balances, &mut vm.memory, &asset_id, 10)
            .unwrap();
        assert!(external_asset_id_balance_sub(
            &mut vm.balances,
            &mut vm.memory,
            &asset_id,
            1
        )
        .is_err());
    }
}

#[test]
fn variable_output_updates_in_memory() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let consensus_params = ConsensusParameters::standard();
    let mut vm = Interpreter::<_, _>::with_storage(
        MemoryStorage::default(),
        InterpreterParams::from(&consensus_params),
    );

    let gas_limit = 1_000_000;
    let height = Default::default();
    let asset_id_to_update: AssetId = rng.gen();
    let amount_to_set: Word = 100;
    let owner: Address = rng.gen();

    let variable_output = Output::Variable {
        to: rng.gen(),
        amount: 0,
        asset_id: rng.gen(),
    };

    let tx = TransactionBuilder::script(vec![], vec![])
        .script_gas_limit(gas_limit)
        .add_random_fee_input()
        .add_output(variable_output)
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    vm.init_script(tx).expect("Failed to init VM!");

    // increase variable output
    let variable = Output::variable(owner, amount_to_set, asset_id_to_update);
    let tx_offset = vm.tx_offset();

    set_variable_output(&mut vm.tx, &mut vm.memory, tx_offset, 0, variable).unwrap();

    // verify the referenced tx output is updated properly
    assert!(matches!(
        vm.transaction().outputs()[0],
        Output::Variable {amount, asset_id, to} if amount == amount_to_set
                                                && asset_id == asset_id_to_update
                                                && to == owner
    ));

    // verify the vm memory is updated properly
    let position = vm.tx_offset() + vm.transaction().outputs_offset_at(0).unwrap();

    let mem_output = Output::decode(&mut &vm.memory()[position..]).unwrap();
    assert_eq!(vm.transaction().outputs()[0], mem_output);
}

#[test]
fn test_inc_pc_errors_on_of() {
    let mut pc = Word::MAX - 4;
    inc_pc(RegMut::new(&mut pc)).unwrap();
    inc_pc(RegMut::new(&mut pc)).expect_err("Expected overflow error");
}
