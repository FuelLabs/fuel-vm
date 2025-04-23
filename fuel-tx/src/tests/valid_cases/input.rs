#![allow(clippy::cast_possible_truncation)]

use super::PREDICATE_PARAMS;
use crate::{
    ConsensusParameters,
    builder::TransactionBuilder,
    field,
    field::Witnesses,
    test_helper::{
        TransactionFactory,
        generate_bytes,
        generate_nonempty_padded_bytes,
    },
    *,
};
use fuel_crypto::{
    PublicKey,
    SecretKey,
};
use fuel_types::ChainId;
use rand::{
    CryptoRng,
    Rng,
    SeedableRng,
    rngs::StdRng,
};

#[test]
fn input_coin_message_signature() {
    fn test<Tx: Buildable>(txs: &mut impl Iterator<Item = (Tx, Vec<SecretKey>)>) {
        let rng = &mut StdRng::seed_from_u64(8586);

        fn check_inputs<Tx: Buildable>(tx: Tx) -> Result<(), ValidityError> {
            let chain_id = ChainId::default();
            let txhash = tx.id(&chain_id);
            let outputs = tx.outputs();
            let witnesses = tx.witnesses();

            tx.inputs()
                .iter()
                .enumerate()
                .try_for_each(|(index, input)| match input {
                    Input::CoinSigned(_)
                    | Input::MessageCoinSigned(_)
                    | Input::MessageDataSigned(_) => input.check(
                        index,
                        &txhash,
                        outputs,
                        witnesses,
                        &Default::default(),
                        &mut None,
                    ),
                    _ => Ok(()),
                })
        }

        #[allow(clippy::too_many_arguments)]
        fn sign_and_validate<R, I, F, Tx>(
            rng: &mut R,
            mut iter: I,
            f: F,
        ) -> Result<(), ValidityError>
        where
            R: Rng + CryptoRng,
            I: Iterator<Item = (Tx, Vec<SecretKey>)>,
            F: Fn(&mut Tx, &PublicKey),
            Tx: Buildable,
        {
            let (mut tx, keys) = iter.next().expect("Failed to generate a transaction");

            let secret = SecretKey::random(rng);
            let public = secret.public_key();

            f(&mut tx, &public);

            let chain_id = ChainId::default();

            tx.sign_inputs(&secret, &chain_id);
            keys.iter().for_each(|sk| tx.sign_inputs(sk, &chain_id));

            check_inputs(tx)
        }

        txs.take(10)
            .map(|(tx, _)| tx)
            .try_for_each(check_inputs)
            .expect("Failed to validate transactions");

        for _ in 0..3 {
            let utxo_id = rng.r#gen();
            let amount = rng.r#gen();
            let asset_id = rng.r#gen();
            let tx_pointer = rng.r#gen();

            sign_and_validate(rng, txs.by_ref(), |tx, public| {
                let witness_index = <Tx as field::Witnesses>::witnesses(tx).len();
                <Tx as field::Witnesses>::witnesses_mut(tx).push(Witness::default());
                tx.add_unsigned_coin_input(
                    utxo_id,
                    public,
                    amount,
                    asset_id,
                    tx_pointer,
                    witness_index as u16,
                )
            })
            .expect("Failed to validate transaction");
        }

        for _ in 0..3 {
            let sender = rng.r#gen();
            let nonce = rng.r#gen();
            let amount = rng.r#gen();
            let data = generate_bytes(rng);

            sign_and_validate(rng, txs.by_ref(), |tx, public| {
                let witness_index = <Tx as field::Witnesses>::witnesses(tx).len();
                <Tx as field::Witnesses>::witnesses_mut(tx).push(Witness::default());
                tx.add_unsigned_message_input(
                    sender,
                    Input::owner(public),
                    nonce,
                    amount,
                    data.clone(),
                    witness_index as u16,
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
    let mut tx = TransactionBuilder::script(vec![], vec![]).finalize();

    let input = Input::coin_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
    );
    tx.add_input(input);

    let block_height = rng.r#gen();
    let err = tx
        .check(block_height, &ConsensusParameters::standard())
        .expect_err("Expected failure");

    assert_eq!(ValidityError::InputWitnessIndexBounds { index: 0 }, err);
}

#[test]
fn duplicate_secrets_reuse_witness() {
    let rng = &mut StdRng::seed_from_u64(10000);
    let key = SecretKey::random(rng);

    let script = TransactionBuilder::script(vec![], vec![])
        // coin 1
        .add_unsigned_coin_input(key, rng.r#gen(), 100, Default::default(), Default::default())
        // coin 2
        .add_unsigned_coin_input(key, rng.r#gen(), 200, rng.r#gen(), Default::default())
        // message 1
        .add_unsigned_message_input(key, rng.r#gen(), rng.r#gen(), 100, vec![])
        .add_unsigned_message_input(key, rng.r#gen(), rng.r#gen(), 100, vec![rng.r#gen()])
        .finalize();

    assert_eq!(
        script.witnesses().len(),
        1,
        "Script should only have one witness as only one private key is used"
    );

    // verify witness reuse for creation txs
    let create = TransactionBuilder::create(Witness::default(), rng.r#gen(), vec![])
        // coin 1
        .add_unsigned_coin_input(key, rng.r#gen(), 100, Default::default(), Default::default())
        // coin 2
        .add_unsigned_coin_input(key, rng.r#gen(), 200, rng.r#gen(), Default::default())
        // message 1
        .add_unsigned_message_input(key, rng.r#gen(), rng.r#gen(), 100, vec![])
        .add_unsigned_message_input(key, rng.r#gen(), rng.r#gen(), 100, vec![rng.r#gen()])
        .finalize();

    assert_eq!(
        create.witnesses().len(),
        2,
        "Create should only have two witnesses (bytecode + signature) as only one private key is used"
    )
}

#[test]
fn coin_predicate() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.r#gen();

    let predicate = generate_nonempty_padded_bytes(rng);
    let owner = Input::predicate_owner(&predicate);

    Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .unwrap();

    let predicate = vec![];
    let owner = Input::predicate_owner(&predicate);

    let err = Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .err()
    .unwrap();

    assert_eq!(ValidityError::InputPredicateEmpty { index: 1 }, err);

    let mut predicate = generate_nonempty_padded_bytes(rng);
    let owner = (*Contract::root_from_code(&predicate)).into();
    predicate[0] = predicate[0].wrapping_add(1);

    let err = Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .err()
    .unwrap();

    assert_eq!(ValidityError::InputPredicateOwner { index: 1 }, err);
}

#[test]
fn contract() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let txhash: Bytes32 = rng.r#gen();

    Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .check(
        1,
        &txhash,
        &[Output::contract(1, rng.r#gen(), rng.r#gen())],
        &[],
        &Default::default(),
        &mut None,
    )
    .unwrap();

    let err = Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .err()
    .unwrap();

    assert_eq!(
        ValidityError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .check(
        1,
        &txhash,
        &[Output::coin(rng.r#gen(), rng.r#gen(), rng.r#gen())],
        &[],
        &Default::default(),
        &mut None,
    )
    .err()
    .unwrap();

    assert_eq!(
        ValidityError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )
    .check(
        1,
        &txhash,
        &[Output::contract(2, rng.r#gen(), rng.r#gen())],
        &[],
        &Default::default(),
        &mut None,
    )
    .err()
    .unwrap();

    assert_eq!(
        ValidityError::InputContractAssociatedOutputContract { index: 1 },
        err
    );
}

#[test]
fn message_metadata() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let txhash: Bytes32 = rng.r#gen();

    let predicate = generate_nonempty_padded_bytes(rng);
    let recipient = Input::predicate_owner(&predicate);

    Input::message_data_predicate(
        rng.r#gen(),
        recipient,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        generate_bytes(rng),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect("failed to validate empty message input");

    let mut tx = TransactionBuilder::script(vec![], vec![]).finalize();

    let input = Input::message_data_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
        generate_bytes(rng),
    );
    let fee_input =
        Input::message_coin_signed(rng.r#gen(), rng.r#gen(), rng.r#gen(), rng.r#gen(), 1);

    tx.add_input(input);
    tx.add_input(fee_input);

    let block_height = rng.r#gen();
    let err = tx
        .check(block_height, &ConsensusParameters::standard())
        .expect_err("Expected failure");

    assert_eq!(ValidityError::InputWitnessIndexBounds { index: 0 }, err,);

    let mut predicate = generate_nonempty_padded_bytes(rng);
    let recipient = Input::predicate_owner(&predicate);
    predicate[0] = predicate[0].wrapping_add(1);

    let err = Input::message_data_predicate(
        rng.r#gen(),
        recipient,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        generate_bytes(rng),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("Expected failure");

    assert_eq!(ValidityError::InputPredicateOwner { index: 1 }, err);

    let data = vec![0xff; PREDICATE_PARAMS.max_message_data_length() as usize + 1];

    let err = Input::message_data_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
        data.clone(),
    )
    .check(
        1,
        &txhash,
        &[],
        &[vec![].into()],
        &Default::default(),
        &mut None,
    )
    .expect_err("expected max data length error");

    assert_eq!(ValidityError::InputMessageDataLength { index: 1 }, err,);

    let err = Input::message_data_predicate(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        data,
        generate_nonempty_padded_bytes(rng),
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("expected max data length error");

    assert_eq!(ValidityError::InputMessageDataLength { index: 1 }, err,);

    let predicate = vec![0xff; PREDICATE_PARAMS.max_predicate_length() as usize + 1];

    let err = Input::message_data_predicate(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        generate_bytes(rng),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("expected max predicate length error");

    assert_eq!(ValidityError::InputPredicateLength { index: 1 }, err,);

    let predicate_data =
        vec![0xff; PREDICATE_PARAMS.max_predicate_data_length() as usize + 1];

    let err = Input::message_data_predicate(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        generate_bytes(rng),
        generate_bytes(rng),
        predicate_data,
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("expected max predicate data length error");

    assert_eq!(ValidityError::InputPredicateDataLength { index: 1 }, err,);
}

#[test]
fn message_message_coin() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let txhash: Bytes32 = rng.r#gen();

    let predicate = generate_nonempty_padded_bytes(rng);
    let recipient = Input::predicate_owner(&predicate);

    Input::message_coin_predicate(
        rng.r#gen(),
        recipient,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect("failed to validate empty message input");

    let mut tx = TransactionBuilder::script(vec![], vec![]).finalize();

    let input =
        Input::message_coin_signed(rng.r#gen(), rng.r#gen(), rng.r#gen(), rng.r#gen(), 0);
    tx.add_input(input);

    let block_height = rng.r#gen();
    let err = tx
        .check(block_height, &ConsensusParameters::standard())
        .expect_err("Expected failure");

    assert_eq!(ValidityError::InputWitnessIndexBounds { index: 0 }, err,);

    let mut predicate = generate_nonempty_padded_bytes(rng);
    let recipient = Input::predicate_owner(&predicate);
    predicate[0] = predicate[0].wrapping_add(1);

    let err = Input::message_coin_predicate(
        rng.r#gen(),
        recipient,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("Expected failure");

    assert_eq!(ValidityError::InputPredicateOwner { index: 1 }, err);

    let predicate = vec![0xff; PREDICATE_PARAMS.max_predicate_length() as usize + 1];

    let err = Input::message_coin_predicate(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        predicate,
        generate_bytes(rng),
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("expected max predicate length error");

    assert_eq!(ValidityError::InputPredicateLength { index: 1 }, err,);

    let predicate_data =
        vec![0xff; PREDICATE_PARAMS.max_predicate_data_length() as usize + 1];

    let err = Input::message_coin_predicate(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        generate_bytes(rng),
        predicate_data,
    )
    .check(1, &txhash, &[], &[], &Default::default(), &mut None)
    .expect_err("expected max predicate data length error");

    assert_eq!(ValidityError::InputPredicateDataLength { index: 1 }, err,);
}

#[test]
fn transaction_with_duplicate_coin_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let utxo_id = rng.r#gen();

    let a = Input::coin_signed(
        utxo_id,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
    );
    let b = Input::coin_signed(
        utxo_id,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
    );

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_witness(rng.r#gen())
        .finalize()
        .check_without_signatures(Default::default(), &ConsensusParameters::standard())
        .expect_err("Expected checkable failure");

    assert_eq!(err, ValidityError::DuplicateInputUtxoId { utxo_id });
}

#[test]
fn transaction_with_duplicate_message_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let message_input = Input::message_data_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
        generate_bytes(rng),
    );
    let nonce = message_input.nonce().cloned().unwrap();
    let fee = Input::coin_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    );

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(fee)
        .add_input(message_input.clone())
        // duplicate input
        .add_input(message_input)
        .add_witness(rng.r#gen())
        .finalize()
        .check_without_signatures(
            Default::default(),
            &ConsensusParameters::standard(),
        )
        .expect_err("Expected checkable failure");

    assert_eq!(err, ValidityError::DuplicateInputNonce { nonce });
}

#[test]
fn transaction_with_duplicate_contract_inputs_is_invalid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let contract_id = rng.r#gen();
    let fee = Input::coin_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    );

    let a = Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        contract_id,
    );
    let b = Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        contract_id,
    );

    let o = Output::contract(0, rng.r#gen(), rng.r#gen());
    let p = Output::contract(1, rng.r#gen(), rng.r#gen());

    let err = TransactionBuilder::script(vec![], vec![])
        .add_input(fee)
        .add_input(a)
        .add_input(b)
        .add_output(o)
        .add_output(p)
        .finalize()
        .check_without_signatures(Default::default(), &ConsensusParameters::standard())
        .expect_err("Expected checkable failure");

    assert_eq!(err, ValidityError::DuplicateInputContractId { contract_id });
}

#[test]
fn transaction_with_duplicate_contract_utxo_id_is_valid() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let input_utxo_id: UtxoId = rng.r#gen();

    let a = Input::contract(
        input_utxo_id,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    );
    let b = Input::contract(
        input_utxo_id,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    );
    let fee = Input::coin_signed(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
    );

    let o = Output::contract(0, rng.r#gen(), rng.r#gen());
    let p = Output::contract(1, rng.r#gen(), rng.r#gen());

    TransactionBuilder::script(vec![], vec![])
        .add_input(a)
        .add_input(b)
        .add_input(fee)
        .add_output(o)
        .add_output(p)
        .add_witness(rng.r#gen())
        .finalize()
        .check_without_signatures(Default::default(), &ConsensusParameters::standard())
        .expect("Duplicated UTXO id is valid for contract input");
}
