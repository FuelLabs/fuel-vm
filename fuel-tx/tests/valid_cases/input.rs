use fuel_crypto::SecretKey;
use fuel_tx::*;
use fuel_tx_test_helpers::{generate_bytes, generate_nonempty_bytes, TransactionFactory};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn coin_signed() {
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
                Input::CoinSigned { .. } => {
                    input.validate(index, &txhash, outputs, witnesses, &Default::default())
                }
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
    ) -> Result<(), ValidationError>
    where
        R: Rng,
        I: Iterator<Item = (Transaction, Vec<SecretKey>)>,
    {
        let (mut tx, keys) = iter.next().expect("Failed to generate a transaction");

        let secret = SecretKey::random(rng);
        let public = secret.public_key();

        tx.add_unsigned_coin_input(utxo_id, &public, amount, asset_id, maturity);

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
    sign_coin_and_validate(rng, txs.by_ref(), utxo_id, amount, asset_id, maturity)
        .expect("Failed to validate transaction");

    let utxo_id = rng.gen();
    let amount = rng.gen();
    let asset_id = rng.gen();
    let maturity = rng.gen();
    sign_coin_and_validate(rng, txs.by_ref(), utxo_id, amount, asset_id, maturity)
        .expect("Failed to validate transaction");

    let mut tx = Transaction::default();

    let input = Input::coin_signed(rng.gen(), rng.gen(), rng.gen(), rng.gen(), 0, rng.gen());
    tx.add_input(input);

    let block_height = rng.gen();
    let err = tx
        .validate(block_height, &Default::default())
        .err()
        .expect("Expected failure");

    assert!(matches!(
        err,
        ValidationError::InputCoinWitnessIndexBounds { .. }
    ));
}

#[test]
fn coin_predicate() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.gen();

    let predicate = generate_nonempty_bytes(rng);
    let owner = (*Contract::root_from_code(&predicate)).into();

    Input::coin_predicate(
        rng.gen(),
        owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate,
        generate_bytes(rng),
    )
    .validate(1, &txhash, &[], &[], &Default::default())
    .unwrap();

    let predicate = vec![];
    let owner = (*Contract::root_from_code(&predicate)).into();

    let err = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate,
        generate_bytes(rng),
    )
    .validate(1, &txhash, &[], &[], &Default::default())
    .err()
    .unwrap();

    assert_eq!(ValidationError::InputCoinPredicateLength { index: 1 }, err);

    let mut predicate = generate_nonempty_bytes(rng);
    let owner = (*Contract::root_from_code(&predicate)).into();
    predicate[0] = predicate[0].wrapping_add(1);

    let err = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate,
        generate_bytes(rng),
    )
    .validate(1, &txhash, &[], &[], &Default::default())
    .err()
    .unwrap();

    assert_eq!(ValidationError::InputCoinPredicateOwner { index: 1 }, err);
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
            &Default::default(),
        )
        .unwrap();

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(1, &txhash, &[], &[], &Default::default())
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
            &Default::default(),
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
            &Default::default(),
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
    let utxo_id = rng.gen();

    let a = Input::coin_signed(utxo_id, rng.gen(), rng.gen(), rng.gen(), 0, rng.gen());
    let b = Input::coin_signed(utxo_id, rng.gen(), rng.gen(), rng.gen(), 0, rng.gen());

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_witness(rng.gen())
        .finalize()
        .validate_without_signature(0, &Default::default())
        .err()
        .expect("Expected validation failure");

    assert_eq!(err, ValidationError::DuplicateInputUtxoId { utxo_id });
}

#[test]
fn transaction_with_duplicate_contract_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let contract_id = rng.gen();

    let a = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract_id);
    let b = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract_id);

    let o = Output::contract(0, rng.gen(), rng.gen());
    let p = Output::contract(1, rng.gen(), rng.gen());

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_output(o)
        .add_output(p)
        .finalize()
        .validate_without_signature(0, &Default::default())
        .err()
        .expect("Expected validation failure");

    assert_eq!(
        err,
        ValidationError::DuplicateInputContractId { contract_id }
    );
}

#[test]
fn transaction_with_duplicate_contract_utxo_id_is_valid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let input_utxo_id: UtxoId = rng.gen();

    let a = Input::contract(input_utxo_id, rng.gen(), rng.gen(), rng.gen());
    let b = Input::contract(input_utxo_id, rng.gen(), rng.gen(), rng.gen());

    let o = Output::contract(0, rng.gen(), rng.gen());
    let p = Output::contract(1, rng.gen(), rng.gen());

    TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_output(o)
        .add_output(p)
        .finalize()
        .validate_without_signature(0, &Default::default())
        .expect("Duplicated UTXO id is valid for contract input");
}
