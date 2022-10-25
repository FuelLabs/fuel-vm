use super::PARAMS;

use fuel_crypto::{PublicKey, SecretKey};
use fuel_tx::*;
use fuel_tx_test_helpers::{generate_bytes, generate_nonempty_padded_bytes, TransactionFactory};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn input_coin_message_signature() {
    fn test<Tx: Buildable>(txs: &mut impl Iterator<Item = (Tx, Vec<SecretKey>)>) {
        let rng = &mut StdRng::seed_from_u64(8586);

        fn check_inputs<Tx: Buildable>(tx: Tx) -> Result<(), CheckError> {
            let txhash = tx.id();
            let outputs = tx.outputs();
            let witnesses = tx.witnesses();

            tx.inputs()
                .iter()
                .enumerate()
                .try_for_each(|(index, input)| match input {
                    Input::CoinSigned { .. } | Input::MessageSigned { .. } => {
                        input.check(index, &txhash, outputs, witnesses, &Default::default())
                    }
                    _ => Ok(()),
                })
        }

        #[allow(clippy::too_many_arguments)]
        fn sign_and_validate<R, I, F, Tx>(rng: &mut R, mut iter: I, f: F) -> Result<(), CheckError>
        where
            R: Rng,
            I: Iterator<Item = (Tx, Vec<SecretKey>)>,
            F: Fn(&mut Tx, &PublicKey),
            Tx: Buildable,
        {
            let (mut tx, keys) = iter.next().expect("Failed to generate a transaction");

            let secret = SecretKey::random(rng);
            let public = secret.public_key();

            f(&mut tx, &public);

            tx.sign_inputs(&secret);
            keys.iter().for_each(|sk| tx.sign_inputs(sk));

            check_inputs(tx)
        }

        txs.take(10)
            .map(|(tx, _)| tx)
            .try_for_each(check_inputs)
            .expect("Failed to validate transactions");

        for _ in 0..3 {
            let utxo_id = rng.gen();
            let amount = rng.gen();
            let asset_id = rng.gen();
            let tx_pointer = rng.gen();
            let maturity = rng.gen();

            sign_and_validate(rng, txs.by_ref(), |tx, public| {
                tx.add_unsigned_coin_input(utxo_id, public, amount, asset_id, tx_pointer, maturity)
            })
            .expect("Failed to validate transaction");
        }

        for _ in 0..3 {
            let sender = rng.gen();
            let nonce = rng.gen();
            let amount = rng.gen();
            let data = generate_bytes(rng);

            sign_and_validate(rng, txs.by_ref(), |tx, public| {
                tx.add_unsigned_message_input(
                    sender,
                    Input::owner(public),
                    nonce,
                    amount,
                    data.clone(),
                )
            })
            .expect("Failed to validate transaction");
        }
    }

    let mut factory = TransactionFactory::<_, Script>::from_seed(3493);
    let txs = factory.by_ref();
    test(txs);
}

#[test]
fn coin_signed() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut tx = Script::default();

    let input = Input::coin_signed(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        rng.gen(),
    );
    tx.add_input(input);

    let block_height = rng.gen();
    let err = tx
        .check(block_height, &Default::default())
        .expect_err("Expected failure");

    assert_eq!(CheckError::InputWitnessIndexBounds { index: 0 }, err);
}

#[test]
fn coin_predicate() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.gen();

    let predicate = generate_nonempty_padded_bytes(rng);
    let owner = (*Contract::root_from_code(&predicate)).into();

    Input::coin_predicate(
        rng.gen(),
        owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .unwrap();

    let predicate = vec![];
    let owner = (*Contract::root_from_code(&predicate)).into();

    let err = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .err()
    .unwrap();

    assert_eq!(CheckError::InputPredicateEmpty { index: 1 }, err);

    let mut predicate = generate_nonempty_padded_bytes(rng);
    let owner = (*Contract::root_from_code(&predicate)).into();
    predicate[0] = predicate[0].wrapping_add(1);

    let err = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .err()
    .unwrap();

    assert_eq!(CheckError::InputPredicateOwner { index: 1 }, err);
}

#[test]
fn contract() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.gen();

    Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .check(
            1,
            &txhash,
            &[Output::contract(1, rng.gen(), rng.gen())],
            &[],
            &Default::default(),
        )
        .unwrap();

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .check(1, &txhash, &[], &[], &Default::default())
        .err()
        .unwrap();

    assert_eq!(
        CheckError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .check(
            1,
            &txhash,
            &[Output::coin(rng.gen(), rng.gen(), rng.gen())],
            &[],
            &Default::default(),
        )
        .err()
        .unwrap();

    assert_eq!(
        CheckError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .check(
            1,
            &txhash,
            &[Output::contract(2, rng.gen(), rng.gen())],
            &[],
            &Default::default(),
        )
        .err()
        .unwrap();

    assert_eq!(
        CheckError::InputContractAssociatedOutputContract { index: 1 },
        err
    );
}

#[test]
fn message() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.gen();

    let predicate = generate_nonempty_padded_bytes(rng);
    let recipient = (*Contract::root_from_code(&predicate)).into();

    Input::message_predicate(
        rng.gen(),
        rng.gen(),
        recipient,
        rng.gen(),
        rng.gen(),
        generate_bytes(rng),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .expect("failed to validate empty message input");

    let mut tx = Script::default();

    let input = Input::message_signed(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        generate_bytes(rng),
    );

    tx.add_input(input);

    let block_height = rng.gen();
    let err = tx
        .check(block_height, &Default::default())
        .expect_err("Expected failure");

    assert_eq!(CheckError::InputWitnessIndexBounds { index: 0 }, err,);

    let mut predicate = generate_nonempty_padded_bytes(rng);
    let recipient = (*Contract::root_from_code(&predicate)).into();
    predicate[0] = predicate[0].wrapping_add(1);

    let err = Input::message_predicate(
        rng.gen(),
        rng.gen(),
        recipient,
        rng.gen(),
        rng.gen(),
        generate_bytes(rng),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .expect_err("Expected failure");

    assert_eq!(CheckError::InputPredicateOwner { index: 1 }, err);

    let data = vec![0xff; PARAMS.max_message_data_length as usize + 1];

    let err = Input::message_signed(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        data.clone(),
    )
    .check(1, &txhash, &[], &[vec![].into()], &Default::default())
    .expect_err("expected max data length error");

    assert_eq!(CheckError::InputMessageDataLength { index: 1 }, err,);

    let err = Input::message_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        data,
        generate_nonempty_padded_bytes(rng),
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .expect_err("expected max data length error");

    assert_eq!(CheckError::InputMessageDataLength { index: 1 }, err,);

    let predicate = vec![0xff; PARAMS.max_predicate_length as usize + 1];

    let err = Input::message_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        generate_bytes(rng),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .expect_err("expected max predicate length error");

    assert_eq!(CheckError::InputPredicateLength { index: 1 }, err,);

    let predicate_data = vec![0xff; PARAMS.max_predicate_data_length as usize + 1];

    let err = Input::message_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        generate_bytes(rng),
        generate_bytes(rng),
        predicate_data,
    )
    .check(1, &txhash, &[], &[], &Default::default())
    .expect_err("expected max predicate data length error");

    assert_eq!(CheckError::InputPredicateDataLength { index: 1 }, err,);
}

#[test]
fn transaction_with_duplicate_coin_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let utxo_id = rng.gen();

    let a = Input::coin_signed(
        utxo_id,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        rng.gen(),
    );
    let b = Input::coin_signed(
        utxo_id,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        rng.gen(),
    );

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_witness(rng.gen())
        .finalize()
        .check_without_signatures(0, &Default::default())
        .expect_err("Expected checkable failure");

    assert_eq!(err, CheckError::DuplicateInputUtxoId { utxo_id });
}

#[test]
fn transaction_with_duplicate_message_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let message_id = rng.gen();

    let message_input = Input::message_signed(
        message_id,
        rng.gen(),
        rng.gen(),
        rng.gen(),
        0,
        0,
        generate_bytes(rng),
    );

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(message_input.clone())
        // duplicate input
        .add_input(message_input)
        .add_witness(rng.gen())
        .finalize()
        .check_without_signatures(0, &Default::default())
        .expect_err("Expected checkable failure");

    assert_eq!(err, CheckError::DuplicateMessageInputId { message_id });
}

#[test]
fn transaction_with_duplicate_contract_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let contract_id = rng.gen();

    let a = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_id);
    let b = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_id);

    let o = Output::contract(0, rng.gen(), rng.gen());
    let p = Output::contract(1, rng.gen(), rng.gen());

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_output(o)
        .add_output(p)
        .finalize()
        .check_without_signatures(0, &Default::default())
        .expect_err("Expected checkable failure");

    assert_eq!(err, CheckError::DuplicateInputContractId { contract_id });
}

#[test]
fn transaction_with_duplicate_contract_utxo_id_is_valid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let input_utxo_id: UtxoId = rng.gen();

    let a = Input::contract(input_utxo_id, rng.gen(), rng.gen(), rng.gen(), rng.gen());
    let b = Input::contract(input_utxo_id, rng.gen(), rng.gen(), rng.gen(), rng.gen());

    let o = Output::contract(0, rng.gen(), rng.gen());
    let p = Output::contract(1, rng.gen(), rng.gen());

    TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_output(o)
        .add_output(p)
        .finalize()
        .check_without_signatures(0, &Default::default())
        .expect("Duplicated UTXO id is valid for contract input");
}
