use fuel_crypto::SecretKey;
use fuel_tx::consts::*;
use fuel_tx::*;
use fuel_tx_test_helpers::generate_bytes;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

use std::cmp;
use std::io::Write;

#[test]
fn gas_limit() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100;
    let block_height = 1000;

    Transaction::script(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        maturity,
        generate_bytes(rng),
        generate_bytes(rng),
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .expect("Failed to validate transaction");

    Transaction::create(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![vec![0xfau8].into()],
    )
    .validate(block_height)
    .expect("Failed to validate transaction");

    let err = Transaction::script(
        rng.gen(),
        MAX_GAS_PER_TX + 1,
        rng.gen(),
        maturity,
        generate_bytes(rng),
        generate_bytes(rng),
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionGasLimit, err);

    let err = Transaction::create(
        rng.gen(),
        MAX_GAS_PER_TX + 1,
        rng.gen(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![generate_bytes(rng).into()],
    )
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionGasLimit, err);
}

#[test]
fn maturity() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let block_height = 1000;

    Transaction::script(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        block_height,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .expect("Failed to validate script");

    Transaction::create(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        1000,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![rng.gen()],
    )
    .validate(block_height)
    .expect("Failed to validate tx create");

    let err = Transaction::script(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        1001,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionMaturity, err);

    let err = Transaction::create(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        1001,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionMaturity, err);
}

#[test]
fn max_iow() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100;
    let block_height = 1000;

    let secret = SecretKey::random(rng);

    let mut builder = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng));

    let asset_id: AssetId = rng.gen();

    builder
        .gas_price(rng.gen())
        .gas_limit(MAX_GAS_PER_TX)
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), asset_id, maturity);

    while builder.outputs().len() < MAX_OUTPUTS as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), asset_id));
    }

    while builder.witnesses().len() < MAX_WITNESSES as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    builder
        .finalize()
        .validate(block_height)
        .expect("Failed to validate transaction");

    // Add inputs up to maximum and validate
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![]);

    builder
        .gas_price(rng.gen())
        .gas_limit(MAX_GAS_PER_TX)
        .maturity(maturity);

    let secrets = cmp::min(MAX_INPUTS, MAX_WITNESSES - 1) as usize;
    let secrets: Vec<SecretKey> = (0..secrets - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    let asset_id: AssetId = rng.gen();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(rng.gen(), k, rng.gen(), asset_id, maturity);
    });

    while builder.outputs().len() < MAX_OUTPUTS as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), asset_id));
    }

    while builder.witnesses().len() < MAX_WITNESSES as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    builder
        .finalize()
        .validate(block_height)
        .expect("Failed to validate transaction");

    // Overflow maximum inputs and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![]);

    builder
        .gas_price(rng.gen())
        .gas_limit(MAX_GAS_PER_TX)
        .maturity(maturity);

    let secrets: Vec<SecretKey> = (0..1 + MAX_INPUTS as usize - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(rng.gen(), k, rng.gen(), rng.gen(), maturity);
    });

    while builder.outputs().len() < MAX_OUTPUTS as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), rng.gen()));
    }

    while builder.witnesses().len() < MAX_WITNESSES as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionInputsMax, err);

    // Overflow outputs maximum and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![]);

    builder
        .gas_price(rng.gen())
        .gas_limit(MAX_GAS_PER_TX)
        .maturity(maturity);

    let secrets: Vec<SecretKey> = (0..MAX_INPUTS as usize - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(rng.gen(), k, rng.gen(), rng.gen(), maturity);
    });

    while builder.outputs().len() < 1 + MAX_OUTPUTS as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), rng.gen()));
    }

    while builder.witnesses().len() < MAX_WITNESSES as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionOutputsMax, err);

    // Overflow witnesses maximum and expect error
    let mut builder =
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![]);

    builder
        .gas_price(rng.gen())
        .gas_limit(MAX_GAS_PER_TX)
        .maturity(maturity);

    let secrets: Vec<SecretKey> = (0..MAX_INPUTS as usize - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();

    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(rng.gen(), k, rng.gen(), rng.gen(), maturity);
    });

    while builder.outputs().len() < MAX_OUTPUTS as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), rng.gen()));
    }

    while builder.witnesses().len() < 1 + MAX_WITNESSES as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    let err = builder
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionWitnessesMax, err);
}

#[test]
fn output_change_asset_id() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100;
    let block_height = 1000;

    let a: AssetId = rng.gen();
    let b: AssetId = rng.gen();
    let c: AssetId = rng.gen();

    let secret = SecretKey::random(rng);

    TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), a, rng.gen())
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), b, rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), b))
        .finalize()
        .validate(block_height)
        .expect("Failed to validate transaction");

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), a, rng.gen())
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), b, rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(
        ValidationError::TransactionOutputChangeAssetIdDuplicated,
        err
    );

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), a, rng.gen())
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), b, rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), c))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert!(matches!(
        err,
        ValidationError::TransactionOutputChangeAssetIdNotFound(asset_id) if asset_id == c
    ));

    let err = TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), a, rng.gen())
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), b, rng.gen())
        .add_output(Output::coin(rng.gen(), rng.next_u64(), a))
        .add_output(Output::coin(rng.gen(), rng.next_u64(), c))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert!(matches!(
        err,
        ValidationError::TransactionOutputCoinAssetIdNotFound(asset_id) if asset_id == c
    ));
}

#[test]
fn script() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100;
    let block_height = 1000;

    let secret = SecretKey::random(rng);
    let asset_id: AssetId = rng.gen();

    TransactionBuilder::script(
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), asset_id, rng.gen())
    .add_output(Output::change(rng.gen(), rng.gen(), asset_id))
    .finalize()
    .validate(block_height)
    .expect("Failed to validate transaction");

    let err = TransactionBuilder::script(
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), asset_id, rng.gen())
    .add_output(Output::contract_created(rng.gen(), rng.gen()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(
        ValidationError::TransactionScriptOutputContractCreated { index: 0 },
        err
    );

    let err = TransactionBuilder::script(
        vec![0xfa; 1 + MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), asset_id, rng.gen())
    .add_output(Output::contract_created(rng.gen(), rng.gen()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionScriptLength, err);

    let err = TransactionBuilder::script(
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; 1 + MAX_SCRIPT_DATA_LENGTH as usize],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), asset_id, rng.gen())
    .add_output(Output::contract_created(rng.gen(), rng.gen()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionScriptDataLength, err);
}

#[test]
fn create() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100;
    let block_height = 1000;

    let secret = SecretKey::random(rng);
    let secret_b = SecretKey::random(rng);

    TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![])
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), rng.gen(), maturity)
        .finalize()
        .validate(block_height)
        .expect("Failed to validate tx");

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![])
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_input(Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
        .add_output(Output::contract(0, rng.gen(), rng.gen()))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidationError::TransactionCreateInputContract { index: 0 }
    );

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![])
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), rng.gen(), maturity)
        .add_output(Output::variable(rng.gen(), rng.gen(), rng.gen()))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidationError::TransactionCreateOutputVariable { index: 0 }
    );

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![])
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
        .add_unsigned_coin_input(rng.gen(), &secret_b, rng.gen(), rng.gen(), maturity)
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidationError::TransactionOutputChangeAssetIdDuplicated,
    );

    let asset_id: AssetId = rng.gen();

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![])
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
        .add_unsigned_coin_input(rng.gen(), &secret_b, rng.gen(), asset_id, maturity)
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
        .add_output(Output::change(rng.gen(), rng.gen(), asset_id))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidationError::TransactionCreateOutputChangeNotBaseAsset { index: 1 },
    );

    let err = TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![], vec![])
        .gas_limit(MAX_GAS_PER_TX)
        .gas_price(rng.gen())
        .maturity(maturity)
        .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
        .add_unsigned_coin_input(rng.gen(), &secret_b, rng.gen(), rng.gen(), maturity)
        .add_output(Output::contract_created(rng.gen(), rng.gen()))
        .add_output(Output::contract_created(rng.gen(), rng.gen()))
        .finalize()
        .validate(block_height)
        .err()
        .expect("Expected erroneous transaction");

    assert_eq!(
        err,
        ValidationError::TransactionCreateOutputContractCreatedMultiple { index: 1 },
    );

    TransactionBuilder::create(
        vec![0xfa; CONTRACT_MAX_SIZE as usize / 4].into(),
        rng.gen(),
        vec![],
        vec![],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .expect("Failed to validate the transaction");

    let err = TransactionBuilder::create(
        vec![0xfa; 1 + CONTRACT_MAX_SIZE as usize].into(),
        rng.gen(),
        vec![],
        vec![],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(err, ValidationError::TransactionCreateBytecodeLen);

    let err = Transaction::create(
        rng.gen(),
        MAX_GAS_PER_TX,
        rng.gen(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .validate_without_signature(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(err, ValidationError::TransactionCreateBytecodeWitnessIndex);

    let static_contracts = (0..MAX_STATIC_CONTRACTS as u64)
        .map(|i| {
            let mut id = ContractId::default();

            id.as_mut()[..8].copy_from_slice(&i.to_be_bytes());

            id
        })
        .collect::<Vec<ContractId>>();

    TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        static_contracts.clone(),
        vec![],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .expect("Failed to validate the transaction");

    let mut contracts_overflow = static_contracts.clone();

    let id = [0xff; ContractId::LEN];
    let id = ContractId::from(id);
    contracts_overflow.push(id);

    let err = TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        contracts_overflow,
        vec![],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(err, ValidationError::TransactionCreateStaticContractsMax,);

    let mut contracts_order = static_contracts.clone();

    contracts_order[0].as_mut()[0] = 0xff;

    let err = TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        contracts_order,
        vec![],
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(err, ValidationError::TransactionCreateStaticContractsOrder,);

    let mut slot_data = [0u8; 64];
    let mut slot = StorageSlot::default();

    let storage_slots = (0..MAX_STORAGE_SLOTS as u64)
        .map(|i| {
            slot_data[..8].copy_from_slice(&i.to_be_bytes());
            let _ = slot.write(&slot_data).unwrap();
            slot.clone()
        })
        .collect::<Vec<StorageSlot>>();

    // Test max slots is valid
    TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        static_contracts.clone(),
        storage_slots.clone(),
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .expect("Failed to validate the transaction");

    // Test max slots can't be exceeded
    let mut storage_slots_max = storage_slots.clone();

    let s = StorageSlot::new([255u8; 32].into(), Default::default());
    storage_slots_max.push(s);

    let err = TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        static_contracts.clone(),
        storage_slots_max,
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionCreateStorageSlotMax, err);

    // Test storage slots must be sorted correctly
    let mut storage_slots_reverse = storage_slots;

    storage_slots_reverse.reverse();

    let err = TransactionBuilder::create(
        generate_bytes(rng).into(),
        rng.gen(),
        static_contracts,
        storage_slots_reverse,
    )
    .gas_limit(MAX_GAS_PER_TX)
    .gas_price(rng.gen())
    .maturity(maturity)
    .add_unsigned_coin_input(rng.gen(), &secret, rng.gen(), AssetId::default(), maturity)
    .add_output(Output::change(rng.gen(), rng.gen(), AssetId::default()))
    .finalize()
    .validate(block_height)
    .err()
    .expect("Expected erroneous transaction");

    assert_eq!(ValidationError::TransactionCreateStorageSlotOrder, err);
}

#[test]
fn tx_id_bytecode_len() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 100;
    let gas_price = rng.gen();
    let byte_price = rng.gen();
    let salt = rng.gen();

    let w_a = vec![0xfau8; 4].into();
    let w_b = vec![0xfau8; 8].into();
    let w_c = vec![0xfbu8; 4].into();

    let tx_a = Transaction::create(
        gas_price,
        MAX_GAS_PER_TX,
        byte_price,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![w_a],
    );

    let tx_b = Transaction::create(
        gas_price,
        MAX_GAS_PER_TX,
        byte_price,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![w_b],
    );

    let tx_c = Transaction::create(
        gas_price,
        MAX_GAS_PER_TX,
        byte_price,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![w_c],
    );

    let id_a = tx_a.id();
    let id_b = tx_b.id();
    let id_c = tx_c.id();

    // bytecode with different length should produce different id
    assert_ne!(id_a, id_b);

    // bytecode with same length and different content should produce same id
    //
    // Note that this isn't related to the validation itself - this checks exclusively the id
    // behavior. the witness payload for a bytecode cannot be tampered and the validation rules
    // should not allow this case to pass.
    //
    // For further reference, check
    // https://github.com/FuelLabs/fuel-specs/blob/1856de801fabc7e52f5c010c45c3fc6d5d4e2be3/specs/protocol/tx_format.md?plain=1#L160
    assert_eq!(id_a, id_c);
}
