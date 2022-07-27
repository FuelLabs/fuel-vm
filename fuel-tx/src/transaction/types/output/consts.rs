use fuel_types::bytes::WORD_SIZE;
use fuel_types::{Address, AssetId, Bytes32, ContractId};

pub(super) const OUTPUT_CCV_TO_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const OUTPUT_CCV_ASSET_ID_OFFSET: usize = OUTPUT_CCV_TO_OFFSET
    + Address::LEN // To
    + WORD_SIZE; // Amount
pub(super) const OUTPUT_CCV_SIZE: usize = OUTPUT_CCV_ASSET_ID_OFFSET + AssetId::LEN; // AssetId

pub(super) const OUTPUT_MESSAGE_RECIPIENT_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const OUTPUT_MESSAGE_SIZE: usize = OUTPUT_MESSAGE_RECIPIENT_OFFSET
    + Address::LEN // Recipient
    + WORD_SIZE; // Amount

pub(super) const OUTPUT_CONTRACT_BALANCE_ROOT_OFFSET: usize = WORD_SIZE // Identifier
    + WORD_SIZE; // Input index
pub(super) const OUTPUT_CONTRACT_STATE_ROOT_OFFSET: usize =
    OUTPUT_CONTRACT_BALANCE_ROOT_OFFSET + Bytes32::LEN; // Balance root
pub(super) const OUTPUT_CONTRACT_SIZE: usize = OUTPUT_CONTRACT_STATE_ROOT_OFFSET + Bytes32::LEN; // State root

pub(super) const OUTPUT_CONTRACT_CREATED_ID_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const OUTPUT_CONTRACT_CREATED_STATE_ROOT_OFFSET: usize =
    OUTPUT_CONTRACT_CREATED_ID_OFFSET + ContractId::LEN; // Contract Id
pub(super) const OUTPUT_CONTRACT_CREATED_SIZE: usize =
    OUTPUT_CONTRACT_CREATED_STATE_ROOT_OFFSET + Bytes32::LEN; // State Root
