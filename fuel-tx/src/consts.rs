/// Maximum contract size, in bytes.
pub const CONTRACT_MAX_SIZE: u64 = 16 * 1024 * 1024;

/// Maximum number of inputs.
pub const MAX_INPUTS: u8 = 8;

/// Maximum number of outputs.
pub const MAX_OUTPUTS: u8 = 8;

/// Maximum number of witnesses.
pub const MAX_WITNESSES: u8 = 16;

/// Maximum gas per transaction.
pub const MAX_GAS_PER_TX: u64 = 1000000;

// TODO set max script length const
/// Maximum length of script, in instructions.
pub const MAX_SCRIPT_LENGTH: u64 = 1024 * 1024;

// TODO set max script length const
/// Maximum length of script data, in bytes.
pub const MAX_SCRIPT_DATA_LENGTH: u64 = 1024 * 1024;

/// Maximum number of static contracts.
pub const MAX_STATIC_CONTRACTS: u64 = 255;

/// Maximum number of initial storage slots.
pub const MAX_STORAGE_SLOTS: u16 = 255;

// TODO set max predicate length value
/// Maximum length of predicate, in instructions.
pub const MAX_PREDICATE_LENGTH: u64 = 1024 * 1024;

// TODO set max predicate data length value
/// Maximum length of predicate data, in bytes.
pub const MAX_PREDICATE_DATA_LENGTH: u64 = 1024 * 1024;
