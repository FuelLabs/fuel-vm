#![allow(clippy::cast_possible_truncation)]
use super::{
    test_params,
    CHAIN_ID,
    CONTRACT_PARAMS,
    SCRIPT_PARAMS,
    TX_PARAMS,
};
use crate::{
    policies::Policies,
    *,
};
use core::cmp;
use fuel_crypto::{
    SecretKey,
    Signature,
};
use fuel_tx_test_helpers::generate_bytes;
use fuel_types::canonical::{
    Deserialize,
    Serialize,
};
use rand::{
    rngs::StdRng,
    Rng,
    RngCore,
    SeedableRng,
};

#[test]
fn gas_limit() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let arb_gas_price = 1;

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate transaction");

    TransactionBuilder::create(vec![0xfau8].into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate transaction");

    let err = Transaction::script(
        TX_PARAMS.max_gas_per_tx + 1,
        generate_bytes(rng),
        generate_bytes(rng),
        Policies::new(),
        vec![],
        vec![],
        vec![],
    )
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionMaxGasExceeded, err);
}

#[test]
fn maturity() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let arb_gas_price = 1;

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(block_height)
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate script");

    TransactionBuilder::create(rng.gen(), rng.gen(), vec![])
        .maturity(block_height)
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate tx create");

    let err = Transaction::script(
        Default::default(),
        vec![],
        vec![],
        Policies::new().with_maturity(1001.into()),
        vec![],
        vec![],
        vec![],
    )
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionMaturity, err);

    let err = Transaction::create(
        0,
        Policies::new().with_maturity(1001.into()),
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![rng.gen()],
    )
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionMaturity, err);
}

#[test]
fn script_not_set_witness_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let arb_gas_price = 1;

    // When
    let result = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price);

    // Then
    assert!(result.is_ok());
}

#[test]
fn create_not_set_witness_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let bytecode = vec![];
    let arb_gas_price = 1;

    // When
    let result = TransactionBuilder::create(bytecode.clone().into(), rng.gen(), vec![])
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price);

    // Then
    assert!(result.is_ok());
}

#[test]
fn script_set_witness_limit_for_empty_witness_success() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    let arb_gas_price = 1;
    // Given
    let limit = Signature::LEN /* witness from random fee */ + vec![0u8; 0].size_static();

    // When
    let result = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_random_fee_input()
        .witness_limit(limit as u64)
        .finalize()
        .check(block_height, &test_params(), arb_gas_price);

    // Then
    assert!(result.is_ok());
}

#[test]
fn script_set_witness_limit_less_than_witness_data_size_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let witness_size = Signature::LEN + vec![0u8; 0].size_static();
    let arb_gas_price = 1;

    // Given
    let limit = witness_size - 1;

    // When
    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_random_fee_input()
        .witness_limit(limit as u64)
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionWitnessLimitExceeded, err);
}

#[test]
fn create_set_witness_limit_for_empty_witness_success() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let bytecode = vec![];
    let arb_gas_price = 1;
    // Given
    let limit = Signature::LEN /* witness from random fee */ + bytecode.size_static() + bytecode.size_static();

    // When
    let result = TransactionBuilder::create(bytecode.clone().into(), rng.gen(), vec![])
        .add_random_fee_input()
        .witness_limit(limit as u64)
        .finalize()
        .check(block_height, &test_params(), arb_gas_price);

    // Then
    assert!(result.is_ok());
}

#[test]
fn create_set_witness_limit_less_than_witness_data_size_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let bytecode = vec![];
    let arb_gas_price = 1;
    // Given
    let limit = Signature::LEN /* witness from random fee */ + bytecode.size_static() + bytecode.size_static();

    // When
    let err = TransactionBuilder::create(bytecode.clone().into(), rng.gen(), vec![])
        .add_random_fee_input()
        .witness_limit(limit as u64 - 1)
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionWitnessLimitExceeded, err);
}

#[test]
fn script_not_set_max_fee_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let arb_gas_price = 1;

    // When
    let result = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price);

    // Then
    assert!(result.is_ok());
}

#[test]
fn script_set_max_fee_limit_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    let arb_gas_price = 1;
    // Given
    let max_fee = 0;

    // When
    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .max_fee_limit(max_fee)
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionMaxFeeLimitExceeded, err);
}

#[test]
fn create_not_set_max_fee_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    let arb_gas_price = 1;

    // When
    let result = TransactionBuilder::create(rng.gen(), rng.gen(), vec![])
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price);

    // Then
    assert!(result.is_ok());
}

#[test]
fn create_set_max_fee_limit_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    let arb_gas_price = 1;
    // Given
    let max_fee = 0;

    // When
    let err = TransactionBuilder::create(rng.gen(), rng.gen(), vec![])
        .max_fee_limit(max_fee)
        .add_random_fee_input()
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionMaxFeeLimitExceeded, err);
}

#[test]
fn max_iow() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let arb_gas_price = 1;

    let secret = SecretKey::random(rng);

    let mut builder =
        TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng));

    let asset_id: AssetId = rng.gen();

    builder.maturity(maturity).add_unsigned_coin_input(
        secret,
        rng.gen(),
        rng.gen(),
        asset_id,
        rng.gen(),
        maturity,
    );

    while builder.outputs().len() < TX_PARAMS.max_outputs as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), asset_id));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    builder
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate transaction");

    // Add inputs up to maximum and validate
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![]);

    builder.maturity(maturity);

    let secrets =
        cmp::min(TX_PARAMS.max_inputs as u32, TX_PARAMS.max_witnesses - 1) as usize;
    let secrets: Vec<SecretKey> = (0..secrets - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    let asset_id: AssetId = rng.gen();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            asset_id,
            rng.gen(),
            maturity,
        );
    });

    while builder.outputs().len() < TX_PARAMS.max_outputs as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), asset_id));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    builder
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate transaction");

    // Overflow maximum inputs and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![]);

    builder.maturity(maturity);

    let secrets: Vec<SecretKey> = (0..1 + TX_PARAMS.max_inputs as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        );
    });

    while builder.outputs().len() < TX_PARAMS.max_outputs as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), rng.gen()));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionInputsMax, err);

    // Overflow outputs maximum and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![]);

    builder.maturity(maturity);

    let secrets: Vec<SecretKey> = (0..TX_PARAMS.max_inputs as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        );
    });

    while builder.outputs().len() < 1 + TX_PARAMS.max_outputs as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), rng.gen()));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionOutputsMax, err);

    // Overflow witnesses maximum and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![]);

    builder.maturity(maturity);

    let secrets: Vec<SecretKey> = (0..TX_PARAMS.max_inputs as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        );
    });

    while builder.outputs().len() < TX_PARAMS.max_outputs as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), rng.gen()));
    }

    while builder.witnesses().len() < 1 + TX_PARAMS.max_witnesses as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionWitnessesMax, err);
}

#[test]
fn output_change_asset_id() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let arb_gas_price = 1;

    let maturity = 100.into();
    let block_height = 1000.into();

    let a: AssetId = rng.gen();
    let b: AssetId = rng.gen();
    let c: AssetId = rng.gen();

    let secret = SecretKey::random(rng);

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), a, rng.gen(), rng.gen())
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), b, rng.gen(), rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), b))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate transaction");

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), a, rng.gen(), rng.gen())
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), b, rng.gen(), rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        ValidityError::TransactionOutputChangeAssetIdDuplicated(a),
        err
    );

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), a, rng.gen(), rng.gen())
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), b, rng.gen(), rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), c))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert!(matches!(
        err,
        ValidityError::TransactionOutputChangeAssetIdNotFound(asset_id) if asset_id == c
    ));

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), a, rng.gen(), rng.gen())
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), b, rng.gen(), rng.gen())
        .add_output(Output::coin(rng.gen(), rng.next_u64(), a))
        .add_output(Output::coin(rng.gen(), rng.next_u64(), c))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert!(matches!(
        err,
        ValidityError::TransactionOutputCoinAssetIdNotFound(asset_id) if asset_id == c
    ));
}

#[test]
fn script() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let arb_gas_price = 1;

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.gen();

    TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), asset_id, rng.gen(), rng.gen())
    .add_output(Output::change(rng.gen(), rng.gen(), asset_id))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect("Failed to validate transaction");

    let err = TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), asset_id, rng.gen(), rng.gen())
    .add_output(Output::contract_created(rng.gen(), rng.gen()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(
        ValidityError::TransactionScriptOutputContractCreated { index: 0 },
        err
    );

    let err = TransactionBuilder::script(
        vec![0xfa; 1 + SCRIPT_PARAMS.max_script_length as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), asset_id, rng.gen(), rng.gen())
    .add_output(Output::contract_created(rng.gen(), rng.gen()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionScriptLength, err);

    let err = TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length as usize],
        vec![0xfb; 1 + SCRIPT_PARAMS.max_script_data_length as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), asset_id, rng.gen(), rng.gen())
    .add_output(Output::contract_created(rng.gen(), rng.gen()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionScriptDataLength, err);
}

#[test]
fn create() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let arb_gas_price = 1;

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let secret_b = SecretKey::random(rng);

    TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate tx");

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_input(Input::contract(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ))
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .add_output(Output::contract(0, rng.gen(), rng.gen()))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionCreateInputContract { index: 0 }
    );

    let not_empty_data = vec![0x1];
    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_unsigned_message_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            not_empty_data,
        )
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionCreateMessageData { index: 0 }
    );

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .add_output(Output::variable(rng.gen(), rng.gen(), rng.gen()))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionCreateOutputVariable { index: 0 }
    );

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            AssetId::default(),
            rng.gen(),
            maturity,
        )
        .add_unsigned_coin_input(
            secret_b,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::BASE))
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::BASE))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionOutputChangeAssetIdDuplicated(AssetId::BASE)
    );

    let asset_id: AssetId = rng.gen();

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            AssetId::default(),
            rng.gen(),
            maturity,
        )
        .add_unsigned_coin_input(
            secret_b,
            rng.gen(),
            rng.gen(),
            asset_id,
            rng.gen(),
            maturity,
        )
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
        .add_output(Output::change(rng.gen(), rng.gen(), asset_id))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionCreateOutputChangeNotBaseAsset { index: 1 },
    );

    let witness = generate_bytes(rng);
    let contract = Contract::from(witness.as_ref());
    let salt = rng.gen();
    let storage_slots: Vec<StorageSlot> = vec![];
    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id = contract.id(&salt, &contract.root(), &state_root);

    let err = TransactionBuilder::create(witness.into(), salt, storage_slots)
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            AssetId::default(),
            rng.gen(),
            maturity,
        )
        .add_unsigned_coin_input(
            secret_b,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .add_output(Output::contract_created(contract_id, state_root))
        .add_output(Output::contract_created(contract_id, state_root))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionCreateOutputContractCreatedMultiple { index: 1 },
    );

    TransactionBuilder::create(
        vec![0xfa; CONTRACT_PARAMS.contract_max_size as usize / 4].into(),
        rng.gen(),
        vec![],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.gen(),
        rng.gen(),
        AssetId::default(),
        rng.gen(),
        maturity,
    )
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect("Failed to validate the transaction");

    let err = TransactionBuilder::create(
        vec![0xfa; 1 + CONTRACT_PARAMS.contract_max_size as usize].into(),
        rng.gen(),
        vec![],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.gen(),
        rng.gen(),
        AssetId::default(),
        rng.gen(),
        maturity,
    )
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionCreateBytecodeLen);

    let err = Transaction::create(
        1,
        Default::default(),
        rng.gen(),
        vec![],
        vec![Input::coin_signed(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            0,
            rng.gen(),
        )],
        vec![],
        vec![Default::default()],
    )
    .check_without_signatures(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionCreateBytecodeWitnessIndex);

    TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            AssetId::default(),
            rng.gen(),
            maturity,
        )
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
        .finalize()
        .check(block_height, &test_params(), arb_gas_price)
        .expect("Failed to validate the transaction");

    let storage_slots = (0..CONTRACT_PARAMS.max_storage_slots)
        .map(|i| {
            let mut slot_data = StorageSlot::default().to_bytes();
            slot_data[..8].copy_from_slice(&i.to_be_bytes()); // Force ordering
            StorageSlot::from_bytes(&slot_data).unwrap()
        })
        .collect::<Vec<StorageSlot>>();

    // Test max slots is valid
    TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        storage_slots.clone(),
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.gen(),
        rng.gen(),
        AssetId::default(),
        rng.gen(),
        maturity,
    )
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect("Failed to validate the transaction");

    // Test max slots can't be exceeded
    let mut storage_slots_max = storage_slots;

    let s = StorageSlot::new([255u8; 32].into(), Default::default());
    storage_slots_max.push(s);

    let err = TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        storage_slots_max,
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.gen(),
        rng.gen(),
        AssetId::default(),
        rng.gen(),
        maturity,
    )
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionCreateStorageSlotMax, err);
}

#[test]
fn script_transaction_at_maximum_size_is_valid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let secret = SecretKey::random(rng);

    let maturity = 100.into();
    let block_height = 100.into();
    let mut params = test_params();
    let max_size = 1024usize;
    params.tx_params.max_size = max_size as u64;
    let arb_gas_price = 1;

    let base_size = {
        let tx = TransactionBuilder::script(vec![], vec![])
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                maturity,
            )
            .finalize();
        tx.size()
    };

    let script_size = max_size - base_size;

    let script = {
        let mut data = alloc::vec![0u8; script_size];
        rng.fill_bytes(data.as_mut_slice());
        data
    };
    let tx = TransactionBuilder::script(script, vec![])
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .finalize();

    tx.check(block_height, &params, arb_gas_price)
        .expect("Expected valid transaction");
}

#[test]
fn script_transaction_exceeding_maximum_size_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let secret = SecretKey::random(rng);

    let maturity = 100.into();
    let block_height = 100.into();
    let mut params = test_params();
    let max_size = 1024usize;
    let arb_gas_price = 1;
    params.tx_params.max_size = max_size as u64;

    let base_size = {
        let tx = TransactionBuilder::script(vec![], vec![])
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                maturity,
            )
            .finalize();
        tx.size()
    };

    let script_size = max_size - base_size;

    let script = {
        // Exceed the maximum size by 1 byte
        let script_size = script_size + 1;
        let mut data = alloc::vec![0u8; script_size];
        rng.fill_bytes(data.as_mut_slice());
        data
    };
    let tx = TransactionBuilder::script(script, vec![])
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        )
        .finalize();

    let err = tx
        .check(block_height, &params, arb_gas_price)
        .expect_err("Expected valid transaction");

    assert_eq!(err, ValidityError::TransactionSizeLimitExceeded);
}

#[test]
fn mint() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    let arb_gas_price = 1;

    let err = TransactionBuilder::mint(
        block_height,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
    )
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionMintIncorrectOutputIndex);

    let err = TransactionBuilder::mint(
        block_height,
        rng.gen(),
        rng.gen(),
        output::contract::Contract {
            input_index: 0,
            balance_root: rng.gen(),
            state_root: rng.gen(),
        },
        rng.gen(),
        rng.gen(),
        rng.gen(),
    )
    .finalize()
    .check(block_height, &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionMintNonBaseAsset);

    let err = TransactionBuilder::mint(
        block_height,
        rng.gen(),
        rng.gen(),
        output::contract::Contract {
            input_index: 0,
            balance_root: rng.gen(),
            state_root: rng.gen(),
        },
        rng.gen(),
        rng.gen(),
        rng.gen(),
    )
    .finalize()
    .check(block_height.succ().unwrap(), &test_params(), arb_gas_price)
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionMintIncorrectBlockHeight);
}

#[test]
fn tx_id_bytecode_len() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let salt = rng.gen();

    let w_a = vec![0xfau8; 4].into();
    let w_b = vec![0xfau8; 8].into();
    let w_c = vec![0xfbu8; 4].into();

    let tx_a = Transaction::create(
        0,
        Policies::new().with_maturity(maturity),
        salt,
        vec![],
        vec![],
        vec![],
        vec![w_a],
    );

    let tx_b = Transaction::create(
        0,
        Policies::new().with_maturity(maturity),
        salt,
        vec![],
        vec![],
        vec![],
        vec![w_b],
    );

    let tx_c = Transaction::create(
        0,
        Policies::new().with_maturity(maturity),
        salt,
        vec![],
        vec![],
        vec![],
        vec![w_c],
    );

    let id_a = tx_a.id(&CHAIN_ID);
    let id_b = tx_b.id(&CHAIN_ID);
    let id_c = tx_c.id(&CHAIN_ID);

    // bytecode with different length should produce different id
    assert_ne!(id_a, id_b);

    // bytecode with same length and different content should produce same id
    //
    // Note that this isn't related to the checkable itself - this checks exclusively the
    // id behavior. the witness payload for a bytecode cannot be tampered and the
    // checkable rules should not allow this case to pass.
    //
    // For further reference, check
    // https://github.com/FuelLabs/fuel-specs/blob/1856de801fabc7e52f5c010c45c3fc6d5d4e2be3/specs/protocol/tx_format.md?plain=1#L160
    assert_eq!(id_a, id_c);
}

mod inputs {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn coin_predicate_check_owner_works() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let predicate = (0..1000).map(|_| rng.gen()).collect_vec();
        // The predicate is an owner of the coin
        let owner: Address = Input::predicate_owner(&predicate);

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
                .maturity(rng.gen())
                .add_input(Input::coin_predicate(
                    rng.gen(),
                    owner,
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    predicate,
                    vec![],
                ))
                .with_tx_params(TX_PARAMS)
                .finalize();

        assert!(tx.check_predicate_owners());
    }

    #[test]
    fn coin_predicate_check_owners_fails_incorrect_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let predicate = (0..1000).map(|_| rng.gen()).collect_vec();

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
                .maturity(rng.gen())
                .add_input(Input::coin_predicate(
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    predicate,
                    vec![],
                ))
                .with_tx_params(TX_PARAMS)
                .finalize();

        assert!(!tx.check_predicate_owners());
    }

    #[test]
    fn message_predicate_check_owners_works() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let predicate = (0..1000).map(|_| rng.gen()).collect_vec();
        // The predicate is an recipient(owner) of the message
        let recipient: Address = Input::predicate_owner(&predicate);

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
                .maturity(rng.gen())
                .add_input(Input::message_data_predicate(
                    rng.gen(),
                    recipient,
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    vec![],
                    predicate,
                    vec![],
                ))
                .with_tx_params(TX_PARAMS)
                .finalize();

        assert!(tx.check_predicate_owners());
    }

    #[test]
    fn message_predicate_check_owners_fails_incorrect_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let predicate = (0..1000).map(|_| rng.gen()).collect_vec();

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
                .maturity(rng.gen())
                .add_input(Input::message_data_predicate(
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    vec![],
                    predicate,
                    vec![],
                ))
                .with_tx_params(TX_PARAMS)
                .finalize();

        assert!(!tx.check_predicate_owners());
    }
}
