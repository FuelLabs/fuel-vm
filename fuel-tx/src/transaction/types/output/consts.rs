use fuel_types::{
    bytes::WORD_SIZE,
    Address,
    Bytes32,
    ContractId,
};

pub(super) const OUTPUT_CCV_TO_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const OUTPUT_CCV_ASSET_ID_OFFSET: usize = OUTPUT_CCV_TO_OFFSET
    + Address::LEN // To
    + WORD_SIZE; // Amount

pub(super) const OUTPUT_CONTRACT_BALANCE_ROOT_OFFSET: usize = WORD_SIZE // Identifier
    + WORD_SIZE; // Input index
pub(super) const OUTPUT_CONTRACT_STATE_ROOT_OFFSET: usize =
    OUTPUT_CONTRACT_BALANCE_ROOT_OFFSET + Bytes32::LEN; // Balance root

pub(super) const OUTPUT_CONTRACT_CREATED_ID_OFFSET: usize = WORD_SIZE; // Identifier
pub(super) const OUTPUT_CONTRACT_CREATED_STATE_ROOT_OFFSET: usize =
    OUTPUT_CONTRACT_CREATED_ID_OFFSET + ContractId::LEN; // Contract Id
