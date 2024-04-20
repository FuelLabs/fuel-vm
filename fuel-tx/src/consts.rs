use crate::TxPointer;
use fuel_types::{
    bytes::WORD_SIZE,
    AssetId,
    Bytes32,
    Salt,
};

/// Size of balance entry, i.e. asset id and associated balance.
pub const BALANCE_ENTRY_SIZE: usize = AssetId::LEN + WORD_SIZE;

pub const TRANSACTION_SCRIPT_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Maturity
    + WORD_SIZE // Script size
    + WORD_SIZE // Script data size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE // Witnesses size
    + Bytes32::LEN; // Receipts root

pub const TRANSACTION_CREATE_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Maturity
    + WORD_SIZE // Bytecode size
    + WORD_SIZE // Bytecode witness index
    + WORD_SIZE // Storage slots size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE // Witnesses size
    + Salt::LEN; // Salt

pub const TRANSACTION_MINT_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + TxPointer::LEN // Tx pointer
    + WORD_SIZE; // Outputs size
