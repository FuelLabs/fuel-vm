use fuel_types::bytes::WORD_SIZE;
use fuel_types::{Bytes32, Salt};

pub const TRANSACTION_SCRIPT_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Byte price
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
    + WORD_SIZE // Byte price
    + WORD_SIZE // Maturity
    + WORD_SIZE // Bytecode size
    + WORD_SIZE // Bytecode witness index
    + WORD_SIZE // Storage slots size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE // Witnesses size
    + Salt::LEN; // Salt
