use fuel_types::bytes::WORD_SIZE;
use fuel_types::{AssetId, Bytes32};

const MAX_GAS: u64 = 100_000_000;

/// Consensus configurable parameters used for verifying transactions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
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
    /// Maximum number of initial storage slots.
    pub max_storage_slots: u64,
    /// Maximum length of predicate, in instructions.
    pub max_predicate_length: u64,
    /// Maximum length of predicate data, in bytes.
    pub max_predicate_data_length: u64,
    /// Maximum gas per predicate
    pub max_gas_per_predicate: u64,
    /// Factor to convert between gas and transaction assets value.
    pub gas_price_factor: u64,
    /// A fixed ratio linking metered bytes to gas price
    pub gas_per_byte: u64,
    /// Maximum length of message data, in bytes.
    pub max_message_data_length: u64,
    /// The unique identifier of this chain
    pub chain_id: u64,
}

impl ConsensusParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        contract_max_size: 16 * 1024 * 1024,
        max_inputs: 255,
        max_outputs: 255,
        max_witnesses: 255,
        max_gas_per_tx: MAX_GAS,
        max_script_length: 1024 * 1024,
        max_script_data_length: 1024 * 1024,
        max_storage_slots: 255,
        max_predicate_length: 1024 * 1024,
        max_predicate_data_length: 1024 * 1024,
        max_gas_per_predicate: MAX_GAS,
        gas_price_factor: 1_000_000_000,
        gas_per_byte: 4,
        max_message_data_length: 1024 * 1024,
        chain_id: 0,
    };

    /// Transaction memory offset in VM runtime
    pub const fn tx_offset(&self) -> usize {
        Bytes32::LEN // Tx ID
            + WORD_SIZE // Tx size
            // Asset ID/Balance coin input pairs
            + self.max_inputs as usize * (AssetId::LEN + WORD_SIZE)
    }

    /// Replace the max contract size with the given argument
    pub const fn with_contract_max_size(mut self, contract_max_size: u64) -> Self {
        self.contract_max_size = contract_max_size;
        self
    }

    /// Replace the max inputs with the given argument
    pub const fn with_max_inputs(mut self, max_inputs: u64) -> Self {
        self.max_inputs = max_inputs;
        self
    }

    /// Replace the max outputs with the given argument
    pub const fn with_max_outputs(mut self, max_outputs: u64) -> Self {
        self.max_outputs = max_outputs;
        self
    }

    /// Replace the max witnesses with the given argument
    pub const fn with_max_witnesses(mut self, max_witnesses: u64) -> Self {
        self.max_witnesses = max_witnesses;
        self
    }

    /// Replace the max gas per transaction with the given argument
    pub const fn with_max_gas_per_tx(mut self, max_gas_per_tx: u64) -> Self {
        self.max_gas_per_tx = max_gas_per_tx;
        self
    }

    /// Replace the max script length with the given argument
    pub const fn with_max_script_length(mut self, max_script_length: u64) -> Self {
        self.max_script_length = max_script_length;
        self
    }

    /// Replace the max script data length with the given argument
    pub const fn with_max_script_data_length(mut self, max_script_data_length: u64) -> Self {
        self.max_script_data_length = max_script_data_length;
        self
    }

    /// Replace the max storage slots with the given argument
    pub const fn with_max_storage_slots(mut self, max_storage_slots: u64) -> Self {
        self.max_storage_slots = max_storage_slots;
        self
    }

    /// Replace the max predicate length with the given argument
    pub const fn with_max_predicate_length(mut self, max_predicate_length: u64) -> Self {
        self.max_predicate_length = max_predicate_length;
        self
    }

    /// Replace the max predicate data length with the given argument
    pub const fn with_max_predicate_data_length(mut self, max_predicate_data_length: u64) -> Self {
        self.max_predicate_data_length = max_predicate_data_length;
        self
    }

    /// Replace the max gas per predicate with the given argument
    pub const fn with_max_gas_per_predicate(mut self, max_gas_per_predicate: u64) -> Self {
        self.max_gas_per_predicate = max_gas_per_predicate;
        self
    }

    /// Replace the gas price factor with the given argument
    pub const fn with_gas_price_factor(mut self, gas_price_factor: u64) -> Self {
        self.gas_price_factor = gas_price_factor;
        self
    }

    pub const fn with_gas_per_byte(mut self, gas_per_byte: u64) -> Self {
        self.gas_per_byte = gas_per_byte;
        self
    }

    /// Replace the max message data length with the given argument
    pub const fn with_max_message_data_length(mut self, max_message_data_length: u64) -> Self {
        self.max_message_data_length = max_message_data_length;
        self
    }
}

impl Default for ConsensusParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Arbitrary default consensus parameters. While best-efforts are made to adjust these to
/// reasonable settings, they may not be useful for every network instantiation.
#[deprecated(since = "0.12.2", note = "use `ConsensusParameters` instead.")]
pub mod default_parameters {
    use super::ConsensusParameters;

    pub const CONTRACT_MAX_SIZE: u64 = ConsensusParameters::DEFAULT.contract_max_size;
    pub const MAX_INPUTS: u64 = ConsensusParameters::DEFAULT.max_inputs;
    pub const MAX_OUTPUTS: u64 = ConsensusParameters::DEFAULT.max_outputs;
    pub const MAX_WITNESSES: u64 = ConsensusParameters::DEFAULT.max_witnesses;
    pub const MAX_GAS_PER_TX: u64 = ConsensusParameters::DEFAULT.max_gas_per_tx;
    pub const MAX_SCRIPT_LENGTH: u64 = ConsensusParameters::DEFAULT.max_script_length;
    pub const MAX_SCRIPT_DATA_LENGTH: u64 = ConsensusParameters::DEFAULT.max_script_data_length;
    pub const MAX_STORAGE_SLOTS: u64 = ConsensusParameters::DEFAULT.max_storage_slots;
    pub const MAX_PREDICATE_LENGTH: u64 = ConsensusParameters::DEFAULT.max_predicate_length;
    pub const MAX_PREDICATE_DATA_LENGTH: u64 = ConsensusParameters::DEFAULT.max_predicate_data_length;
    pub const MAX_GAS_PER_PREDICATE: u64 = ConsensusParameters::DEFAULT.max_gas_per_predicate;
    pub const GAS_PRICE_FACTOR: u64 = ConsensusParameters::DEFAULT.gas_price_factor;
    pub const GAS_PER_BYTE: u64 = ConsensusParameters::DEFAULT.gas_per_byte;
    pub const MAX_MESSAGE_DATA_LENGTH: u64 = ConsensusParameters::DEFAULT.max_message_data_length;
    pub const CHAIN_ID: u64 = ConsensusParameters::DEFAULT.chain_id;
}
