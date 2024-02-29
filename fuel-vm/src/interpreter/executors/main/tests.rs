#![allow(non_snake_case)]

use alloc::{
    vec,
    vec::Vec,
};

use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    ConsensusParameters,
    TransactionBuilder,
};
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

use crate::{
    checked_transaction::CheckPredicates,
    interpreter::InterpreterParams,
    prelude::*,
};

#[test]
fn estimate_gas_gives_proper_gas_used() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let params = &ConsensusParameters::standard();

    let gas_limit = 1_000_000;
    let script = vec![
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    builder
        .script_gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
    );

    let transaction_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params.into())
        .expect("Predicate check failed even if we don't have any predicates");

    let mut client = MemoryClient::default();

    client.transact(transaction_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();
    let gas_without_predicate = receipts_without_predicate[1]
        .gas_used()
        .expect("Should retrieve gas used");

    builder.script_gas_limit(gas_without_predicate);

    let predicate: Vec<u8> = vec![op::addi(0x20, 0x20, 1), op::ret(RegId::ONE)]
        .into_iter()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect();
    let owner = Input::predicate_owner(&predicate);
    let input = Input::coin_predicate(
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
        rng.gen(),
        predicate,
        vec![],
    );

    builder.add_input(input);

    let mut transaction = builder.finalize();

    // unestimated transaction should fail as it's predicates are not estimated
    assert!(transaction
        .clone()
        .into_checked(Default::default(), params)
        .is_err());

    Interpreter::<PredicateStorage, _>::estimate_predicates(
        &mut transaction,
        &params.into(),
    )
    .expect("Should successfully estimate predicates");

    // transaction should pass checking after estimation

    let check_res = transaction.into_checked(Default::default(), params);
    assert!(check_res.is_ok());
}

#[test]
fn transact__tx_with_wrong_gas_price_causes_error() {
    let mut rng = &mut StdRng::seed_from_u64(2322u64);

    // given
    let tx_gas_price = 1;
    let interpreter_gas_price = 2;
    let input_amount = 1000;
    let arb_max_fee = input_amount;

    let interpreter_params = InterpreterParams {
        gas_price: interpreter_gas_price,
        ..Default::default()
    };

    let ready_tx = TransactionBuilder::script(vec![], vec![])
        .max_fee_limit(arb_max_fee)
        .add_unsigned_coin_input(
            SecretKey::random(&mut rng),
            rng.gen(),
            input_amount,
            AssetId::default(),
            rng.gen(),
        )
        .finalize_checked_basic(Default::default())
        .into_ready(
            tx_gas_price,
            &interpreter_params.gas_costs,
            &interpreter_params.fee_params,
        )
        .unwrap();

    // when
    let mut transactor =
        Transactor::<_, _>::new(MemoryStorage::default(), interpreter_params);
    transactor.transact_ready_tx(ready_tx);

    // then
    let err = transactor.error().expect("Expected error");
    assert!(matches!(
        *err,
        InterpreterError::ReadyTransactionWrongGasPrice { .. }
    ));
}
