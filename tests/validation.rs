use fuel_vm::{prelude::*, util::test_helpers::TestBuilder};

#[test]
fn transaction_validation_fails_when_provided_fees_dont_cover_byte_costs() {
    let input_amount = 1000;
    let gas_price = 0;
    // make byte price too high for the input amount
    let byte_price = 1000;

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(
            VmValidationError::InsufficientFeeAmount {
                provided: _,
                expected: _
            }
        ))
    ));
}

#[test]
fn transaction_validation_fails_when_provided_fees_dont_cover_gas_costs() {
    let input_amount = 1000;
    // make gas price too high for the input amount
    let gas_price = 1000;
    let byte_price = 0;

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(
            VmValidationError::InsufficientFeeAmount {
                provided: _,
                expected: _
            }
        ))
    ));
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

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(
            VmValidationError::TransactionValidation(ValidationError::TransactionOutputChangeAssetIdNotFound(
                asset
            ))
        )) if asset == missing_asset
    ));
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

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(
            VmValidationError::TransactionValidation(ValidationError::TransactionOutputCoinAssetIdNotFound(asset))
        )) if asset == missing_asset
    ));
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

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(
            VmValidationError::TransactionValidation(ValidationError::TransactionOutputChangeAssetIdDuplicated)
        ))
    ))
}

#[test]
fn bytes_fee_cant_overflow() {
    let input_amount = 1000;
    let gas_price = 0;
    // make byte price too high for the input amount
    let byte_price = Word::MAX;

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(VmValidationError::ArithmeticOverflow))
    ));
}

#[test]
fn gas_fee_cant_overflow() {
    let input_amount = 1000;
    let gas_price = Word::MAX;
    let gas_limit = 2;
    // make byte price too high for the input amount
    let byte_price = 0;

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(VmValidationError::ArithmeticOverflow))
    ));
}

#[test]
fn total_fee_cant_overflow() {
    // ensure that total fee can't overflow as a result of adding the gas fee and byte fee

    let input_amount = 1000;
    let gas_price = Word::MAX;
    let gas_limit = 1;
    // make byte price too high for the input amount
    let byte_price = 1;

    let transaction = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .build();

    let mut interpreter = Interpreter::with_memory_storage();
    let result = interpreter.transact(transaction);
    assert!(matches!(
        result,
        Err(InterpreterError::ValidationError(VmValidationError::ArithmeticOverflow))
    ));
}
