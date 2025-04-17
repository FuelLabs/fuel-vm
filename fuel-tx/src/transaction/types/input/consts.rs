use crate::{
    TxPointer,
    UtxoId,
};

use fuel_types::{
    bytes::WORD_SIZE,
    Address,
    AssetId,
    Bytes32,
    Nonce,
};

pub(super) const INPUT_UTXO_ID_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const INPUT_COIN_OWNER_OFFSET: usize = INPUT_UTXO_ID_OFFSET + UtxoId::LEN;

pub(super) const INPUT_COIN_ASSET_ID_OFFSET: usize = INPUT_COIN_OWNER_OFFSET
    + Address::LEN // Owner
    + WORD_SIZE; // Amount
pub(super) const INPUT_COIN_TX_POINTER_OFFSET: usize =
    INPUT_COIN_ASSET_ID_OFFSET + AssetId::LEN; // AssetId
pub(super) const INPUT_COIN_FIXED_SIZE: usize = INPUT_COIN_TX_POINTER_OFFSET
    + TxPointer::LEN // TxPointer
    + WORD_SIZE // Witness index
    + WORD_SIZE // Predicate size
    + WORD_SIZE // Predicate data size
    + WORD_SIZE; // Predicate gas used

pub(super) const INPUT_DATA_COIN_FIXED_SIZE: usize = INPUT_COIN_TX_POINTER_OFFSET
    + TxPointer::LEN // TxPointer
    + WORD_SIZE // Witness index
    + WORD_SIZE // Predicate size
    + WORD_SIZE // Predicate data size
    + WORD_SIZE // Predicate gas used
    + WORD_SIZE; // TODO: Figure out why is this the right size.

pub(super) const INPUT_CONTRACT_BALANCE_ROOT_OFFSET: usize =
    INPUT_UTXO_ID_OFFSET + UtxoId::LEN; // UtxoId
pub(super) const INPUT_CONTRACT_STATE_ROOT_OFFSET: usize =
    INPUT_CONTRACT_BALANCE_ROOT_OFFSET + Bytes32::LEN; // Balance root
pub(super) const INPUT_CONTRACT_TX_POINTER_OFFSET: usize =
    INPUT_CONTRACT_STATE_ROOT_OFFSET + Bytes32::LEN; // State root
pub(super) const INPUT_CONTRACT_ID_OFFSET: usize =
    INPUT_CONTRACT_TX_POINTER_OFFSET + TxPointer::LEN; // TxPointer

pub(super) const INPUT_MESSAGE_SENDER_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const INPUT_MESSAGE_RECIPIENT_OFFSET: usize =
    INPUT_MESSAGE_SENDER_OFFSET + Address::LEN; // sender
pub(super) const INPUT_NONCE_RECIPIENT_OFFSET: usize = INPUT_MESSAGE_RECIPIENT_OFFSET
        + Address::LEN //amount
        + WORD_SIZE; // recipient

pub(super) const INPUT_MESSAGE_FIXED_SIZE: usize = INPUT_NONCE_RECIPIENT_OFFSET
    + Nonce::LEN // nonce
    + WORD_SIZE // witness_index
    + WORD_SIZE // Data size
    + WORD_SIZE // Predicate size
    + WORD_SIZE // Predicate data size
    + WORD_SIZE; // Predicate gas used
