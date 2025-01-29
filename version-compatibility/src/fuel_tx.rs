#[test]
fn latest_can_deserialize_0_58_2() {
    // Given
    let tx = fuel_tx_0_58_2::Transaction::default_test_tx();
    let bytes_0_58_2 = postcard::to_allocvec(&tx).unwrap();

    // When
    let latest_tx: Result<latest_fuel_tx::Transaction, _> =
        postcard::from_bytes(&bytes_0_58_2);

    // Then
    let _ = latest_tx.expect("Deserialization failed");
}

#[test]
fn release_0_58_2_can_deserialize_latest() {
    // Given
    let tx = latest_fuel_tx::Transaction::default_test_tx();
    let bytes_0_58_2 = postcard::to_allocvec(&tx).unwrap();

    // When
    let latest_tx: Result<fuel_tx_0_58_2::Transaction, _> =
        postcard::from_bytes(&bytes_0_58_2);

    // Then
    let _ = latest_tx.expect("Deserialization failed");
}

#[test]
fn latest_can_deserialize_previous_tx_pointer() {
    for idx in 0..u16::MAX {
        // Given
        let tx_pointer = fuel_tx_0_59_1::TxPointer::new(1u32.into(), idx);
        let expected = latest_fuel_tx::TxPointer::new(1u32.into(), idx.into());
        let bytes_expected = postcard::to_allocvec(&expected).unwrap();
        let str_expected = format!("{}", expected);
        let bytes_0_59_1 = postcard::to_allocvec(&tx_pointer).unwrap();
        let str_0_59_1 = format!("{}", tx_pointer);

        // When
        let latest_tx_pointer_from_bytes: Result<latest_fuel_tx::TxPointer, _> =
            postcard::from_bytes(&bytes_0_59_1);
        let latest_tx_pointer_from_str: Result<latest_fuel_tx::TxPointer, _> =
            str_0_59_1.parse();

        // Then
        assert_eq!(bytes_expected, bytes_0_59_1);
        assert_eq!(str_expected, str_0_59_1);
        let latest_tx_pointer_from_bytes =
            latest_tx_pointer_from_bytes.expect("Deserialization from bytes failed");
        let latest_tx_pointer_from_str =
            latest_tx_pointer_from_str.expect("Deserialization from str failed");
        assert_eq!(latest_tx_pointer_from_bytes, expected);
        assert_eq!(latest_tx_pointer_from_str, expected);
    }
}
