#![allow(clippy::cast_possible_truncation)]
#![allow(non_snake_case)]

use super::*;
use crate::field::Witnesses;
use fuel_asm::op;
use fuel_types::{
    BlobId,
    BlockHeight,
};

// Creates a predicate that always is valid - returns `true`.
fn predicate() -> Vec<u8> {
    vec![op::ret(1)].into_iter().collect::<Vec<u8>>()
}

fn test_params() -> ConsensusParameters {
    ConsensusParameters::default()
}

fn valid_blob_transaction() -> TransactionBuilder<Blob> {
    let blob_data = vec![1; 100];
    let mut builder = TransactionBuilder::blob(BlobBody {
        id: BlobId::compute(&blob_data),
        witness_index: 0,
    });
    builder.add_witness(blob_data.into());
    builder.max_fee_limit(0);
    builder.add_input(Input::coin_predicate(
        Default::default(),
        Input::predicate_owner(predicate()),
        Default::default(),
        AssetId::BASE,
        Default::default(),
        Default::default(),
        predicate(),
        vec![],
    ));
    builder.expiration(u32::MAX.into());

    builder
}

#[test]
fn check__valid_blob_transaction_passes_check() {
    // Given
    let block_height: BlockHeight = 1000.into();
    let tx = valid_blob_transaction().finalize();

    // When
    let tx = tx.check(block_height, &test_params());

    // Then
    assert_eq!(tx, Ok(()));
}

#[test]
fn check__fails_if_maturity_not_met() {
    // Given
    let block_height: BlockHeight = 1000.into();
    let failing_block_height = block_height.succ().unwrap();
    let tx = valid_blob_transaction()
        .maturity(failing_block_height)
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionMaturity), result);
}

#[test]
fn check__success_if_expiration_met() {
    // Given
    let block_height: BlockHeight = 1000.into();
    let success_block_height = block_height.succ().unwrap();
    let tx = valid_blob_transaction()
        .expiration(success_block_height)
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Ok(()), result);
}

#[test]
fn check__fails_if_expiration_not_met() {
    // Given
    let block_height: BlockHeight = 1000.into();
    let failing_block_height = block_height.pred().unwrap();
    let tx = valid_blob_transaction()
        .expiration(failing_block_height)
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionExpiration), result);
}

#[test]
fn check__fails_if_owner_bad_index() {
    // Given
    let block_height: BlockHeight = 1000.into();
    let tx = valid_blob_transaction().owner(1).finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionOwnerIndexOutOfBounds), result);
}

#[test]
fn check__fails_if_blob_id_doesnt_match_payload() {
    // Given
    let blob_data = vec![1; 100];
    let mut builder = TransactionBuilder::blob(BlobBody {
        id: BlobId::from_bytes(&[0xf0; 32]).unwrap(),
        witness_index: 0,
    });
    builder.add_witness(blob_data.into());
    builder.max_fee_limit(0);
    builder.add_input(Input::coin_predicate(
        Default::default(),
        Input::predicate_owner(predicate()),
        Default::default(),
        AssetId::BASE,
        Default::default(),
        Default::default(),
        predicate(),
        vec![],
    ));

    let block_height: BlockHeight = 1000.into();
    let tx = builder.maturity(block_height).finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionBlobIdVerificationFailed),
        result
    );
}

#[test]
fn check__not_set_witness_limit_success() {
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction().finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn check__set_witness_limit_for_empty_witness_success() {
    // Given
    let block_height = 1000.into();
    let subsection_size = valid_blob_transaction()
        .finalize()
        .witnesses()
        .size_dynamic();
    let limit = subsection_size + Signature::LEN + vec![0u8; 0].size_static();
    let tx = valid_blob_transaction()
        .witness_limit(limit as u64)
        .add_witness(vec![0; Signature::LEN].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Ok(()), result);
}

#[test]
fn check__set_witness_limit_less_than_witness_data_size_fails() {
    let block_height = 1000.into();
    let subsection_size = valid_blob_transaction()
        .finalize()
        .witnesses()
        .size_dynamic();
    let limit = subsection_size + Signature::LEN + vec![0u8; 0].size_static();

    // Given
    let failing_limit = limit - 1;
    let tx = valid_blob_transaction()
        .witness_limit(failing_limit as u64)
        .add_witness(vec![0; Signature::LEN].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionWitnessLimitExceeded), result);
}

#[test]
fn check__no_max_fee_fails() {
    let block_height = 1000.into();
    let mut tx = valid_blob_transaction().add_fee_input().finalize();

    // Given
    tx.policies_mut().set(PolicyType::MaxFee, None);

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionMaxFeeNotSet), result);
}

#[test]
fn check__reached_max_inputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_blob_transaction();

    while builder.outputs().len() < test_params().tx_params().max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), AssetId::BASE));
    }

    // Given
    let secrets: Vec<SecretKey> =
        (0..1 + test_params().tx_params().max_inputs() as usize - builder.inputs().len())
            .map(|_| SecretKey::random(rng))
            .collect();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::BASE,
            rng.r#gen(),
        );
    });
    while builder.witnesses().len() < test_params().tx_params().max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }
    let tx = builder.finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionInputsMax), result);
}

#[test]
fn check__reached_max_outputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_blob_transaction();

    let secrets: Vec<SecretKey> = (0..test_params().tx_params().max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::BASE,
            rng.r#gen(),
        );
    });
    while builder.witnesses().len() < test_params().tx_params().max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    // Given
    while builder.outputs().len() < test_params().tx_params().max_outputs() as usize + 1 {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), AssetId::BASE));
    }
    let tx = builder.finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionOutputsMax), result);
}

#[test]
fn check__reached_max_witnesses() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_blob_transaction();

    let secrets: Vec<SecretKey> = (0..test_params().tx_params().max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.r#gen(),
            rng.r#gen(),
            AssetId::BASE,
            rng.r#gen(),
        );
    });
    while builder.outputs().len() < test_params().tx_params().max_outputs() as usize {
        builder.add_output(Output::coin(rng.r#gen(), rng.r#gen(), AssetId::BASE));
    }

    // Given
    while builder.witnesses().len()
        < test_params().tx_params().max_witnesses() as usize + 1
    {
        builder.add_witness(generate_bytes(rng).into());
    }
    let tx = builder.finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionWitnessesMax), result);
}

#[test]
fn check__fail_if_output_change_asset_id_is_duplicated() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let secret = SecretKey::random(rng);

    // Given
    let a: AssetId = rng.r#gen();
    let tx = valid_blob_transaction()
        .add_unsigned_coin_input(secret, rng.r#gen(), rng.r#gen(), a, rng.r#gen())
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), a))
        .finalize();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionOutputChangeAssetIdDuplicated(a)),
        result
    );
}

#[test]
fn check__fail_if_output_asset_id_not_in_inputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let c: AssetId = rng.r#gen();
    let tx = valid_blob_transaction()
        .add_output(Output::change(rng.r#gen(), rng.next_u64(), c))
        .finalize();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionOutputChangeAssetIdNotFound(c)),
        result
    );
}

#[test]
fn check__cannot_have_contract_input() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction()
        .add_input(Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ))
        .add_output(Output::contract(1, rng.r#gen(), rng.r#gen()))
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionInputContainsContract { index: 1 }),
        result
    );
}

#[test]
fn check__cannot_have_coin_with_non_base_asset_id() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let secret = SecretKey::random(rng);

    // Given
    let tx = valid_blob_transaction()
        .add_unsigned_coin_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionInputContainsNonBaseAssetId { index: 1 }),
        result
    );
}

#[test]
fn check__can_have_message_coin_input() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let secret = SecretKey::random(rng);

    // Given
    let empty_data = vec![];
    let tx = valid_blob_transaction()
        .add_unsigned_message_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            empty_data,
        )
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Ok(()), result);
}

#[test]
fn check__cannot_have_message_data_input() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let secret = SecretKey::random(rng);

    // Given
    let not_empty_data = vec![0x1];
    let tx = valid_blob_transaction()
        .add_unsigned_message_input(
            secret,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            not_empty_data,
        )
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionInputContainsMessageData { index: 1 }),
        result
    );
}

#[test]
fn check__cannot_have_variable_output() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction()
        .add_output(Output::variable(rng.r#gen(), rng.r#gen(), rng.r#gen()))
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionOutputContainsVariable { index: 0 }),
        result
    );
}

#[test]
fn check__cannot_have_contract_output() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction()
        .add_output(Output::Contract(rng.r#gen()))
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::OutputContractInputIndex { index: 0 }),
        result
    );
}

#[test]
fn check__cannot_have_create_contract_output() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction()
        .add_output(Output::contract_created(rng.r#gen(), rng.r#gen()))
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionOutputContainsContractCreated { index: 0 }),
        result
    );
}

#[test]
fn check__can_have_change_output() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction()
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), AssetId::BASE))
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Ok(()), result);
}

#[test]
fn check__errors_if_change_is_wrong_asset() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let a: AssetId = rng.r#gen();
    let tx = valid_blob_transaction()
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            rng.r#gen(),
            a,
            rng.r#gen(),
        )
        .add_output(Output::change(rng.r#gen(), rng.r#gen(), a))
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionInputContainsNonBaseAssetId { index: 1 }),
        result,
    );
}

#[test]
fn check__errors_when_transactions_too_big() {
    let block_height = 1000.into();

    // Given
    let tx = valid_blob_transaction()
        .add_witness(vec![0; test_params().tx_params().max_size() as usize].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionSizeLimitExceeded), result);
}

mod inputs {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn check__succeeds_with_correct_coin_predicate_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.r#gen()).collect_vec();
        let owner: Address = Input::predicate_owner(&predicate);

        // Given
        let tx = valid_blob_transaction()
            .add_input(Input::coin_predicate(
                rng.r#gen(),
                owner,
                rng.r#gen(),
                AssetId::BASE,
                rng.r#gen(),
                0,
                predicate,
                vec![],
            ))
            .finalize_as_transaction();

        // When
        let result = tx.check(block_height, &test_params());

        // Then
        assert_eq!(Ok(()), result);
    }

    #[test]
    fn check__fails_with_incorrect_coin_predicate_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.r#gen()).collect_vec();
        let incorrect_owner: Address = [1; 32].into();

        // Given
        let tx = valid_blob_transaction()
            .add_input(Input::coin_predicate(
                rng.r#gen(),
                incorrect_owner,
                rng.r#gen(),
                AssetId::BASE,
                rng.r#gen(),
                0,
                predicate,
                vec![],
            ))
            .finalize_as_transaction();

        // When
        let result = tx.check(block_height, &test_params());

        // Then
        assert_eq!(Err(ValidityError::InputPredicateOwner { index: 1 }), result);
    }

    #[test]
    fn check__succeeds_with_correct_coin_predicate_input_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.r#gen()).collect_vec();
        let owner: Address = Input::predicate_owner(&predicate);

        // Given
        let tx = valid_blob_transaction()
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                owner,
                rng.r#gen(),
                rng.r#gen(),
                0,
                predicate,
                vec![],
            ))
            .finalize_as_transaction();

        // When
        let result = tx.check(block_height, &test_params());

        // Then
        assert_eq!(Ok(()), result);
    }

    #[test]
    fn check__fails_with_incorrect_message_predicate_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.r#gen()).collect_vec();
        let incorrect_owner: Address = [1; 32].into();

        // Given
        let tx = valid_blob_transaction()
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                incorrect_owner,
                rng.r#gen(),
                rng.r#gen(),
                0,
                predicate,
                vec![],
            ))
            .finalize_as_transaction();

        // When
        let result = tx.check(block_height, &test_params());

        // Then
        assert_eq!(Err(ValidityError::InputPredicateOwner { index: 1 }), result);
    }
}
