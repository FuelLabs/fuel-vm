#![allow(clippy::cast_possible_truncation)]
#![allow(non_snake_case)]

use super::*;
use crate::field::{
    BytecodeRoot,
    BytecodeWitnessIndex,
    PartIndex,
    PartsNumber,
    ProofSet,
    Witnesses,
};
use fuel_asm::op;
use fuel_types::BlockHeight;
use std::ops::Deref;

const PART_SIZE: usize = 256;

fn bytecode() -> Vec<u8> {
    vec![op::ret(1); 4321].into_iter().collect::<Vec<u8>>()
}

// Creates a predicate that always is valid - returns `true`.
fn predicate() -> Vec<u8> {
    vec![op::ret(1)].into_iter().collect::<Vec<u8>>()
}

fn test_params() -> ConsensusParameters {
    ConsensusParameters::default()
}

fn valid_upload_transaction() -> TransactionBuilder<Upload> {
    let parts = UploadPart::split_bytecode(&bytecode(), PART_SIZE)
        .expect("Should be able to split bytecode");
    let part = parts[0].clone();
    let mut builder = TransactionBuilder::upload(UploadBody {
        root: part.root,
        witness_index: 0,
        part_index: part.part_index,
        parts_number: part.parts_number,
        proof_set: part.proof_set,
    });
    builder.add_witness(part.part_bytecode.into());
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

    builder
}

#[test]
fn split_bytecode__can_recover_bytecode() {
    // Given
    let parts = UploadPart::split_bytecode(&bytecode(), PART_SIZE)
        .expect("Should be able to split bytecode");
    let expected_root = parts[0].root;
    let len = parts.len();
    let mut recovered_bytecode = vec![];
    let mut recovered_merkle = fuel_merkle::binary::in_memory::MerkleTree::new();

    // When
    for (i, part) in parts.into_iter().enumerate() {
        recovered_merkle.push(&part.part_bytecode);
        recovered_bytecode.extend_from_slice(&part.part_bytecode);

        // Then
        assert_eq!(expected_root, part.root);
        assert!(!part.part_bytecode.is_empty());
        assert_eq!(i, part.part_index as usize);
        assert_eq!(len, part.parts_number as usize);
        assert!(!part.proof_set.is_empty());
    }

    // Then
    assert_eq!(bytecode(), recovered_bytecode);
    assert_eq!(expected_root, recovered_merkle.root().into());
}

#[test]
fn split_bytecode__generated_parts_are_provable() {
    // Given
    let parts = UploadPart::split_bytecode(&bytecode(), PART_SIZE)
        .expect("Should be able to split bytecode");

    for part in parts.into_iter() {
        // When
        let proof_set = part
            .proof_set
            .iter()
            .map(|p| (*p).into())
            .collect::<Vec<_>>();
        let result = fuel_merkle::binary::verify(
            part.root.deref(),
            &part.part_bytecode,
            &proof_set,
            part.part_index as u64,
            part.parts_number as u64,
        );

        // Then
        assert!(result);
    }
}

#[test]
fn split_bytecode__generates_valid_transactions() {
    let parts = UploadPart::split_bytecode(&bytecode(), PART_SIZE)
        .expect("Should be able to split bytecode");

    for part in parts.into_iter() {
        // Given
        let tx = Transaction::upload_from_part(
            part,
            Policies::new().with_max_fee(0),
            vec![Input::coin_predicate(
                Default::default(),
                Input::predicate_owner(predicate()),
                Default::default(),
                AssetId::BASE,
                Default::default(),
                Default::default(),
                predicate(),
                vec![],
            )],
            vec![],
            vec![],
        );

        // When
        let result = tx.check(1000.into(), &test_params());

        // Then
        assert_eq!(Ok(()), result);
    }
}

#[test]
fn valid_upload_transaction_can_pass_check() {
    let block_height: BlockHeight = 1000.into();
    let tx = valid_upload_transaction()
        .finalize()
        .check(block_height, &test_params());
    assert_eq!(tx, Ok(()));
}

#[test]
fn maturity() {
    let block_height: BlockHeight = 1000.into();
    let failing_block_height = block_height.succ().unwrap();

    // Given
    let tx = valid_upload_transaction()
        .maturity(failing_block_height)
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionMaturity), result);
}

#[test]
fn check__not_set_witness_limit_success() {
    // Given
    let block_height = 1000.into();
    let tx = valid_upload_transaction().finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn check__set_witness_limit_for_empty_witness_success() {
    // Given
    let block_height = 1000.into();
    let limit = PART_SIZE + Signature::LEN + 2 * vec![0u8; 0].size_static();
    let tx = valid_upload_transaction()
        .witness_limit(limit as u64)
        .add_witness(vec![0; Signature::LEN].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Ok(()), result);
}

#[test]
fn script_set_witness_limit_less_than_witness_data_size_fails() {
    let block_height = 1000.into();
    let limit = PART_SIZE + Signature::LEN + 2 * vec![0u8; 0].size_static();

    // Given
    let failing_limit = limit - 1;
    let tx = valid_upload_transaction()
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
    let mut tx = valid_upload_transaction().add_random_fee_input().finalize();

    // Given
    tx.policies_mut().set(PolicyType::MaxFee, None);

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionMaxFeeNotSet), result);
}

#[test]
fn max_iow_reached_max_inputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_upload_transaction();

    while builder.outputs().len() < test_params().tx_params().max_outputs() as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), AssetId::BASE));
    }

    // Given
    let secrets: Vec<SecretKey> =
        (0..1 + test_params().tx_params().max_inputs() as usize - builder.inputs().len())
            .map(|_| SecretKey::random(rng))
            .collect();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            AssetId::BASE,
            rng.gen(),
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
fn max_iow_reached_max_outputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_upload_transaction();

    let secrets: Vec<SecretKey> = (0..test_params().tx_params().max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            AssetId::BASE,
            rng.gen(),
        );
    });
    while builder.witnesses().len() < test_params().tx_params().max_witnesses() as usize {
        builder.add_witness(generate_bytes(rng).into());
    }

    // Given
    while builder.outputs().len() < test_params().tx_params().max_outputs() as usize + 1 {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), AssetId::BASE));
    }
    let tx = builder.finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionOutputsMax), result);
}

#[test]
fn max_iow_reached_max_witnesses() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_upload_transaction();

    let secrets: Vec<SecretKey> = (0..test_params().tx_params().max_inputs() as usize
        - builder.inputs().len())
        .map(|_| SecretKey::random(rng))
        .collect();
    secrets.iter().for_each(|k| {
        builder.add_unsigned_coin_input(
            *k,
            rng.gen(),
            rng.gen(),
            AssetId::BASE,
            rng.gen(),
        );
    });
    while builder.outputs().len() < test_params().tx_params().max_outputs() as usize {
        builder.add_output(Output::coin(rng.gen(), rng.gen(), AssetId::BASE));
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
fn output_change_asset_id_duplicated_output() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let secret = SecretKey::random(rng);

    // Given
    let a: AssetId = rng.gen();
    let tx = valid_upload_transaction()
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), a, rng.gen())
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
        .add_output(Output::change(rng.gen(), rng.next_u64(), a))
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
fn output_change_asset_id_foreign_asset() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();

    // Given
    let c: AssetId = rng.gen();
    let tx = valid_upload_transaction()
        .add_output(Output::change(rng.gen(), rng.next_u64(), c))
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
    let tx = valid_upload_transaction()
        .add_input(Input::contract(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ))
        .add_output(Output::contract(1, rng.gen(), rng.gen()))
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
    let tx = valid_upload_transaction()
        .add_unsigned_coin_input(secret, rng.gen(), rng.gen(), rng.gen(), rng.gen())
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
    let tx = valid_upload_transaction()
        .add_unsigned_message_input(secret, rng.gen(), rng.gen(), rng.gen(), empty_data)
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
    let tx = valid_upload_transaction()
        .add_unsigned_message_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
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
    let tx = valid_upload_transaction()
        .add_output(Output::variable(rng.gen(), rng.gen(), rng.gen()))
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
    let tx = valid_upload_transaction()
        .add_output(Output::Contract(rng.gen()))
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
    let tx = valid_upload_transaction()
        .add_output(Output::contract_created(rng.gen(), rng.gen()))
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
    let tx = valid_upload_transaction()
        .add_output(Output::change(rng.gen(), rng.gen(), AssetId::BASE))
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
    let a: AssetId = rng.gen();
    let tx = valid_upload_transaction()
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.gen(),
            rng.gen(),
            a,
            rng.gen(),
        )
        .add_output(Output::change(rng.gen(), rng.gen(), a))
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
    let tx = valid_upload_transaction()
        .add_witness(vec![0; test_params().tx_params().max_size() as usize].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionSizeLimitExceeded), result);
}

#[test]
fn check__errors_when_parts_number_is_too_big() {
    let block_height = 1000.into();
    let tx = valid_upload_transaction().finalize();

    // Given
    let mut params = test_params();
    params.set_tx_params(TxParameters::default().with_max_bytecode_parts(0));

    // When
    let result = tx.check(block_height, &params);

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUploadTooManyBytecodeParts),
        result
    );
}

#[test]
fn check__errors_when_bytecode_witness_index_is_invalid() {
    let block_height = 1000.into();
    let mut tx = valid_upload_transaction().finalize();

    // Given
    *tx.bytecode_witness_index_mut() = u16::MAX;

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::InputWitnessIndexBounds {
            index: u16::MAX as usize
        }),
        result
    );
}

#[test]
fn check__errors_when_root_doesnt_match() {
    let block_height = 1000.into();
    let mut tx = valid_upload_transaction().finalize();

    // Given
    *tx.bytecode_root_mut() = [123; 32].into();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUploadRootVerificationFailed),
        result
    );
}

#[test]
fn check__errors_when_part_index_doesnt_match() {
    let block_height = 1000.into();
    let mut tx = valid_upload_transaction().finalize();

    // Given
    *tx.part_index_mut() = u16::MAX;

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUploadRootVerificationFailed),
        result
    );
}

// TODO: Remove `#[ignore]` when https://github.com/FuelLabs/fuel-vm/issues/716 is resolved
#[ignore]
#[test]
fn check__errors_when_parts_number_doesnt_match() {
    let block_height = 1000.into();
    let mut tx = valid_upload_transaction().finalize();

    // Given
    *tx.parts_number_mut() = *tx.parts_number() + 1;

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUploadRootVerificationFailed),
        result
    );
}

#[test]
fn check__errors_when_proof_set_doesnt_match() {
    let block_height = 1000.into();
    let mut tx = valid_upload_transaction().finalize();

    // Given
    tx.proof_set_mut().clear();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUploadRootVerificationFailed),
        result
    );
}

#[test]
fn check__errors_when_witness_doesnt_match() {
    let block_height = 1000.into();
    let mut tx = valid_upload_transaction().finalize();

    // Given
    tx.witnesses_mut()[0].as_vec_mut().push(0);

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUploadRootVerificationFailed),
        result
    );
}

mod inputs {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn coin_predicate_check_owner_works() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.gen()).collect_vec();
        let owner: Address = Input::predicate_owner(&predicate);

        // Given
        let tx = valid_upload_transaction()
            .add_input(Input::coin_predicate(
                rng.gen(),
                owner,
                rng.gen(),
                AssetId::BASE,
                rng.gen(),
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
    fn coin_predicate_check_owners_fails_incorrect_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.gen()).collect_vec();
        let incorrect_owner: Address = [1; 32].into();

        // Given
        let tx = valid_upload_transaction()
            .add_input(Input::coin_predicate(
                rng.gen(),
                incorrect_owner,
                rng.gen(),
                AssetId::BASE,
                rng.gen(),
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
    fn message_predicate_check_owners_works() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.gen()).collect_vec();
        let owner: Address = Input::predicate_owner(&predicate);

        // Given
        let tx = valid_upload_transaction()
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                owner,
                rng.gen(),
                rng.gen(),
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
    fn message_predicate_check_owners_fails_incorrect_owner() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.gen()).collect_vec();
        let incorrect_owner: Address = [1; 32].into();

        // Given
        let tx = valid_upload_transaction()
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                incorrect_owner,
                rng.gen(),
                rng.gen(),
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
