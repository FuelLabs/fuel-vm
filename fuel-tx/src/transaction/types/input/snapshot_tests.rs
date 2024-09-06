//! snapshot tests to ensure the serialized format of inputs doesn't change

use super::*;
use crate::TransactionBuilder;
use fuel_types::canonical::Serialize;

#[test]
fn tx_with_signed_coin_snapshot() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::CoinSigned(CoinSigned {
            common: CoinCommon {
                utxo_id: UtxoId::new([1u8; 32].into(), 2),
                owner: [2u8; 32].into(),
                amount: 11,
                asset_id: [5u8; 32].into(),
                tx_pointer: TxPointer::new(46.into(), 5),
            },
            witness_index: 4,
        }))
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .witness_limit(1000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_coin_snapshot() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::CoinPredicate(CoinPredicate {
            common: CoinCommon {
                utxo_id: UtxoId::new([1u8; 32].into(), 2),
                owner: [2u8; 32].into(),
                amount: 11,
                asset_id: [5u8; 32].into(),
                tx_pointer: TxPointer::new(46.into(), 5),
            },
            predicate: Predicate {
                gas_used: 100_000,
                code: vec![3u8; 10],
                data: vec![4u8; 12],
            },
        }))
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
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
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_message_coin() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageCoinSigned(MessageCoinSigned {
            common: MessageCommon {
                sender: [2u8; 32].into(),
                recipient: [3u8; 32].into(),
                amount: 4,
                nonce: [5u8; 32].into(),
            },
            witness_index: 6,
        }))
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .witness_limit(1000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_message_coin() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageCoinPredicate(MessageCoinPredicate {
            common: MessageCommon {
                sender: [2u8; 32].into(),
                recipient: [3u8; 32].into(),
                amount: 4,
                nonce: [5u8; 32].into(),
            },
            predicate: Predicate {
                gas_used: 100_000,
                code: vec![7u8; 11],
                data: vec![8u8; 12],
            },
        }))
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_signed_message_data() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageDataSigned(MessageDataSigned {
            common: MessageCommon {
                sender: [2u8; 32].into(),
                recipient: [3u8; 32].into(),
                amount: 4,
                nonce: [5u8; 32].into(),
            },
            witness_index: 6,
            data: vec![7u8; 10],
        }))
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .witness_limit(1000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}

#[test]
fn tx_with_predicate_message_data() {
    let tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::MessageDataPredicate(MessageDataPredicate {
            common: MessageCommon {
                sender: [2u8; 32].into(),
                recipient: [3u8; 32].into(),
                amount: 4,
                nonce: [5u8; 32].into(),
            },
            data: vec![6u8; 10],
            predicate: Predicate {
                gas_used: 100_000,
                code: vec![7u8; 11],
                data: vec![8u8; 12],
            },
        }))
        .tip(1)
        .maturity(123.into())
        .max_fee_limit(1000000)
        .finalize_as_transaction();

    let bytes = tx.to_bytes();
    let hex = hex::encode(bytes);
    insta::assert_snapshot!(hex);
}
