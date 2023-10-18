use fuel_types::{
    bytes::WORD_SIZE,
    AssetId,
    Bytes32,
    ChainId,
};

pub mod gas;

pub use gas::{
    DependentCost,
    GasCosts,
    GasCostsValues,
    GasUnit,
};

const MAX_GAS: u64 = 100_000_000;
const MAX_SIZE: u64 = 17 * 1024 * 1024;

/// A collection of parameters for convenience
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ConsensusParameters {
    pub tx_params: TxParameters,
    pub predicate_params: PredicateParameters,
    pub script_params: ScriptParameters,
    pub contract_params: ContractParameters,
    pub fee_params: FeeParameters,
    pub chain_id: ChainId,
    pub gas_costs: GasCosts,
    pub base_asset_id: AssetId,
}

impl Default for ConsensusParameters {
    fn default() -> Self {
        Self::standard()
    }
}

impl ConsensusParameters {
    /// Constructor for the `ConsensusParameters` with Standard values.
    pub fn standard() -> Self {
        Self {
            tx_params: TxParameters::DEFAULT,
            predicate_params: PredicateParameters::DEFAULT,
            script_params: ScriptParameters::DEFAULT,
            contract_params: ContractParameters::DEFAULT,
            fee_params: FeeParameters::DEFAULT,
            chain_id: ChainId::default(),
            gas_costs: GasCosts::default(),
            base_asset_id: Default::default(),
        }
    }

    /// Constructor for the `ConsensusParameters` with Standard values around `ChainId`.
    pub fn standard_with_id(chain_id: ChainId) -> Self {
        Self {
            tx_params: TxParameters::DEFAULT,
            predicate_params: PredicateParameters::DEFAULT,
            script_params: ScriptParameters::DEFAULT,
            contract_params: ContractParameters::DEFAULT,
            fee_params: FeeParameters::DEFAULT,
            chain_id,
            gas_costs: GasCosts::default(),
            base_asset_id: Default::default(),
        }
    }

    /// Constructor for the `ConsensusParameters`
    pub const fn new(
        tx_params: TxParameters,
        predicate_params: PredicateParameters,
        script_params: ScriptParameters,
        contract_params: ContractParameters,
        fee_params: FeeParameters,
        chain_id: ChainId,
        gas_costs: GasCosts,
        base_asset_id: AssetId,
    ) -> Self {
        Self {
            tx_params,
            predicate_params,
            script_params,
            contract_params,
            fee_params,
            chain_id,
            gas_costs,
            base_asset_id,
        }
    }

    /// Get the transaction parameters
    pub fn tx_params(&self) -> &TxParameters {
        &self.tx_params
    }

    /// Get the predicate parameters
    pub fn predicate_params(&self) -> &PredicateParameters {
        &self.predicate_params
    }

    /// Get the script parameters
    pub fn script_params(&self) -> &ScriptParameters {
        &self.script_params
    }

    /// Get the contract parameters
    pub fn contract_params(&self) -> &ContractParameters {
        &self.contract_params
    }

    /// Get the fee parameters
    pub fn fee_params(&self) -> &FeeParameters {
        &self.fee_params
    }

    pub fn base_asset_id(&self) -> &AssetId {
        &self.base_asset_id
    }

    /// Get the chain ID
    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    /// Get the gas costs
    pub fn gas_costs(&self) -> &GasCosts {
        &self.gas_costs
    }
}

/// Consensus configurable parameters used for verifying transactions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FeeParameters {
    /// Factor to convert between gas and transaction assets value.
    pub gas_price_factor: u64,
    /// A fixed ratio linking metered bytes to gas price
    pub gas_per_byte: u64,
}

impl FeeParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        gas_price_factor: 1_000_000_000,
        gas_per_byte: 4,
    };

    /// Replace the gas price factor with the given argument
    pub const fn with_gas_price_factor(mut self, gas_price_factor: u64) -> Self {
        self.gas_price_factor = gas_price_factor;
        self
    }

    pub const fn with_gas_per_byte(mut self, gas_per_byte: u64) -> Self {
        self.gas_per_byte = gas_per_byte;
        self
    }
}

impl Default for FeeParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Consensus configurable parameters used for verifying transactions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct PredicateParameters {
    /// Maximum length of predicate, in instructions.
    pub max_predicate_length: u64,
    /// Maximum length of predicate data, in bytes.
    pub max_predicate_data_length: u64,
    /// Maximum length of message data, in bytes.
    pub max_message_data_length: u64,
    /// Maximum gas spent per predicate
    pub max_gas_per_predicate: u64,
}

impl PredicateParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        max_predicate_length: 1024 * 1024,
        max_predicate_data_length: 1024 * 1024,
        max_message_data_length: 1024 * 1024,
        max_gas_per_predicate: MAX_GAS,
    };

    /// Replace the max predicate length with the given argument
    pub const fn with_max_predicate_length(mut self, max_predicate_length: u64) -> Self {
        self.max_predicate_length = max_predicate_length;
        self
    }

    /// Replace the max predicate data length with the given argument
    pub const fn with_max_predicate_data_length(
        mut self,
        max_predicate_data_length: u64,
    ) -> Self {
        self.max_predicate_data_length = max_predicate_data_length;
        self
    }

    /// Replace the max message data length with the given argument
    pub const fn with_max_message_data_length(
        mut self,
        max_message_data_length: u64,
    ) -> Self {
        self.max_message_data_length = max_message_data_length;
        self
    }
}

impl Default for PredicateParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TxParameters {
    /// Maximum number of inputs.
    pub max_inputs: u64,
    /// Maximum number of outputs.
    pub max_outputs: u64,
    /// Maximum number of witnesses.
    pub max_witnesses: u64,
    /// Maximum gas per transaction.
    pub max_gas_per_tx: u64,
    /// Maximum size in bytes
    pub max_size: u64,
}

impl TxParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        max_inputs: 255,
        max_outputs: 255,
        max_witnesses: 255,
        max_gas_per_tx: MAX_GAS,
        max_size: MAX_SIZE,
    };

    /// Transaction memory offset in VM runtime
    pub const fn tx_offset(&self) -> usize {
        Bytes32::LEN // Tx ID
            + WORD_SIZE // Tx size
            // Asset ID/Balance coin input pairs
            + self.max_inputs as usize * (AssetId::LEN + WORD_SIZE)
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

    /// Replace the max size of the transaction with the given argument
    pub const fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = max_size;
        self
    }
}

impl Default for TxParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ScriptParameters {
    /// Maximum length of script, in instructions.
    pub max_script_length: u64,
    /// Maximum length of script data, in bytes.
    pub max_script_data_length: u64,
}

impl ScriptParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        max_script_length: 1024 * 1024,
        max_script_data_length: 1024 * 1024,
    };

    /// Replace the max script length with the given argument
    pub const fn with_max_script_length(mut self, max_script_length: u64) -> Self {
        self.max_script_length = max_script_length;
        self
    }

    /// Replace the max script data length with the given argument
    pub const fn with_max_script_data_length(
        mut self,
        max_script_data_length: u64,
    ) -> Self {
        self.max_script_data_length = max_script_data_length;
        self
    }
}

impl Default for ScriptParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ContractParameters {
    /// Maximum contract size, in bytes.
    pub contract_max_size: u64,

    /// Maximum number of initial storage slots.
    pub max_storage_slots: u64,
}

impl ContractParameters {
    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        contract_max_size: 16 * 1024 * 1024,
        max_storage_slots: 255,
    };

    /// Replace the max contract size with the given argument
    pub const fn with_contract_max_size(mut self, contract_max_size: u64) -> Self {
        self.contract_max_size = contract_max_size;
        self
    }

    /// Replace the max storage slots with the given argument
    pub const fn with_max_storage_slots(mut self, max_storage_slots: u64) -> Self {
        self.max_storage_slots = max_storage_slots;
        self
    }
}

impl Default for ContractParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Arbitrary default consensus parameters. While best-efforts are made to adjust these to
/// reasonable settings, they may not be useful for every network instantiation.
#[deprecated(since = "0.12.2", note = "use `ConsensusParameters` instead.")]
pub mod default_parameters {
    use crate::{
        transaction::consensus_parameters::{
            PredicateParameters,
            ScriptParameters,
            TxParameters,
        },
        ContractParameters,
        FeeParameters,
    };
    use fuel_types::ChainId;

    pub const CONTRACT_MAX_SIZE: u64 = ContractParameters::DEFAULT.contract_max_size;
    pub const MAX_INPUTS: u64 = TxParameters::DEFAULT.max_inputs;
    pub const MAX_OUTPUTS: u64 = TxParameters::DEFAULT.max_outputs;
    pub const MAX_WITNESSES: u64 = TxParameters::DEFAULT.max_witnesses;
    pub const MAX_GAS_PER_TX: u64 = TxParameters::DEFAULT.max_gas_per_tx;

    pub const MAX_SCRIPT_LENGTH: u64 = ScriptParameters::DEFAULT.max_script_length;
    pub const MAX_SCRIPT_DATA_LENGTH: u64 =
        ScriptParameters::DEFAULT.max_script_data_length;

    pub const MAX_STORAGE_SLOTS: u64 = ContractParameters::DEFAULT.max_storage_slots;

    pub const MAX_PREDICATE_LENGTH: u64 =
        PredicateParameters::DEFAULT.max_predicate_length;
    pub const MAX_PREDICATE_DATA_LENGTH: u64 =
        PredicateParameters::DEFAULT.max_predicate_data_length;
    pub const MAX_MESSAGE_DATA_LENGTH: u64 =
        PredicateParameters::DEFAULT.max_message_data_length;

    pub const MAX_GAS_PER_PREDICATE: u64 =
        PredicateParameters::DEFAULT.max_gas_per_predicate;
    pub const GAS_PRICE_FACTOR: u64 = FeeParameters::DEFAULT.gas_price_factor;
    pub const GAS_PER_BYTE: u64 = FeeParameters::DEFAULT.gas_per_byte;

    pub const CHAIN_ID: ChainId = ChainId::new(0);
}
