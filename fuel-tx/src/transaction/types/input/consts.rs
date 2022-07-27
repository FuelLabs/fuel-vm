use crate::UtxoId;

use fuel_types::bytes::WORD_SIZE;
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId};

pub(super) const INPUT_UTXO_ID_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const INPUT_COIN_OWNER_OFFSET: usize = INPUT_UTXO_ID_OFFSET + UtxoId::LEN;
pub(super) const INPUT_COIN_ASSET_ID_OFFSET: usize = INPUT_COIN_OWNER_OFFSET
    + Address::LEN // Owner
    + WORD_SIZE; // Amount
pub(super) const INPUT_COIN_FIXED_SIZE: usize = INPUT_COIN_ASSET_ID_OFFSET
    + AssetId::LEN // AssetId
    + WORD_SIZE // Witness index
    + WORD_SIZE // Maturity
    + WORD_SIZE // Predicate size
    + WORD_SIZE; // Predicate data size

pub(super) const INPUT_CONTRACT_BALANCE_ROOT_OFFSET: usize = INPUT_UTXO_ID_OFFSET + UtxoId::LEN; // UtxoId
pub(super) const INPUT_CONTRACT_STATE_ROOT_OFFSET: usize =
    INPUT_CONTRACT_BALANCE_ROOT_OFFSET + Bytes32::LEN; // Balance root
pub(super) const INPUT_CONTRACT_ID_OFFSET: usize = INPUT_CONTRACT_STATE_ROOT_OFFSET + Bytes32::LEN; // State root
pub(super) const INPUT_CONTRACT_SIZE: usize = INPUT_CONTRACT_ID_OFFSET + ContractId::LEN; // Contract address

pub(super) const INPUT_MESSAGE_ID_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const INPUT_MESSAGE_SENDER_OFFSET: usize = INPUT_MESSAGE_ID_OFFSET + MessageId::LEN; // message_id
pub(super) const INPUT_MESSAGE_RECIPIENT_OFFSET: usize = INPUT_MESSAGE_SENDER_OFFSET + Address::LEN; // sender
pub(super) const INPUT_MESSAGE_OWNER_OFFSET: usize = INPUT_MESSAGE_RECIPIENT_OFFSET
    + Address::LEN // recipient
    + WORD_SIZE //amount
    + WORD_SIZE; // nonce

pub(super) const INPUT_MESSAGE_FIXED_SIZE: usize = INPUT_MESSAGE_OWNER_OFFSET
    + Address::LEN // owner
    + WORD_SIZE // witness_index
    + WORD_SIZE // Data size
    + WORD_SIZE // Predicate size
    + WORD_SIZE; // Predicate data size
