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
const MAX_SIZE: u64 = 110 * 1024;

/// A versioned set of consensus parameters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConsensusParameters {
    /// Version 1 of the consensus parameters
    V1(ConsensusParametersV1),
}

impl Default for ConsensusParameters {
    fn default() -> Self {
        Self::standard()
    }
}

impl ConsensusParameters {
    /// Constructor for the `ConsensusParameters` with Standard values.
    pub fn standard() -> Self {
        ConsensusParametersV1::standard().into()
    }

    /// Constructor for the `ConsensusParameters` with Standard values around `ChainId`.
    pub fn standard_with_id(chain_id: ChainId) -> Self {
        ConsensusParametersV1::standard_with_id(chain_id).into()
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
        block_gas_limit: u64,
    ) -> Self {
        Self::V1(ConsensusParametersV1 {
            tx_params,
            predicate_params,
            script_params,
            contract_params,
            fee_params,
            chain_id,
            gas_costs,
            base_asset_id,
            block_gas_limit,
        })
    }

    /// Get the transaction parameters
    pub fn tx_params(&self) -> &TxParameters {
        match self {
            Self::V1(params) => &params.tx_params,
        }
    }

    /// Get the predicate parameters
    pub fn predicate_params(&self) -> &PredicateParameters {
        match self {
            Self::V1(params) => &params.predicate_params,
        }
    }

    /// Get the script parameters
    pub fn script_params(&self) -> &ScriptParameters {
        match self {
            Self::V1(params) => &params.script_params,
        }
    }

    /// Get the contract parameters
    pub fn contract_params(&self) -> &ContractParameters {
        match self {
            Self::V1(params) => &params.contract_params,
        }
    }

    /// Get the fee parameters
    pub fn fee_params(&self) -> &FeeParameters {
        match self {
            Self::V1(params) => &params.fee_params,
        }
    }

    /// Get the chain ID
    pub fn chain_id(&self) -> ChainId {
        match self {
            Self::V1(params) => params.chain_id,
        }
    }

    /// Get the gas costs
    pub fn gas_costs(&self) -> &GasCosts {
        match self {
            Self::V1(params) => &params.gas_costs,
        }
    }

    /// Get the base asset ID
    pub fn base_asset_id(&self) -> &AssetId {
        match self {
            Self::V1(params) => &params.base_asset_id,
        }
    }

    /// Get the block gas limit
    pub fn block_gas_limit(&self) -> u64 {
        match self {
            Self::V1(params) => params.block_gas_limit,
        }
    }
}

#[cfg(feature = "builder")]
impl ConsensusParameters {
    /// Set the transaction parameters.
    pub fn set_tx_params(&mut self, tx_params: TxParameters) {
        match self {
            Self::V1(params) => params.tx_params = tx_params,
        }
    }

    /// Set the predicate parameters.
    pub fn set_predicate_params(&mut self, predicate_params: PredicateParameters) {
        match self {
            Self::V1(params) => params.predicate_params = predicate_params,
        }
    }

    /// Set the script parameters.
    pub fn set_script_params(&mut self, script_params: ScriptParameters) {
        match self {
            Self::V1(params) => params.script_params = script_params,
        }
    }

    /// Set the contract parameters.
    pub fn set_contract_params(&mut self, contract_params: ContractParameters) {
        match self {
            Self::V1(params) => params.contract_params = contract_params,
        }
    }

    /// Set the fee parameters.
    pub fn set_fee_params(&mut self, fee_params: FeeParameters) {
        match self {
            Self::V1(params) => params.fee_params = fee_params,
        }
    }

    /// Set the chain ID.
    pub fn set_chain_id(&mut self, chain_id: ChainId) {
        match self {
            Self::V1(params) => params.chain_id = chain_id,
        }
    }

    /// Set the gas costs.
    pub fn set_gas_costs(&mut self, gas_costs: GasCosts) {
        match self {
            Self::V1(params) => params.gas_costs = gas_costs,
        }
    }

    /// Set the base asset ID.
    pub fn set_base_asset_id(&mut self, base_asset_id: AssetId) {
        match self {
            Self::V1(params) => params.base_asset_id = base_asset_id,
        }
    }

    /// Set the block gas limit.
    pub fn set_block_gas_limit(&mut self, block_gas_limit: u64) {
        match self {
            Self::V1(params) => params.block_gas_limit = block_gas_limit,
        }
    }
}

/// A collection of parameters for convenience
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ConsensusParametersV1 {
    pub tx_params: TxParameters,
    pub predicate_params: PredicateParameters,
    pub script_params: ScriptParameters,
    pub contract_params: ContractParameters,
    pub fee_params: FeeParameters,
    pub chain_id: ChainId,
    pub gas_costs: GasCosts,
    pub base_asset_id: AssetId,
    pub block_gas_limit: u64,
}

impl ConsensusParametersV1 {
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
            block_gas_limit: TxParameters::DEFAULT.max_gas_per_tx,
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
            block_gas_limit: TxParameters::DEFAULT.max_gas_per_tx,
        }
    }
}

impl Default for ConsensusParametersV1 {
    fn default() -> Self {
        Self::standard()
    }
}

impl From<ConsensusParametersV1> for ConsensusParameters {
    fn from(params: ConsensusParametersV1) -> Self {
        Self::V1(params)
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
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
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
    pub max_inputs: u16,
    /// Maximum number of outputs.
    pub max_outputs: u16,
    /// Maximum number of witnesses.
    pub max_witnesses: u32,
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
    pub const fn with_max_inputs(mut self, max_inputs: u16) -> Self {
        self.max_inputs = max_inputs;
        self
    }

    /// Replace the max outputs with the given argument
    pub const fn with_max_outputs(mut self, max_outputs: u16) -> Self {
        self.max_outputs = max_outputs;
        self
    }

    /// Replace the max witnesses with the given argument
    pub const fn with_max_witnesses(mut self, max_witnesses: u32) -> Self {
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
        contract_max_size: 100 * 1024,
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

#[cfg(feature = "typescript")]
mod typescript {
    use wasm_bindgen::prelude::*;

    use super::PredicateParameters;

    #[wasm_bindgen]
    impl PredicateParameters {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new() -> Self {
            Self::DEFAULT
        }
    }
}
