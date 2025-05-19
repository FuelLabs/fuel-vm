#![allow(clippy::cast_possible_truncation)]
#![allow(non_snake_case)]

mod blob;
mod upgrade;
mod upload;

use super::{
    CONTRACT_PARAMS,
    SCRIPT_PARAMS,
    TX_PARAMS,
    test_params,
};
use crate::{
    policies::{
        Policies,
        PolicyType,
    },
    test_helper::generate_bytes,
    transaction::field::Policies as PoliciesField,
    *,
};
use core::cmp;
use fuel_crypto::{
    SecretKey,
    Signature,
};
use fuel_types::canonical::{
    Deserialize,
    Serialize,
};
use rand::{
    Rng,
    RngCore,
    SeedableRng,
    rngs::StdRng,
};

#[test]
fn gas_limit() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_fee_input()
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate transaction");

    TransactionBuilder::create(vec![0xfau8].into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate transaction");

    let err = Transaction::script(
        TX_PARAMS.max_gas_per_tx() + 1,
        generate_bytes(rng),
        generate_bytes(rng),
        Policies::new().with_max_fee(0),
        vec![],
        vec![],
        vec![],
    )
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionMaxGasExceeded, err);
}

#[test]
fn maturity() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(block_height)
        .add_fee_input()
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate script");

    TransactionBuilder::create(rng.r#gen(), rng.r#gen(), vec![])
        .maturity(block_height)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate tx create");

    let err = Transaction::script(
        Default::default(),
        vec![],
        vec![],
        Policies::new().with_maturity(1001.into()).with_max_fee(0),
        vec![],
        vec![],
        vec![],
    )
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionMaturity, err);

    let err = Transaction::create(
        0,
        Policies::new().with_maturity(1001.into()).with_max_fee(0),
        rng.r#gen(),
        vec![],
        vec![],
        vec![],
        vec![rng.r#gen()],
    )
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionMaturity, err);
}

#[test]
fn script__check__valid_expiration_policy() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        // Given
        .expiration(block_height)
        .add_fee_input()
        .finalize()
        // When
        .check(block_height, &test_params())
        // Then
        .expect("Failed to validate script");
}

#[test]
fn script__check__invalid_expiration_policy() {
    let rng = &mut StdRng::seed_from_u64(8586);

    // Given
    let block_height = 1000.into();
    let old_block_height = 999u32.into();
    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        // Given
        .expiration(old_block_height)
        .add_fee_input()
        .finalize()
        // When
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionExpiration, err);
}

#[test]
fn create__check__valid_expiration_policy() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    TransactionBuilder::create(rng.r#gen(), rng.r#gen(), vec![])
        // Given
        .expiration(block_height)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        // When
        .check(block_height, &test_params())
        // Then
        .expect("Failed to validate tx create");
}

#[test]
fn create__check__invalid_expiration_policy() {
    let rng = &mut StdRng::seed_from_u64(8586);

    // Given
    let block_height = 1000.into();
    let old_block_height = 999u32.into();
    let err = TransactionBuilder::create(rng.r#gen(), rng.r#gen(), vec![])
        .expiration(old_block_height)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        // When
        .check(block_height, &test_params())
        .expect_err("Failed to validate tx create");

    // Then
    assert_eq!(ValidityError::TransactionExpiration, err);
}

#[test]
fn script__check__not_set_witness_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // When
    let result = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_fee_input()
        .finalize()
        .check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn create_not_set_witness_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let bytecode = vec![];

    // When
    let result = TransactionBuilder::create(bytecode.clone().into(), rng.r#gen(), vec![])
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn script__check__set_witness_limit_for_empty_witness_success() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    // Given
    let limit = Signature::LEN /* witness from random fee */ + vec![0u8; 0].size_static();

    // When
    let result = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_fee_input()
        .witness_limit(limit as u64)
        .finalize()
        .check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn script_set_witness_limit_less_than_witness_data_size_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let witness_size = Signature::LEN + vec![0u8; 0].size_static();

    // Given
    let limit = witness_size - 1;

    // When
    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_fee_input()
        .witness_limit(limit as u64)
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionWitnessLimitExceeded, err);
}

#[test]
fn create_set_witness_limit_for_empty_witness_success() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let bytecode = vec![];
    // Given
    let limit = Signature::LEN /* witness from random fee */ + bytecode.size_static() + bytecode.size_static();

    // When
    let result = TransactionBuilder::create(bytecode.clone().into(), rng.r#gen(), vec![])
        .add_fee_input()
        .add_contract_created()
        .witness_limit(limit as u64)
        .finalize()
        .check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn create_set_witness_limit_less_than_witness_data_size_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();
    let bytecode = vec![];
    // Given
    let limit = Signature::LEN /* witness from random fee */ + bytecode.size_static() + bytecode.size_static();

    // When
    let err = TransactionBuilder::create(bytecode.clone().into(), rng.r#gen(), vec![])
        .add_fee_input()
        .witness_limit(limit as u64 - 1)
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionWitnessLimitExceeded, err);
}

#[test]
fn script_not_set_max_fee_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // When
    let result = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_fee_input()
        .finalize()
        .check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn script__check__no_max_fee_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let mut tx = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .add_fee_input()
        .finalize();
    tx.policies_mut().set(PolicyType::MaxFee, None);

    // When
    let err = tx
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionMaxFeeNotSet, err);
}

#[test]
fn create_not_set_max_fee_limit_success() {
    // Given
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // When
    let result = TransactionBuilder::create(rng.r#gen(), rng.r#gen(), vec![])
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn create__check__no_max_fee_fails() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let mut tx = TransactionBuilder::create(rng.r#gen(), rng.r#gen(), vec![])
        .add_fee_input()
        .finalize();
    tx.policies_mut().set(PolicyType::MaxFee, None);

    // When
    let err = tx
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    // Then
    assert_eq!(ValidityError::TransactionMaxFeeNotSet, err);
}

#[test]
fn max_iow() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    let mut builder =
        TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng));

    let asset_id: AssetId = rng.r#gen();

    builder.maturity(maturity).add_unsigned_coin_input(
        secret,
        rng.r#gen(),
        rng.r#gen(),
        asset_id,
        rng.r#gen(),
    );

    while builder.outputs().len() < TX_PARAMS.max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), asset_id));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    builder
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate transaction");

    // Add inputs up to maximum and validate
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![]);

    builder.maturity(maturity);
    builder.add_contract_created();

    let secrets =
        cmp::min(TX_PARAMS.max_inputs() as u32, TX_PARAMS.max_witnesses() - 1) as usize;
    let secrets: Vec<SecretKey> = (0..secrets - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    let asset_id: AssetId = AssetId::BASE;
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            asset_id,
            rng.r#gen(),
        );
    });

    while builder.outputs().len() < TX_PARAMS.max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), asset_id));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    builder
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate transaction");

    // Overflow maximum inputs and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![]);

    builder.maturity(maturity);

    let secrets: Vec<SecretKey> = (0..1 + TX_PARAMS.max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        );
    });

    while builder.outputs().len() < TX_PARAMS.max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), rng.r#gen()));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionInputsMax, err);

    // Overflow outputs maximum and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![]);

    builder.maturity(maturity);

    let secrets: Vec<SecretKey> = (0..TX_PARAMS.max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        );
    });

    while builder.outputs().len() < 1 + TX_PARAMS.max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), rng.r#gen()));
    }

    while builder.witnesses().len() < TX_PARAMS.max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionOutputsMax, err);

    // Overflow witnesses maximum and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![]);

    builder.maturity(maturity);

    let secrets: Vec<SecretKey> = (0..TX_PARAMS.max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        );
    });

    while builder.outputs().len() < TX_PARAMS.max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), rng.r#gen()));
    }

    while builder.witnesses().len() < 1 + TX_PARAMS.max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionWitnessesMax, err);
}

#[test]
fn output_change_asset_id() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let a: AssetId = rng.r#gen();
    let b: AssetId = rng.r#gen();
    let c: AssetId = rng.r#gen();

    let secret = SecretKey::random(rng);

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), a, rng.r#gen())
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), b, rng.r#gen())
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), b))
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate transaction");

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), a, rng.r#gen())
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), b, rng.r#gen())
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), a))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        ValidityError::TransactionOutputChangeAssetIdDuplicated(a),
        err
    );

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), a, rng.r#gen())
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), b, rng.r#gen())
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), c))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert!(matches!(
        err,
        ValidityError::TransactionOutputChangeAssetIdNotFound(asset_id) if asset_id == c
    ));

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .maturity(maturity)
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), a, rng.r#gen())
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), b, rng.r#gen())
        .add_output(Output::coin(rng.r#gen(), rng.next_u64(), a))
        .add_output(Output::coin(rng.r#gen(), rng.next_u64(), c))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert!(matches!(
        err,
        ValidityError::TransactionOutputCoinAssetIdNotFound(asset_id) if asset_id == c
    ));
}

#[test]
fn script__check__happy_path() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::change(rng.r#gen(), rng.r#gen(), asset_id))
    .finalize()
    .check(block_height, &test_params())
    .expect("Failed to validate transaction");
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__happy_path() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    TransactionBuilder::script_v2(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::coin(rng.r#gen(), rng.r#gen(), asset_id))
    .finalize()
    .check(block_height, &test_params())
    .expect("Failed to validate transaction");
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__predicate_coin__check__happy_path() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();
    let predicate_index = 0;
    let predicate_data_index = 1;
    let input = Input::coin_predicate_v2(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        asset_id,
        rng.r#gen(),
        predicate_index,
        predicate_data_index,
    );
    let predicate_static_witness = Witness::default();
    let predicate_data_static_witness = Witness::default();

    TransactionBuilder::script_v2(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_input(input)
    .add_output(Output::coin(rng.r#gen(), rng.r#gen(), asset_id))
    .add_static_witness(predicate_static_witness)
    .add_static_witness(predicate_data_static_witness)
    .finalize()
    .check(block_height, &test_params())
    .expect("Failed to validate transaction");
}

#[test]
fn script__check__cannot_create_contract() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(
        ValidityError::TransactionOutputContainsContractCreated { index: 0 },
        err
    );
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__cannot_create_contract() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script_v2(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(
        ValidityError::TransactionOutputContainsContractCreated { index: 0 },
        err
    );
}

#[test]
fn script__check__errors_if_script_too_long() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script(
        vec![0xfa; 1 + SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionScriptLength, err);
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__errors_if_script_too_long() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script_v2(
        vec![0xfa; 1 + SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionScriptLength, err);
}

#[test]
fn script__check__errors_if_script_data_too_long() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; 1 + SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionScriptDataLength, err);
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__errors_if_script_data_too_long() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script_v2(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; 1 + SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionScriptDataLength, err);
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script__check__errors_if_includes_v2_input() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_unsigned_coin_input_v2(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::coin(rng.r#gen(), rng.r#gen(), asset_id))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::WrongInputVersion, err);
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__errors_if_includes_v1_input() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::script_v2(
        vec![0xfa; SCRIPT_PARAMS.max_script_length() as usize],
        vec![0xfb; SCRIPT_PARAMS.max_script_data_length() as usize],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_unsigned_coin_input_v1(secret, rng.r#gen(), rng.r#gen(), asset_id, rng.r#gen())
    .add_output(Output::coin(rng.r#gen(), rng.r#gen(), asset_id))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::WrongInputVersion, err);
}

#[test]
fn create__check__happy_path() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate tx");
}

#[test]
fn create__check__cannot_have_contract_input() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_input(Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ))
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .add_output(Output::contract(0, rng.r#gen(), rng.r#gen()))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionInputContainsContract { index: 0 }
    );
}

#[test]
fn create__check__must_contain_contract_created_output() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_fee_input()
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionOutputDoesntContainContractCreated
    );
}

#[test]
fn create__check__cannot_have_message_input() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    let not_empty_data = vec![0x1];
    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_unsigned_message_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            not_empty_data,
        )
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionInputContainsMessageData { index: 0 }
    );
}

#[test]
fn create__check__cannot_have_variable_output() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_fee_input()
        .add_contract_created()
        .add_output(Output::variable(rng.r#gen(), rng.r#gen(), rng.r#gen()))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionOutputContainsVariable { index: 1 }
    );
}

#[test]
fn create__check__cannot_have_multiple_change_outputs() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let secret_b = SecretKey::random(rng);

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::default(),
            rng.r#gen(),
        )
        .add_unsigned_coin_input(
            secret_b,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::BASE))
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::BASE))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionOutputChangeAssetIdDuplicated(AssetId::BASE)
    );
}

#[test]
fn create__check__errors_if_input_non_base_asset_id() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);
    let secret_b = SecretKey::random(rng);

    let asset_id: AssetId = rng.r#gen();

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::BASE,
            rng.r#gen(),
        )
        .add_unsigned_coin_input(
            secret_b,
            rng.r#gen(),
            rng.r#gen(),
            asset_id,
            rng.r#gen(),
        )
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::BASE))
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), asset_id))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionInputContainsNonBaseAssetId { index: 1 },
    );
}

#[test]
fn create__check__cannot_create_multiple_contract_outputs() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    let witness = generate_bytes(rng);
    let contract = Contract::from(witness.as_ref());
    let salt = rng.r#gen();
    let storage_slots: Vec<StorageSlot> = vec![];
    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id = contract.id(&salt, &contract.root(), &state_root);

    let err = TransactionBuilder::create(witness.into(), salt, storage_slots)
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::BASE,
            rng.r#gen(),
        )
        .add_output(Output::contract_created(contract_id, state_root))
        .add_output(Output::contract_created(contract_id, state_root))
        .finalize()
        .check(block_height, &test_params())
        .expect_err("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidityError::TransactionCreateOutputContractCreatedMultiple { index: 1 },
    );
}

#[test]
fn create__check__something_else() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    TransactionBuilder::create(
        vec![0xfa; CONTRACT_PARAMS.contract_max_size() as usize / 4].into(),
        rng.r#gen(),
        vec![],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.r#gen(),
        rng.r#gen(),
        AssetId::default(),
        rng.r#gen(),
    )
    .add_contract_created()
    .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params())
    .expect("Failed to validate the transaction");
}

#[test]
fn create__check__errors_if_witness_bytecode_too_long() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    let err = TransactionBuilder::create(
        vec![0xfa; 1 + CONTRACT_PARAMS.contract_max_size() as usize].into(),
        rng.r#gen(),
        vec![],
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.r#gen(),
        rng.r#gen(),
        AssetId::default(),
        rng.r#gen(),
    )
    .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionCreateBytecodeLen);
}

#[test]
fn create__check_without_signatures__errors_if_wrong_witness_index() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    let err = Transaction::create(
        1,
        Policies::default().with_max_fee(0),
        rng.r#gen(),
        vec![],
        vec![Input::coin_signed(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            0,
        )],
        vec![],
        vec![Default::default()],
    )
    .check_without_signatures(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionCreateBytecodeWitnessIndex);
}

#[test]
fn create__check__something() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
        .maturity(maturity)
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::default(),
            rng.r#gen(),
        )
        .add_contract_created()
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::default()))
        .finalize()
        .check(block_height, &test_params())
        .expect("Failed to validate the transaction");
}

#[test]
fn create__check__can_max_out_storage_slots() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    let storage_slots = (0..CONTRACT_PARAMS.max_storage_slots())
        .map(|i| {
            let mut slot_data = StorageSlot::default().to_bytes();
            slot_data[..8].copy_from_slice(&i.to_be_bytes()); // Force ordering
            StorageSlot::from_bytes(&slot_data).unwrap()
        })
        .collect::<Vec<StorageSlot>>();

    // Test max slots is valid
    TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.r#gen(),
        storage_slots.clone(),
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.r#gen(),
        rng.r#gen(),
        AssetId::default(),
        rng.r#gen(),
    )
    .add_contract_created()
    .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params())
    .expect("Failed to validate the transaction");
}

#[test]
fn create__check__cannot_exceed_max_storage_slot() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100.into();
    let block_height = 1000.into();

    let secret = SecretKey::random(rng);

    // Test max slots can't be exceeded
    let mut storage_slots_max = (0..CONTRACT_PARAMS.max_storage_slots())
        .map(|i| {
            let mut slot_data = StorageSlot::default().to_bytes();
            slot_data[..8].copy_from_slice(&i.to_be_bytes()); // Force ordering
            StorageSlot::from_bytes(&slot_data).unwrap()
        })
        .collect::<Vec<StorageSlot>>();

    let s = StorageSlot::new([255u8; 32].into(), Default::default());
    storage_slots_max.push(s);

    let err = TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.r#gen(),
        storage_slots_max,
    )
    .maturity(maturity)
    .add_unsigned_coin_input(
        secret,
        rng.r#gen(),
        rng.r#gen(),
        AssetId::default(),
        rng.r#gen(),
    )
    .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::default()))
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(ValidityError::TransactionCreateStorageSlotMax, err);
}

#[test]
fn script__check__transaction_at_maximum_size_is_valid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let secret = SecretKey::random(rng);

    let block_height = 100.into();
    let mut params = test_params();
    let max_size = 1024usize;
    let mut tx_params = *params.tx_params();
    tx_params.set_max_size(max_size as u64);
    params.set_tx_params(tx_params);

    let base_size = {
        let tx = TransactionBuilder::script(vec![], vec![])
            .add_unsigned_coin_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
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
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .finalize();

    tx.check(block_height, &params)
        .expect("Expected valid transaction");
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__transaction_at_maximum_size_is_valid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let secret = SecretKey::random(rng);

    let block_height = 100.into();
    let mut params = test_params();
    let max_size = 1024usize;
    let mut tx_params = *params.tx_params();
    tx_params.set_max_size(max_size as u64);
    params.set_tx_params(tx_params);

    let base_size = {
        let tx = TransactionBuilder::script_v2(vec![], vec![])
            .add_unsigned_coin_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
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
    let tx = TransactionBuilder::script_v2(script, vec![])
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .finalize();

    tx.check(block_height, &params)
        .expect("Expected valid transaction");
}

#[test]
fn script__check__transaction_exceeding_maximum_size_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let secret = SecretKey::random(rng);

    let block_height = 100.into();
    let mut params = test_params();
    let max_size = 1024usize;
    let mut tx_params = *params.tx_params();
    tx_params.set_max_size(max_size as u64);
    params.set_tx_params(tx_params);

    let base_size = {
        let tx = TransactionBuilder::script(vec![], vec![])
            .add_unsigned_coin_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
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
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .finalize();

    let err = tx
        .check(block_height, &params)
        .expect_err("Expected valid transaction");

    assert_eq!(err, ValidityError::TransactionSizeLimitExceeded);
}

#[cfg(feature = "chargeable-tx-v2")]
#[test]
fn script_v2__check__transaction_exceeding_maximum_size_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let secret = SecretKey::random(rng);

    let block_height = 100.into();
    let mut params = test_params();
    let max_size = 1024usize;
    let mut tx_params = *params.tx_params();
    tx_params.set_max_size(max_size as u64);
    params.set_tx_params(tx_params);

    let base_size = {
        let tx = TransactionBuilder::script_v2(vec![], vec![])
            .add_unsigned_coin_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
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
    let tx = TransactionBuilder::script_v2(script, vec![])
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .finalize();

    let err = tx
        .check(block_height, &params)
        .expect_err("Expected valid transaction");

    assert_eq!(err, ValidityError::TransactionSizeLimitExceeded);
}

#[test]
fn mint() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000.into();

    let err = TransactionBuilder::mint(
        block_height,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionMintIncorrectOutputIndex);

    let err = TransactionBuilder::mint(
        block_height,
        rng.r#gen(),
        rng.r#gen(),
        output::contract::Contract {
            input_index: 0,
            balance_root: rng.r#gen(),
            state_root: rng.r#gen(),
        },
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .finalize()
    .check(block_height, &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionMintNonBaseAsset);

    let err = TransactionBuilder::mint(
        block_height,
        rng.r#gen(),
        rng.r#gen(),
        output::contract::Contract {
            input_index: 0,
            balance_root: rng.r#gen(),
            state_root: rng.r#gen(),
        },
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .finalize()
    .check(block_height.succ().unwrap(), &test_params())
    .expect_err("Expected erroneous transaction");

    assert_eq!(err, ValidityError::TransactionMintIncorrectBlockHeight);
}

mod inputs {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn coin_predicate_check_owner_works() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let predicate = (0..1000).map(|_| rng.r#gen()).collect_vec();
        // The predicate is an owner of the coin
        let owner: Address = Input::predicate_owner(&predicate);

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
                .maturity(rng.r#gen())
                .add_input(Input::coin_predicate(
                    rng.r#gen(),
                    owner,
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
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

        let predicate = (0..1000).map(|_| rng.r#gen()).collect_vec();

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
                .maturity(rng.r#gen())
                .add_input(Input::coin_predicate(
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
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

        let predicate = (0..1000).map(|_| rng.r#gen()).collect_vec();
        // The predicate is an recipient(owner) of the message
        let recipient: Address = Input::predicate_owner(&predicate);

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
                .maturity(rng.r#gen())
                .add_input(Input::message_data_predicate(
                    rng.r#gen(),
                    recipient,
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
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

        let predicate = (0..1000).map(|_| rng.r#gen()).collect_vec();

        let tx =
            TransactionBuilder::create(generate_bytes(rng).into(), rng.r#gen(), vec![])
                .maturity(rng.r#gen())
                .add_input(Input::message_data_predicate(
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    rng.r#gen(),
                    vec![],
                    predicate,
                    vec![],
                ))
                .with_tx_params(TX_PARAMS)
                .finalize();

        assert!(!tx.check_predicate_owners());
    }
}
