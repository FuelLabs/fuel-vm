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
    for idx in 0..=u16::MAX {
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

#[cfg(feature = "da-compression")]
mod da_compression {
    use std::convert::Infallible;

    use fuel_compression_0_58_2::{
        CompressibleBy as CompressibleBy_0_58_2,
        RegistryKey as RegistryKey_0_58_2,
    };
    use latest_fuel_compression::{
        CompressibleBy as LatestCompressibleBy,
        RegistryKey as LatestRegistryKey,
    };

    use fuel_tx_0_58_2::{
        input::PredicateCode as PredicateCode_0_58_2,
        Address as Address_0_58_2,
        AssetId as AssetId_0_58_2,
        CompressedUtxoId as CompressedUtxoId_0_58_2,
        ContractId as ContractId_0_58_2,
        ScriptCode as ScriptCode_0_58_2,
        TxPointer as TxPointer_0_58_2,
        UtxoId as UtxoId_0_58_2,
    };
    use latest_fuel_tx::{
        input::PredicateCode as LatestPredicateCode,
        Address as LatestAddress,
        AssetId as LatestAssetId,
        CompressedUtxoId as LatestCompressedUtxoId,
        ContractId as LatestContractId,
        ScriptCode as LatestScriptCode,
        TxPointer as LatestTxPointer,
        UtxoId as LatestUtxoId,
    };
    struct TestContext;

    impl latest_fuel_compression::ContextError for TestContext {
        type Error = Infallible;
    }

    impl fuel_compression_0_58_2::ContextError for TestContext {
        type Error = Infallible;
    }

    impl LatestCompressibleBy<TestContext> for LatestUtxoId {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<LatestCompressedUtxoId, Infallible> {
            let key = LatestCompressedUtxoId {
                tx_pointer: LatestTxPointer::default(),
                output_index: self.output_index(),
            };
            Ok(key)
        }
    }

    impl LatestCompressibleBy<TestContext> for LatestAddress {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<LatestRegistryKey, Infallible> {
            Ok(LatestRegistryKey::DEFAULT_VALUE)
        }
    }

    impl LatestCompressibleBy<TestContext> for LatestAssetId {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<LatestRegistryKey, Infallible> {
            Ok(LatestRegistryKey::DEFAULT_VALUE)
        }
    }

    impl LatestCompressibleBy<TestContext> for LatestContractId {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<LatestRegistryKey, Infallible> {
            Ok(LatestRegistryKey::DEFAULT_VALUE)
        }
    }

    impl LatestCompressibleBy<TestContext> for LatestScriptCode {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<LatestRegistryKey, Infallible> {
            Ok(LatestRegistryKey::DEFAULT_VALUE)
        }
    }

    impl LatestCompressibleBy<TestContext> for LatestPredicateCode {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<LatestRegistryKey, Infallible> {
            Ok(LatestRegistryKey::DEFAULT_VALUE)
        }
    }

    impl CompressibleBy_0_58_2<TestContext> for UtxoId_0_58_2 {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<CompressedUtxoId_0_58_2, Infallible> {
            let key = CompressedUtxoId_0_58_2 {
                tx_pointer: TxPointer_0_58_2::default(),
                output_index: self.output_index(),
            };
            Ok(key)
        }
    }

    impl CompressibleBy_0_58_2<TestContext> for Address_0_58_2 {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<RegistryKey_0_58_2, Infallible> {
            Ok(RegistryKey_0_58_2::DEFAULT_VALUE)
        }
    }

    impl CompressibleBy_0_58_2<TestContext> for AssetId_0_58_2 {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<RegistryKey_0_58_2, Infallible> {
            Ok(RegistryKey_0_58_2::DEFAULT_VALUE)
        }
    }

    impl CompressibleBy_0_58_2<TestContext> for ContractId_0_58_2 {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<RegistryKey_0_58_2, Infallible> {
            Ok(RegistryKey_0_58_2::DEFAULT_VALUE)
        }
    }

    impl CompressibleBy_0_58_2<TestContext> for ScriptCode_0_58_2 {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<RegistryKey_0_58_2, Infallible> {
            Ok(RegistryKey_0_58_2::DEFAULT_VALUE)
        }
    }

    impl CompressibleBy_0_58_2<TestContext> for PredicateCode_0_58_2 {
        async fn compress_with(
            &self,
            _ctx: &mut TestContext,
        ) -> Result<RegistryKey_0_58_2, Infallible> {
            Ok(RegistryKey_0_58_2::DEFAULT_VALUE)
        }
    }

    #[tokio::test]
    async fn release_0_58_2_can_deserialize_compressed_latest() {
        // Given
        let mut context = TestContext;
        let tx = latest_fuel_tx::Transaction::default_test_tx();
        let tx_0_58_2 = fuel_tx_0_58_2::Transaction::default_test_tx();

        // When
        let tx = tx.compress_with(&mut context).await.unwrap();
        let bytes_latest = postcard::to_allocvec(&tx).unwrap();
        let tx_0_58_2 = tx_0_58_2.compress_with(&mut context).await.unwrap();
        let bytes_0_58_2 = postcard::to_allocvec(&tx_0_58_2).unwrap();

        // Then
        assert_eq!(bytes_latest, bytes_0_58_2);
    }
}
