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
