use fuel_types::bytes::WORD_SIZE;
use fuel_types::{AssetId, Bytes32};

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
    /// Factor to convert between gas and transaction assets value.
    pub gas_price_factor: u64,
}

impl ConsensusParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        contract_max_size: 16 * 1024 * 1024,
        max_inputs: 255,
        max_outputs: 255,
        max_witnesses: 255,
        max_gas_per_tx: 100_000_000,
        max_script_length: 1024 * 1024,
        max_script_data_length: 1024 * 1024,
        max_static_contracts: 255,
        max_storage_slots: 255,
        max_predicate_length: 1024 * 1024,
        max_predicate_data_length: 1024 * 1024,
        gas_price_factor: 1_000_000_000,
    };

    /// Transaction memory offset in VM runtime
    pub const fn tx_offset(&self) -> usize {
        Bytes32::LEN // Tx ID
            + WORD_SIZE // Tx size
              // Asset ID/Balance coin input pairs
            + self.max_inputs as usize * (AssetId::LEN + WORD_SIZE)
    }

    /// Replace the max contract size with the given argument
    pub const fn with_contract_max_size(self, contract_max_size: u64) -> Self {
        let Self {
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max inputs with the given argument
    pub const fn with_max_inputs(self, max_inputs: u64) -> Self {
        let Self {
            contract_max_size,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max outputs with the given argument
    pub const fn with_max_outputs(self, max_outputs: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max witnesses with the given argument
    pub const fn with_max_witnesses(self, max_witnesses: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max gas per transaction with the given argument
    pub const fn with_max_gas_per_tx(self, max_gas_per_tx: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max script length with the given argument
    pub const fn with_max_script_length(self, max_script_length: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max script data length with the given argument
    pub const fn with_max_script_data_length(self, max_script_data_length: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max static contracts with the given argument
    pub const fn with_max_static_contracts(self, max_static_contracts: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max storage slots with the given argument
    pub const fn with_max_storage_slots(self, max_storage_slots: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max predicate length with the given argument
    pub const fn with_max_predicate_length(self, max_predicate_length: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_data_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the max predicate data length with the given argument
    pub const fn with_max_predicate_data_length(self, max_predicate_data_length: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            gas_price_factor,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
    }

    /// Replace the gas price factor with the given argument
    pub const fn with_gas_price_factor(self, gas_price_factor: u64) -> Self {
        let Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            ..
        } = self;

        Self {
            contract_max_size,
            max_inputs,
            max_outputs,
            max_witnesses,
            max_gas_per_tx,
            max_script_length,
            max_script_data_length,
            max_static_contracts,
            max_storage_slots,
            max_predicate_length,
            max_predicate_data_length,
            gas_price_factor,
        }
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
    pub const MAX_STATIC_CONTRACTS: u64 = ConsensusParameters::DEFAULT.max_static_contracts;
    pub const MAX_STORAGE_SLOTS: u64 = ConsensusParameters::DEFAULT.max_storage_slots;
    pub const MAX_PREDICATE_LENGTH: u64 = ConsensusParameters::DEFAULT.max_predicate_length;
    pub const MAX_PREDICATE_DATA_LENGTH: u64 =
        ConsensusParameters::DEFAULT.max_predicate_data_length;
    pub const GAS_PRICE_FACTOR: u64 = ConsensusParameters::DEFAULT.gas_price_factor;
}
