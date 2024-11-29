#![allow(non_snake_case)]
use crate::{
    checked_transaction::IntoChecked,
    interpreter::Interpreter,
    storage::UploadedBytecode,
};
use fuel_asm::{
    op,
    PanicReason,
};
use fuel_tx::{
    field::Outputs,
    policies::Policies,
    GasCosts,
    Input,
    Output,
    Transaction,
    Upload,
    UploadSubsection,
    ValidityError,
};
use fuel_types::AssetId;

use crate::{
    checked_transaction::Ready,
    storage::UploadedBytecodes,
};
use fuel_storage::StorageAsRef;

use crate::{
    checked_transaction::CheckError,
    error::InterpreterError,
};
#[cfg(feature = "alloc")]
use alloc::{
    vec,
    vec::Vec,
};

const AMOUNT: u64 = 1000;
const BYTECODE_SIZE: usize = 1024;

fn bytecode() -> Vec<u8> {
    vec![123; BYTECODE_SIZE]
}

fn valid_input() -> Input {
    let predicate = vec![op::ret(1)].into_iter().collect::<Vec<u8>>();
    let owner = Input::predicate_owner(&predicate);
    Input::coin_predicate(
        Default::default(),
        owner,
        AMOUNT,
        AssetId::BASE,
        Default::default(),
        Default::default(),
        predicate,
        vec![],
    )
}

fn valid_transaction_from_subsection(subsection: UploadSubsection) -> Ready<Upload> {
    Transaction::upload_from_subsection(
        subsection,
        Policies::new().with_max_fee(AMOUNT),
        vec![valid_input()],
        vec![Output::change(Default::default(), 0, AssetId::BASE)],
        vec![],
    )
    .into_checked_basic(Default::default(), &Default::default())
    .expect("Failed to generate checked tx")
    .test_into_ready()
}

#[test]
fn transact__uploads_bytecode_with_one_subsection() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections =
        UploadSubsection::split_bytecode(&bytecode(), BYTECODE_SIZE).unwrap();
    let root = subsections[0].root;
    assert_eq!(subsections.len(), 1);

    // Given
    let tx = valid_transaction_from_subsection(subsections[0].clone());
    assert!(!client
        .as_ref()
        .storage_as_ref::<UploadedBytecodes>()
        .contains_key(&root)
        .unwrap());

    // When
    let _ = client.transact(tx).expect("Failed to transact");

    // Then
    assert_eq!(
        client
            .as_ref()
            .storage_as_ref::<UploadedBytecodes>()
            .get(&root)
            .unwrap()
            .unwrap()
            .into_owned(),
        UploadedBytecode::Completed(bytecode())
    );
}

#[test]
fn transact__uploads_bytecode_with_several_subsections() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();

    // Given
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();
    let root = subsections[0].root;
    assert!(subsections.len() > 1);

    // When
    for subsection in subsections {
        let tx = valid_transaction_from_subsection(subsection);
        let _ = client.transact(tx).expect("Failed to transact");
    }

    // Then
    assert_eq!(
        client
            .as_ref()
            .storage_as_ref::<UploadedBytecodes>()
            .get(&root)
            .unwrap()
            .unwrap()
            .into_owned(),
        UploadedBytecode::Completed(bytecode())
    );
}

#[test]
fn transact__uploads_bytecode_with_half_of_subsections() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();

    // Given
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();
    let root = subsections[0].root;
    let len = subsections.len();
    assert!(len > 3);

    // When
    for subsection in subsections.into_iter().take(len / 2) {
        let tx = valid_transaction_from_subsection(subsection);
        let _ = client.transact(tx).expect("Failed to transact");
    }

    // Then
    assert!(matches!(
        client
            .as_ref()
            .storage_as_ref::<UploadedBytecodes>()
            .get(&root)
            .unwrap()
            .unwrap()
            .into_owned(),
        UploadedBytecode::Uncompleted { .. }
    ));
}

#[test]
fn transact__fails_for_completed_bytecode() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections =
        UploadSubsection::split_bytecode(&bytecode(), BYTECODE_SIZE).unwrap();
    assert_eq!(subsections.len(), 1);

    // Given
    let tx = valid_transaction_from_subsection(subsections[0].clone());
    let _ = client.transact(tx.clone()).expect("Failed to transact");

    // When
    let result = client.transact(tx);

    // Then
    assert_eq!(
        result,
        Err(InterpreterError::Panic(
            PanicReason::BytecodeAlreadyUploaded
        ))
    );
}

#[test]
fn transact__fails_when_the_ordering_of_uploading_is_wrong__missed_first_subsection() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();
    assert!(subsections.len() > 1);

    // Given
    let second_subsection = valid_transaction_from_subsection(subsections[1].clone());

    // When
    let result = client.transact(second_subsection);

    // Then
    assert_eq!(
        result,
        Err(InterpreterError::Panic(
            PanicReason::ThePartIsNotSequentiallyConnected
        ))
    );
}

#[test]
fn transact__fails_when_the_ordering_of_uploading_is_wrong__skipped_second_subsection() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();
    assert!(subsections.len() >= 3);
    let first_subsection = valid_transaction_from_subsection(subsections[0].clone());
    let _ = client
        .transact(first_subsection)
        .expect("Should add first subsection");

    // Given
    let third_subsection = valid_transaction_from_subsection(subsections[2].clone());

    // When
    let result = client.transact(third_subsection);

    // Then
    assert_eq!(
        result,
        Err(InterpreterError::Panic(
            PanicReason::ThePartIsNotSequentiallyConnected
        ))
    );
}

#[test]
fn transact__fails_when_the_ordering_of_uploading_is_wrong__second_subsection_sent_twice()
{
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();
    assert!(subsections.len() >= 3);
    let first_subsection = valid_transaction_from_subsection(subsections[0].clone());
    let _ = client
        .transact(first_subsection)
        .expect("Should add first subsection");

    // Given
    let second_subsection = valid_transaction_from_subsection(subsections[1].clone());
    let _ = client
        .transact(second_subsection.clone())
        .expect("Should add second subsection");

    // When
    let result = client.transact(second_subsection);

    // Then
    assert_eq!(
        result,
        Err(InterpreterError::Panic(
            PanicReason::ThePartIsNotSequentiallyConnected
        ))
    );
}

#[test]
fn check__fails_when_subsection_index_more_than_total_number() {
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();
    assert!(subsections.len() >= 3);

    // Given
    let mut subsection = subsections[0].clone();
    subsection.subsection_index = subsection.subsections_number;

    // When
    let result = Transaction::upload_from_subsection(
        subsection,
        Policies::new().with_max_fee(AMOUNT),
        vec![valid_input()],
        vec![],
        vec![],
    )
    .into_checked_basic(Default::default(), &Default::default());

    // Then
    assert_eq!(
        result,
        Err(CheckError::Validity(
            ValidityError::TransactionUploadRootVerificationFailed
        ))
    );
}

#[test]
fn check__fails_when_total_number_is_zero() {
    let subsections = UploadSubsection::split_bytecode(&bytecode(), 123).unwrap();

    // Given
    let mut subsection = subsections[0].clone();
    subsection.subsections_number = 0;

    // When
    let result = Transaction::upload_from_subsection(
        subsection,
        Policies::new().with_max_fee(AMOUNT),
        vec![valid_input()],
        vec![],
        vec![],
    )
    .into_checked_basic(Default::default(), &Default::default());

    // Then
    assert_eq!(
        result,
        Err(CheckError::Validity(
            ValidityError::TransactionUploadRootVerificationFailed
        ))
    );
}

#[test]
fn transact__with_zero_gas_price_doesnt_affect_change_output() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections =
        UploadSubsection::split_bytecode(&bytecode(), BYTECODE_SIZE).unwrap();

    // Given
    let gas_price = 0;
    client.set_gas_price(gas_price);
    let tx = valid_transaction_from_subsection(subsections[0].clone());

    // When
    let state = client.transact(tx).expect("failed to transact");

    // Then
    let Output::Change {
        amount, asset_id, ..
    } = state.tx().outputs()[0]
    else {
        panic!("expected change output");
    };
    assert_eq!(amount, AMOUNT);
    assert_eq!(asset_id, AssetId::BASE);
}

#[test]
fn transact__with_non_zero_gas_price_affects_change_output() {
    let mut client = Interpreter::<_, _, Upload>::with_memory_storage();
    let subsections =
        UploadSubsection::split_bytecode(&bytecode(), BYTECODE_SIZE).unwrap();

    // Given
    let gas_price = 1;
    client.set_gas_price(gas_price);
    let tx = Transaction::upload_from_subsection(
        subsections[0].clone(),
        Policies::new().with_max_fee(AMOUNT),
        vec![valid_input()],
        vec![Output::change(Default::default(), 0, AssetId::BASE)],
        vec![],
    )
    .into_checked_basic(Default::default(), &Default::default())
    .expect("Failed to generate checked tx")
    .into_ready(gas_price, &GasCosts::default(), &Default::default(), None)
    .expect("Failed to generate ready tx");

    // When
    let state = client.transact(tx).expect("failed to transact");

    // Then
    let Output::Change {
        amount, asset_id, ..
    } = state.tx().outputs()[0]
    else {
        panic!("expected change output");
    };
    assert_eq!(amount, AMOUNT - 1);
    assert_eq!(asset_id, AssetId::BASE);
}
