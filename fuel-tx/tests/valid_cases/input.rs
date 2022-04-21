use fuel_crypto::SecretKey;
use fuel_tx::consts::*;
use fuel_tx::*;
use fuel_tx_test_helpers::TransactionFactory;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn coin() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut factory = TransactionFactory::from_seed(3493);
    let txs = factory.by_ref();

    fn validate_coin_inputs(tx: Transaction) -> Result<(), ValidationError> {
        let txhash = tx.id();
        let outputs = tx.outputs();
        let witnesses = tx.witnesses();

        tx.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| match input {
                Input::Coin { .. } => input.validate(index, &txhash, outputs, witnesses),
                _ => Ok(()),
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn sign_coin_and_validate<R, I>(
        rng: &mut R,
        mut iter: I,
        utxo_id: UtxoId,
        amount: Word,
        asset_id: AssetId,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Result<(), ValidationError>
    where
        R: Rng,
        I: Iterator<Item = (Transaction, Vec<SecretKey>)>,
    {
        let (mut tx, keys) = iter.next().expect("Failed to generate a transaction");

        let secret = SecretKey::random(rng);
        let public = secret.public_key();

        tx.add_unsigned_coin_input(
            utxo_id,
            &public,
            amount,
            asset_id,
            maturity,
            predicate,
            predicate_data,
        );

        tx.sign_inputs(&secret);
        keys.iter().for_each(|sk| tx.sign_inputs(sk));

        validate_coin_inputs(tx)
    }

    txs.take(10)
        .map(|(tx, _)| tx)
        .try_for_each(validate_coin_inputs)
        .expect("Failed to validate transactions");

    let utxo_id = rng.gen();
    let amount = rng.gen();
    let asset_id = rng.gen();
    let maturity = rng.gen();
    sign_coin_and_validate(
        rng,
        txs.by_ref(),
        utxo_id,
        amount,
        asset_id,
        maturity,
        vec![0u8; MAX_PREDICATE_LENGTH as usize],
        vec![],
    )
    .expect("Failed to validate transaction");

    let utxo_id = rng.gen();
    let amount = rng.gen();
    let asset_id = rng.gen();
    let maturity = rng.gen();
    sign_coin_and_validate(
        rng,
        txs.by_ref(),
        utxo_id,
        amount,
        asset_id,
        maturity,
        vec![],
        vec![0u8; MAX_PREDICATE_DATA_LENGTH as usize],
    )
    .expect("Failed to validate transaction");

    let utxo_id = rng.gen();
    let amount = rng.gen();
    let asset_id = rng.gen();
    let maturity = rng.gen();
    let err = sign_coin_and_validate(
        rng,
        txs.by_ref(),
        utxo_id,
        amount,
        asset_id,
        maturity,
        vec![0u8; MAX_PREDICATE_LENGTH as usize + 1],
        vec![],
    )
    .err()
    .expect("Expected failure");

    assert!(matches!(
        err,
        ValidationError::InputCoinPredicateLength { .. }
    ));

    let utxo_id = rng.gen();
    let amount = rng.gen();
    let asset_id = rng.gen();
    let maturity = rng.gen();
    let err = sign_coin_and_validate(
        rng,
        txs.by_ref(),
        utxo_id,
        amount,
        asset_id,
        maturity,
        vec![],
        vec![0u8; MAX_PREDICATE_DATA_LENGTH as usize + 1],
    )
    .err()
    .expect("Expected failure");

    assert!(matches!(
        err,
        ValidationError::InputCoinPredicateDataLength { .. }
    ));

    let mut tx = Transaction::default();

    let input = Input::coin(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        rng.gen(),
        vec![],
        vec![],
    );
    tx.add_input(input);

    let block_height = rng.gen();
    let err = tx.validate(block_height).err().expect("Expected failure");

    assert!(matches!(
        err,
        ValidationError::InputCoinWitnessIndexBounds { .. }
    ));
}

#[test]
fn contract() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.gen();

    Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(
            1,
            &txhash,
            &[Output::contract(1, rng.gen(), rng.gen())],
            &[],
        )
        .unwrap();

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(1, &txhash, &[], &[])
        .err()
        .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(
            1,
            &txhash,
            &[Output::coin(rng.gen(), rng.gen(), rng.gen())],
            &[],
        )
        .err()
        .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(
            1,
            &txhash,
            &[Output::contract(2, rng.gen(), rng.gen())],
            &[],
        )
        .err()
        .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );
}

#[test]
fn transaction_with_duplicate_coin_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let input_utxo_id: UtxoId = rng.gen();
    let input = Input::coin(
        input_utxo_id,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        0,
        vec![],
        vec![],
    );
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(input.clone())
        .add_input(input)
        .finalize();

    let err = tx
        .validate_without_signature(0)
        .err()
        .expect("Expected validation failure");
    assert!(matches!(
        err,
        ValidationError::DuplicateInputUtxoId { utxo_id } if utxo_id == input_utxo_id
    ))
}

#[test]
fn transaction_with_duplicate_contract_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let input_utxo_id: UtxoId = rng.gen();
    let input = Input::contract(input_utxo_id, rng.gen(), rng.gen(), rng.gen());
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(input.clone())
        .add_input(input)
        .finalize();

    let err = tx
        .validate_without_signature(0)
        .err()
        .expect("Expected validation failure");
    assert!(matches!(
        err,
        ValidationError::DuplicateInputUtxoId { utxo_id } if utxo_id == input_utxo_id
    ))
}
