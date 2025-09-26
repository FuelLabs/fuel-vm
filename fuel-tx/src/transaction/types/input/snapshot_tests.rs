//! snapshot tests to ensure the serialized format of inputs doesn't change

use super::*;
use crate::{
    Transaction,
    TransactionBuilder,
};
use fuel_types::canonical::Serialize;

fn tx_with_signed_coin_snapshot() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
        .add_input(Input::CoinSigned(CoinSigned {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            owner: [2u8; 32].into(),
            amount: 11,
            asset_id: [5u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), 5),
            witness_index: 4,
            predicate_gas_used: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .witness_limit(1000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_signed_coin_snapshot_canonical() {
    let tx = tx_with_signed_coin_snapshot();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_coin_snapshot_json() {
    let tx = tx_with_signed_coin_snapshot();

    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_signed_coin_snapshot_postcard() {
    let tx = tx_with_signed_coin_snapshot();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

fn tx_with_predicate_coin_snapshot() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
        .add_input(Input::CoinPredicate(CoinPredicate {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            owner: [2u8; 32].into(),
            amount: 11,
            asset_id: [5u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), 5),
            witness_index: Empty::new(),
            predicate_gas_used: 100_000,
            predicate: vec![3u8; 10].into(),
            predicate_data: vec![4u8; 12].into(),
        }))
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_predicate_coin_snapshot_canonical() {
    let tx = tx_with_predicate_coin_snapshot();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_coin_snapshot_json() {
    let tx = tx_with_predicate_coin_snapshot();

    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_predicate_coin_snapshot_postcard() {
    let tx = tx_with_predicate_coin_snapshot();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

fn tx_with_contract_snapshot() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
        .add_input(Input::Contract(Contract {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            balance_root: [2u8; 32].into(),
            state_root: [3u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), 5),
            contract_id: [5u8; 32].into(),
        }))
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_contract_snapshot_canonical() {
    let tx = tx_with_contract_snapshot();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_contract_snapshot_json() {
    let tx = tx_with_contract_snapshot();

    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_contract_snapshot_postcard() {
    let tx = tx_with_contract_snapshot();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

fn tx_with_signed_message_coin() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
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
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .witness_limit(1000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_signed_message_coin_canonical() {
    let tx = tx_with_signed_message_coin();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_message_coin_json() {
    let tx = tx_with_signed_message_coin();

    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_signed_message_coin_postcard() {
    let tx = tx_with_signed_message_coin();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

fn tx_with_predicate_message_coin() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageCoinPredicate(MessageCoinPredicate {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: Empty::new(),
            predicate_gas_used: 100_000,
            data: Empty::new(),
            predicate: vec![7u8; 11].into(),
            predicate_data: vec![8u8; 12].into(),
        }))
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_predicate_message_coin_canonical() {
    let tx = tx_with_predicate_message_coin();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_message_coin_json() {
    let tx = tx_with_predicate_message_coin();
    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_predicate_message_coin_postcard() {
    let tx = tx_with_predicate_message_coin();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

fn tx_with_signed_message_data() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageDataSigned(MessageDataSigned {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: 6,
            predicate_gas_used: Empty::new(),
            data: vec![7u8; 10].into(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .witness_limit(1000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_signed_message_data_canonical() {
    let tx = tx_with_signed_message_data();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_message_data_json() {
    let tx = tx_with_signed_message_data();

    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_signed_message_data_postcard() {
    let tx = tx_with_signed_message_data();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

fn tx_with_predicate_message_data() -> Transaction {
    TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageDataPredicate(MessageDataPredicate {
            sender: [2u8; 32].into(),
            recipient: [3u8; 32].into(),
            amount: 4,
            nonce: [5u8; 32].into(),
            witness_index: Empty::new(),
            predicate_gas_used: 100_000,
            data: vec![6u8; 10].into(),
            predicate: vec![7u8; 11].into(),
            predicate_data: vec![8u8; 12].into(),
        }))
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .owner(0)
        .max_fee_limit(1000000)
        .finalize_as_transaction()
}

#[test]
fn tx_with_predicate_message_data_canonical() {
    let tx = tx_with_predicate_message_data();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_message_data_json() {
    let tx = tx_with_predicate_message_data();

    // Json
    let json = serde_json::to_string_pretty(&tx).unwrap();
    insta::assert_snapshot!(json);
}

#[test]
fn tx_with_predicate_message_data_postcard() {
    let tx = tx_with_predicate_message_data();

    let bytes = postcard::to_allocvec(&tx).unwrap();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}
