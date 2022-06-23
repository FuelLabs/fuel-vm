use fuel_tx::TransactionBuilder;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn transaction_validation_fails_when_provided_fees_dont_cover_byte_costs() {
    let input_amount = 100;
    let gas_price = 0;
    let factor = 1;

    let params = ConsensusParameters::default().with_gas_price_factor(factor);

    // make byte price too high for the input amount
    let byte_price = factor;

    let transaction = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let err = Interpreter::with_memory_storage()
        .with_params(params)
        .transact(transaction)
        .err()
        .expect("insufficient fee amount expected");

    let provided = match err {
        InterpreterError::ValidationError(ValidationError::InsufficientFeeAmount { provided, .. }) => provided,
        _ => panic!("expected insufficient fee amount; found {:?}", err),
    };

    assert_eq!(provided, input_amount);
}

#[test]
fn transaction_validation_fails_when_provided_fees_dont_cover_gas_costs() {
    let input_amount = 10;
    let factor = 1;

    let params = ConsensusParameters::default().with_gas_price_factor(factor);

    // make gas price too high for the input amount
    let gas_price = factor;
    let byte_price = 0;

    let transaction = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let err = Interpreter::with_memory_storage()
        .with_params(params)
        .transact(transaction)
        .err()
        .expect("insufficient fee amount expected");

    let provided = match err {
        InterpreterError::ValidationError(ValidationError::InsufficientFeeAmount { provided, .. }) => provided,
        _ => panic!("expected insufficient fee amount; found {:?}", err),
    };

    assert_eq!(provided, input_amount);
}

#[test]
fn transaction_validation_fails_when_change_asset_id_not_in_inputs() {
    let input_amount = 1000;
    // make gas price too high for the input amount
    let gas_price = 0;
    let byte_price = 0;
    let missing_asset = AssetId::from([1; 32]);

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        // make change output with no corresponding input asset
        .change_output(missing_asset)
        .build();

    let err = Interpreter::with_memory_storage()
        .transact(transaction)
        .err()
        .expect("asset not found expected");

    assert_eq!(
        err,
        ValidationError::TransactionOutputChangeAssetIdNotFound(missing_asset).into()
    );
}

#[test]
fn transaction_validation_fails_when_coin_output_asset_id_not_in_inputs() {
    let input_amount = 1000;
    // make gas price too high for the input amount
    let gas_price = 0;
    let byte_price = 0;
    let missing_asset = AssetId::from([1; 32]);

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        // make coin output with no corresponding input asset
        .coin_output(missing_asset, 0)
        .build();

    let err = Interpreter::with_memory_storage()
        .transact(transaction)
        .err()
        .expect("asset not found expected");

    assert_eq!(
        err,
        ValidationError::TransactionOutputCoinAssetIdNotFound(missing_asset).into()
    );
}

#[test]
fn change_is_not_duplicated_for_each_base_asset_change_output() {
    // create multiple change outputs for the base asset and ensure the total change is correct
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let asset_id = AssetId::default();

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .change_output(asset_id)
        .change_output(asset_id)
        .build();

    let err = Interpreter::with_memory_storage()
        .transact(transaction)
        .err()
        .expect("asset duplicated expected");

    assert_eq!(err, ValidationError::TransactionOutputChangeAssetIdDuplicated.into());
}

#[test]
fn bytes_fee_cant_overflow() {
    let input_amount = 1000;
    let gas_price = 0;
    // make byte price too high for the input amount
    let byte_price = Word::MAX;

    let params = ConsensusParameters::default().with_gas_price_factor(1);

    let transaction = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let err = Interpreter::with_memory_storage()
        .with_params(params)
        .transact(transaction)
        .err()
        .expect("overflow expected");

    assert_eq!(err, ValidationError::ArithmeticOverflow.into());
}

#[test]
fn gas_fee_cant_overflow() {
    let input_amount = 1000;
    let gas_price = Word::MAX;
    let gas_limit = 2;
    // make byte price too high for the input amount
    let byte_price = 0;

    let params = ConsensusParameters::default().with_gas_price_factor(1);

    let transaction = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let err = Interpreter::with_memory_storage()
        .with_params(params)
        .transact(transaction)
        .err()
        .expect("overflow expected");

    assert_eq!(err, ValidationError::ArithmeticOverflow.into());
}

#[test]
fn total_fee_cant_overflow() {
    // ensure that total fee can't overflow as a result of adding the gas fee and byte fee
    let input_amount = 1000;

    let gas_price = Word::MAX;
    let gas_limit = 1;

    // make byte price too high for the input amount
    let byte_price = Word::MAX;

    let params = ConsensusParameters::default().with_gas_price_factor(1);

    let transaction = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let err = Interpreter::with_memory_storage()
        .with_params(params)
        .transact(transaction)
        .err()
        .expect("overflow expected");

    assert_eq!(err, ValidationError::ArithmeticOverflow.into());
}

#[test]
fn transaction_cannot_be_executed_before_maturity() {
    const MATURITY: u64 = 1;
    const BLOCK_HEIGHT: u32 = 0;

    let mut rng = StdRng::seed_from_u64(2322u64);
    let tx = TransactionBuilder::script(vec![Opcode::RET(1)].into_iter().collect(), Default::default())
        .add_unsigned_coin_input(Default::default(), &rng.gen(), 1, Default::default(), 0)
        .gas_limit(100)
        .maturity(MATURITY)
        .finalize();

    let result = TestBuilder::new(2322u64).block_height(BLOCK_HEIGHT).execute_tx(tx);
    assert!(result.err().unwrap().to_string().contains("TransactionMaturity"));
}

#[test]
fn transaction_can_be_executed_after_maturity() {
    const MATURITY: u64 = 1;
    const BLOCK_HEIGHT: u32 = 2;

    let mut rng = StdRng::seed_from_u64(2322u64);
    let tx = TransactionBuilder::script(vec![Opcode::RET(1)].into_iter().collect(), Default::default())
        .add_unsigned_coin_input(Default::default(), &rng.gen(), 1, Default::default(), 0)
        .gas_limit(100)
        .maturity(MATURITY)
        .finalize();

    let result = TestBuilder::new(2322u64).block_height(BLOCK_HEIGHT).execute_tx(tx);
    assert!(result.is_ok());
}
