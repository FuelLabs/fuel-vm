use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::TransactionBuilder;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

use crate::{
    checked_transaction::{
        CheckPredicateParams,
        CheckPredicates,
        ConsensusParams,
    },
    interpreter::CheckedMetadata,
    prelude::*,
};

#[test]
fn estimate_gas_gives_proper_gas_used() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let params = CheckPredicateParams::default();

    let gas_price = 1_000;
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
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        rng.gen(),
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
    );

    let transaction_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(params.clone())
        .expect("Predicate check failed even if we don't have any predicates");

    let mut client = MemoryClient::default();

    client.transact(transaction_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();
    let gas_without_predicate = receipts_without_predicate[1]
        .gas_used()
        .expect("Should retrieve gas used");

    builder.gas_limit(gas_without_predicate);

    let predicate: Vec<u8> = vec![op::addi(0x20, 0x20, 1), op::ret(RegId::ONE)]
        .into_iter()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect();
    let owner = Input::predicate_owner(&predicate, &params.chain_id);
    let input = Input::coin_predicate(
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
        rng.gen(),
        predicate,
        vec![],
    );

    builder.add_input(input);

    let mut transaction = builder.finalize();

    // unestimated transaction should fail as it's predicates are not estimated
    assert!(transaction
        .clone()
        .into_checked(
            Default::default(),
            ConsensusParams::standard(),
            Default::default(),
            GasCosts::default(),
        )
        .is_err());

    // create checked transaction to get access to balances from metadata
    let unestimated_checked = transaction
        .clone()
        .into_checked_basic(
            Default::default(),
            ConsensusParams::standard(),
            &Default::default(),
        )
        .expect("Should successfully create checked tranaction with predicate");

    let balances = unestimated_checked.metadata().balances();

    Interpreter::<PredicateStorage>::estimate_predicates(
        &mut transaction,
        balances,
        params,
    )
    .expect("Should successfully estimate predicates");

    // transaction should pass checking after estimation

    let check_res = transaction.into_checked(
        Default::default(),
        ConsensusParams::standard(),
        Default::default(),
        GasCosts::default(),
    );
    dbg!(&check_res);
    assert!(check_res.is_ok());
}
