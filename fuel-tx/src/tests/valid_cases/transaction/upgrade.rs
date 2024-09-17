#![allow(clippy::cast_possible_truncation)]
#![allow(non_snake_case)]

use super::*;
use crate::field::UpgradePurpose as UpgradePurposeField;
use fuel_asm::op;
use fuel_crypto::Hasher;
use fuel_types::BlockHeight;

fn predicate() -> Vec<u8> {
    vec![op::ret(1)].into_iter().collect::<Vec<u8>>()
}

fn test_params() -> ConsensusParameters {
    let mut params = ConsensusParameters::default();
    params.set_privileged_address(Input::predicate_owner(predicate()));
    params
}

fn valid_upgrade_transaction() -> TransactionBuilder<Upgrade> {
    let mut builder = TransactionBuilder::upgrade(UpgradePurpose::StateTransition {
        root: Default::default(),
    });
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
    builder.with_params(test_params());

    builder
}

fn valid_upgrade_transaction_with_message() -> TransactionBuilder<Upgrade> {
    let mut builder = TransactionBuilder::upgrade(UpgradePurpose::StateTransition {
        root: Default::default(),
    });
    builder.max_fee_limit(0);
    builder.add_input(Input::message_coin_predicate(
        Default::default(),
        Input::predicate_owner(predicate()),
        Default::default(),
        Default::default(),
        Default::default(),
        predicate(),
        vec![],
    ));
    builder.with_params(test_params());

    builder
}

#[test]
fn valid_upgrade_transaction_can_pass_check() {
    let block_height: BlockHeight = 1000.into();
    let tx = valid_upgrade_transaction()
        .finalize()
        .check(block_height, &test_params());
    assert_eq!(tx, Ok(()));
}

#[test]
fn valid_upgrade_transaction_can_pass_check_with_message() {
    let block_height: BlockHeight = 1000.into();
    let tx = valid_upgrade_transaction_with_message()
        .finalize()
        .check(block_height, &test_params());
    assert_eq!(tx, Ok(()));
}

#[test]
fn maturity() {
    let block_height: BlockHeight = 1000.into();
    let failing_block_height = block_height.succ().unwrap();

    // Given
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction().finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn check__set_witness_limit_for_empty_witness_success() {
    // Given
    let block_height = 1000.into();
    let limit = Signature::LEN + vec![0u8; 0].size_static();
    let tx = valid_upgrade_transaction()
        .witness_limit(limit as u64)
        .add_witness(vec![0; Signature::LEN].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert!(result.is_ok());
}

#[test]
fn script_set_witness_limit_less_than_witness_data_size_fails() {
    let block_height = 1000.into();
    let limit = Signature::LEN /* witness from random fee */ + vec![0u8; 0].size_static();

    // Given
    let failing_limit = limit - 1;
    let tx = valid_upgrade_transaction()
        .witness_limit(failing_limit as u64)
        .add_fee_input()
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionWitnessLimitExceeded), result);
}

#[test]
fn check__no_max_fee_fails() {
    let block_height = 1000.into();
    let mut tx = valid_upgrade_transaction().add_fee_input().finalize();

    // Given
    tx.policies_mut().set(PolicyType::MaxFee, None);

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionMaxFeeNotSet), result);
}

#[test]
fn reached_max_inputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_upgrade_transaction();

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
fn reached_max_outputs() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_upgrade_transaction();

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
fn reached_max_witnesses() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let block_height = 1000.into();
    let mut builder = valid_upgrade_transaction();

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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
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
    let tx = valid_upgrade_transaction()
        .add_witness(vec![0; test_params().tx_params().max_size() as usize].into())
        .finalize_as_transaction();

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionSizeLimitExceeded), result);
}

#[test]
fn check__errors_when_owner_is_not_privileged_address() {
    let block_height = 1000.into();
    let tx = valid_upgrade_transaction().finalize_as_transaction();

    // Given
    let mut actual_params = test_params();
    actual_params.set_privileged_address([0; 32].into());

    // When
    let result = tx.check(block_height, &actual_params);

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUpgradeNoPrivilegedAddress),
        result
    );
}

#[test]
fn check__errors_when_consensus_parameters_invalid_witness_index() {
    let block_height = 1000.into();
    let mut tx = valid_upgrade_transaction().finalize();

    // Given
    *tx.upgrade_purpose_mut() = UpgradePurpose::ConsensusParameters {
        witness_index: u16::MAX,
        checksum: Default::default(),
    };

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
fn check__errors_when_consensus_parameters_invalid_checksum() {
    let block_height = 1000.into();

    // Given
    let mut tx = valid_upgrade_transaction()
        .add_witness(vec![123; 1024].into())
        .finalize();
    *tx.upgrade_purpose_mut() = UpgradePurpose::ConsensusParameters {
        witness_index: 0,
        checksum: Default::default(),
    };

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUpgradeConsensusParametersChecksumMismatch),
        result
    );
}

#[test]
fn check__errors_when_consensus_parameters_unable_decode_consensus_parameters() {
    let block_height = 1000.into();
    let serialized_consensus_parameters = vec![123; 1024];

    // Given
    let mut tx = valid_upgrade_transaction()
        .add_witness(serialized_consensus_parameters.clone().into())
        .finalize();
    *tx.upgrade_purpose_mut() = UpgradePurpose::ConsensusParameters {
        witness_index: 0,
        checksum: Hasher::hash(serialized_consensus_parameters.as_slice()),
    };

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(
        Err(ValidityError::TransactionUpgradeConsensusParametersDeserialization),
        result
    );
}

#[test]
fn check__errors_when_consensus_parameters_different_than_calculated_metadata() {
    let block_height = 1000.into();
    let serialized_consensus_parameters = postcard::to_allocvec(&test_params()).unwrap();

    // Given
    // `valid_upgrade_transaction` already returns a transaction with calculated metadata.
    // Setting a new `UpgradePurpose` below will cause mismatch between the calculated
    // metadata and the actual metadata.
    let mut tx = valid_upgrade_transaction()
        .add_witness(serialized_consensus_parameters.clone().into())
        .finalize();
    *tx.upgrade_purpose_mut() = UpgradePurpose::ConsensusParameters {
        witness_index: 0,
        checksum: Hasher::hash(serialized_consensus_parameters.as_slice()),
    };

    // When
    let result = tx.check(block_height, &test_params());

    // Then
    assert_eq!(Err(ValidityError::TransactionMetadataMismatch), result);
}

// The module tests that `Upgrade` transaction can work with different input types.
mod check_inputs {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn coin_predicate_check_owner_works() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let block_height = 1000.into();
        let predicate = (0..100).map(|_| rng.gen()).collect_vec();
        let owner: Address = Input::predicate_owner(&predicate);

        // Given
        let tx = valid_upgrade_transaction()
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
        let tx = valid_upgrade_transaction()
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
        let tx = valid_upgrade_transaction()
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
        let tx = valid_upgrade_transaction()
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
