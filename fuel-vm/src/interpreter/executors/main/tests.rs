#![allow(non_snake_case)]

use crate::{
    checked_transaction::{
        CheckPredicates,
        Checked,
    },
    interpreter::{
        MemoryInstance,
        NotSupportedEcal,
    },
    prelude::{
        predicates::estimate_predicates,
        *,
    },
    storage::predicate::EmptyStorage,
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    TransactionBuilder,
};
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
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
        rng.r#gen(),
        coin_amount,
        AssetId::default(),
        rng.r#gen(),
    );

    let transaction_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(
            &params.into(),
            MemoryInstance::new(),
            &EmptyStorage,
            NotSupportedEcal,
        )
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
        rng.r#gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.r#gen(),
        0,
        predicate,
        vec![],
    );

    builder.add_input(input);

    let mut transaction = builder.finalize();

    // unestimated transaction should fail as it's predicates are not estimated
    assert!(
        transaction
            .clone()
            .into_checked(Default::default(), params)
            .is_err()
    );

    estimate_predicates(
        &mut transaction,
        &params.into(),
        MemoryInstance::new(),
        &EmptyStorage,
        NotSupportedEcal,
    )
    .expect("Should successfully estimate predicates");

    // transaction should pass checking after estimation

    let check_res = transaction.into_checked(Default::default(), params);
    assert!(check_res.is_ok());
}

fn valid_script_tx() -> Checked<Script> {
    let input_amount = 1000;
    let arb_max_fee = input_amount;

    TransactionBuilder::script(vec![], vec![])
        .max_fee_limit(arb_max_fee)
        .add_fee_input()
        .finalize_checked_basic(Default::default())
}

#[test]
fn transact__tx_with_wrong_gas_price_causes_error() {
    let mut interpreter = Interpreter::<_, _, Script>::with_memory_storage();

    // Given
    let tx_gas_price = 1;
    let interpreter_gas_price = 2;
    interpreter.set_gas_price(interpreter_gas_price);

    // When
    let tx = valid_script_tx()
        .into_ready(tx_gas_price, &Default::default(), &Default::default(), None)
        .unwrap();
    let err = interpreter.transact(tx).unwrap_err();

    // Then
    assert!(matches!(
        err,
        InterpreterError::ReadyTransactionWrongGasPrice { .. }
    ));
}

fn valid_create_tx() -> Checked<Create> {
    let input_amount = 1000;
    let arb_max_fee = input_amount;
    let witness = Witness::default();
    let salt = [123; 32].into();

    TransactionBuilder::create(witness, salt, vec![])
        .max_fee_limit(arb_max_fee)
        .add_fee_input()
        .add_contract_created()
        .finalize_checked_basic(Default::default())
}

#[test]
fn deploy__tx_with_wrong_gas_price_causes_error() {
    let mut interpreter = Interpreter::<_, _, Create>::with_memory_storage();

    // Given
    let tx_gas_price = 1;
    let interpreter_gas_price = 2;
    interpreter.set_gas_price(interpreter_gas_price);

    // When
    let tx = valid_create_tx()
        .into_ready(tx_gas_price, &Default::default(), &Default::default(), None)
        .unwrap();
    let err = interpreter.deploy(tx).unwrap_err();

    // Then
    assert!(matches!(
        err,
        InterpreterError::ReadyTransactionWrongGasPrice { .. }
    ));
}

fn valid_upgrade_tx() -> Checked<Upgrade> {
    let input_amount = 1000;
    let arb_max_fee = input_amount;
    TransactionBuilder::upgrade(UpgradePurpose::StateTransition {
        root: Default::default(),
    })
    .max_fee_limit(arb_max_fee)
    .add_input(Input::coin_signed(
        Default::default(),
        *ConsensusParameters::standard().privileged_address(),
        input_amount,
        AssetId::BASE,
        Default::default(),
        0,
    ))
    .add_fee_input()
    .finalize_checked_basic(Default::default())
}

#[test]
fn upgrade__tx_with_wrong_gas_price_causes_error() {
    let mut interpreter = Interpreter::<_, _, Upgrade>::with_memory_storage();

    // Given
    let tx_gas_price = 1;
    let interpreter_gas_price = 2;
    interpreter.set_gas_price(interpreter_gas_price);

    // When
    let tx = valid_upgrade_tx()
        .into_ready(tx_gas_price, &Default::default(), &Default::default(), None)
        .unwrap();
    let err = interpreter.upgrade(tx).unwrap_err();

    // Then
    assert!(matches!(
        err,
        InterpreterError::ReadyTransactionWrongGasPrice { .. }
    ));
}

fn valid_upload_tx() -> Checked<Upload> {
    let input_amount = 1000;
    let arb_max_fee = input_amount;
    let subsections = UploadSubsection::split_bytecode(&vec![123; 1024], 24)
        .expect("Should split bytecode");
    let subsection = subsections[0].clone();
    TransactionBuilder::upload(UploadBody {
        root: subsection.root,
        witness_index: 0,
        subsection_index: subsection.subsection_index,
        subsections_number: subsection.subsections_number,
        proof_set: subsection.proof_set,
    })
    .add_witness(subsection.subsection.into())
    .max_fee_limit(arb_max_fee)
    .add_fee_input()
    .finalize_checked_basic(Default::default())
}

#[test]
fn upload__tx_with_wrong_gas_price_causes_error() {
    let mut interpreter = Interpreter::<_, _, Upload>::with_memory_storage();

    // Given
    let tx_gas_price = 1;
    let interpreter_gas_price = 2;
    interpreter.set_gas_price(interpreter_gas_price);

    // When
    let tx = valid_upload_tx()
        .into_ready(tx_gas_price, &Default::default(), &Default::default(), None)
        .unwrap();
    let err = interpreter.upload(tx).unwrap_err();

    // Then
    assert!(matches!(
        err,
        InterpreterError::ReadyTransactionWrongGasPrice { .. }
    ));
}
