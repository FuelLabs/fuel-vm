//! snapshot tests to ensure the serialized format of inputs doesn't change

use super::*;
use crate::TransactionBuilder;
use fuel_types::canonical::Serialize;

#[test]
fn tx_with_signed_coin_snapshot() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::CoinSigned(CoinSigned {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            owner: [2u8; 32].into(),
            amount: 11,
            asset_id: [5u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), 5),
            witness_index: 4,
            maturity: 2.into(),
            predicate_gas_used: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_coin_snapshot() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::CoinPredicate(CoinPredicate {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            owner: [2u8; 32].into(),
            amount: 11,
            asset_id: [5u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), 5),
            witness_index: Empty::new(),
            maturity: 2.into(),
            predicate_gas_used: 100_000,
            predicate: vec![3u8; 10],
            predicate_data: vec![4u8; 12],
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_contract_snapshot() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::Contract(Contract {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            balance_root: [2u8; 32].into(),
            state_root: [3u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), 5),
            contract_id: [5u8; 32].into(),
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_message_coin() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageCoinSigned(MessageCoinSigned {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: 6,
            predicate_gas_used: Empty::new(),
            data: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_message_coin() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageCoinPredicate(MessageCoinPredicate {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: Empty::new(),
            predicate_gas_used: 100_000,
            data: Empty::new(),
            predicate: vec![7u8; 11],
            predicate_data: vec![8u8; 12],
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_message_data() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageDataSigned(MessageDataSigned {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: 6,
            predicate_gas_used: Empty::new(),
            data: vec![7u8; 10],
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_message_data() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageDataPredicate(MessageDataPredicate {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: Empty::new(),
            predicate_gas_used: 100_000,
            data: vec![6u8; 10],
            predicate: vec![7u8; 11],
            predicate_data: vec![8u8; 12],
        }))
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(&bytes);
    insta::assert_snapshot!(hex);
}
