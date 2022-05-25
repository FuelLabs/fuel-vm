/// Consensus configurable parameters used for verifying transactions

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsensusParameters {
    /// Maximum contract size, in bytes.
    pub contract_max_size: u64,
    /// Maximum number of inputs.
    pub max_inputs: u64,
    /// Maximum number of outputs.
    pub max_outputs: u64,
    /// Maximum number of witnesses.
    pub max_witnesses: u64,
    /// Maximum gas per transaction.
    pub max_gas_per_tx: u64,
    /// Maximum length of script, in instructions.
    pub max_script_length: u64,
    /// Maximum length of script data, in bytes.
    pub max_script_data_length: u64,
    /// Maximum number of static contracts.
    pub max_static_contracts: u64,
    /// Maximum number of initial storage slots.
    pub max_storage_slots: u64,
    /// Maximum length of predicate, in instructions.
    pub max_predicate_length: u64,
    /// Maximum length of predicate data, in bytes.
    pub max_predicate_data_length: u64,
}

impl Default for ConsensusParameters {
    fn default() -> Self {
        use default_parameters::*;
        Self {
            contract_max_size: CONTRACT_MAX_SIZE,
            max_inputs: MAX_INPUTS,
            max_outputs: MAX_OUTPUTS,
            max_witnesses: MAX_WITNESSES,
            max_gas_per_tx: MAX_GAS_PER_TX,
            max_script_length: MAX_SCRIPT_LENGTH,
            max_script_data_length: MAX_SCRIPT_DATA_LENGTH,
            max_static_contracts: MAX_STATIC_CONTRACTS,
            max_storage_slots: MAX_STORAGE_SLOTS,
            max_predicate_length: MAX_PREDICATE_LENGTH,
            max_predicate_data_length: MAX_PREDICATE_DATA_LENGTH,
        }
    }
}

/// Arbitrary default consensus parameters. While best-efforts are made to adjust these to
/// reasonable settings, they may not be useful for every network instantiation.
pub mod default_parameters {
    pub const CONTRACT_MAX_SIZE: u64 = 16 * 1024 * 1024;
    pub const MAX_INPUTS: u64 = 255;
    pub const MAX_OUTPUTS: u64 = 255;
    pub const MAX_WITNESSES: u64 = 255;
    pub const MAX_GAS_PER_TX: u64 = 100_000_000;
    pub const MAX_SCRIPT_LENGTH: u64 = 1024 * 1024;
    pub const MAX_SCRIPT_DATA_LENGTH: u64 = 1024 * 1024;
    pub const MAX_STATIC_CONTRACTS: u64 = 255;
    pub const MAX_STORAGE_SLOTS: u64 = 255;
    pub const MAX_PREDICATE_LENGTH: u64 = 1024 * 1024;
    pub const MAX_PREDICATE_DATA_LENGTH: u64 = 1024 * 1024;
}
