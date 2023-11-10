use super::*;

use fuel_types::canonical::{
    Deserialize,
    Serialize,
};

#[derive(Deserialize, Serialize)]
struct Foo<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Default for Foo<N> {
    fn default() -> Self {
        Self { data: [0; N] }
    }
}

#[test]
fn check_size_returns_ok_for_valid_size() {
    // Given
    let f = Foo::<32>::default();
    let params = TxParameters::default().with_max_size(32);

    // When
    let result = check_size(&f, &params);

    // Then
    result.expect("Expected check_size to succeed");
}

#[test]
fn check_size_returns_transaction_size_limit_exceeded_for_invalid_size() {
    // Given
    let f = Foo::<33>::default();
    let params = TxParameters::default().with_max_size(32);

    // When
    let result = check_size(&f, &params);

    // Then
    let err = result.expect_err("Expected check_size to return err");
    assert_eq!(err, ValidityError::TransactionSizeLimitExceeded);
}
