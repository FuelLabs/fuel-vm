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
    pub const fn tx_params(&self) -> &TxParameters {
        match self {
            Self::V1(params) => &params.tx_params,
        }
    }

    /// Get the predicate parameters
    pub const fn predicate_params(&self) -> &PredicateParameters {
        match self {
            Self::V1(params) => &params.predicate_params,
        }
    }

    /// Get the script parameters
    pub const fn script_params(&self) -> &ScriptParameters {
        match self {
            Self::V1(params) => &params.script_params,
        }
    }

    /// Get the contract parameters
    pub const fn contract_params(&self) -> &ContractParameters {
        match self {
            Self::V1(params) => &params.contract_params,
        }
    }

    /// Get the fee parameters
    pub const fn fee_params(&self) -> &FeeParameters {
        match self {
            Self::V1(params) => &params.fee_params,
        }
    }

    /// Get the chain ID
    pub const fn chain_id(&self) -> ChainId {
        match self {
            Self::V1(params) => params.chain_id,
        }
    }

    /// Get the gas costs
    pub const fn gas_costs(&self) -> &GasCosts {
        match self {
            Self::V1(params) => &params.gas_costs,
        }
    }

    /// Get the base asset ID
    pub const fn base_asset_id(&self) -> &AssetId {
        match self {
            Self::V1(params) => &params.base_asset_id,
        }
    }

    /// Get the block gas limit
    pub const fn block_gas_limit(&self) -> u64 {
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
            block_gas_limit: TxParameters::DEFAULT.max_gas_per_tx(),
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
            block_gas_limit: TxParameters::DEFAULT.max_gas_per_tx(),
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

/// The versioned fee parameters.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FeeParameters {
    V1(FeeParametersV1),
}

impl FeeParameters {
    /// Default fee parameters just for testing.
    pub const DEFAULT: Self = Self::V1(FeeParametersV1::DEFAULT);

    /// Replace the gas price factor with the given argument
    pub const fn with_gas_price_factor(self, gas_price_factor: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.gas_price_factor = gas_price_factor;
                Self::V1(params)
            }
        }
    }

    pub const fn with_gas_per_byte(self, gas_per_byte: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.gas_per_byte = gas_per_byte;
                Self::V1(params)
            }
        }
    }
}

impl FeeParameters {
    /// Get the gas price factor
    pub const fn gas_price_factor(&self) -> u64 {
        match self {
            Self::V1(params) => params.gas_price_factor,
        }
    }

    /// Get the gas per byte
    pub const fn gas_per_byte(&self) -> u64 {
        match self {
            Self::V1(params) => params.gas_per_byte,
        }
    }
}

impl Default for FeeParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<FeeParametersV1> for FeeParameters {
    fn from(params: FeeParametersV1) -> Self {
        Self::V1(params)
    }
}

/// Consensus configurable parameters used for verifying transactions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FeeParametersV1 {
    /// Factor to convert between gas and transaction assets value.
    pub gas_price_factor: u64,
    /// A fixed ratio linking metered bytes to gas price
    pub gas_per_byte: u64,
}

impl FeeParametersV1 {
    /// Default fee parameters just for tests.
    pub const DEFAULT: Self = FeeParametersV1 {
        gas_price_factor: 1_000_000_000,
        gas_per_byte: 4,
    };
}

impl Default for FeeParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned predicate parameters.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PredicateParameters {
    V1(PredicateParametersV1),
}

impl PredicateParameters {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self::V1(PredicateParametersV1::DEFAULT);

    /// Replace the max predicate length with the given argument
    pub const fn with_max_predicate_length(self, max_predicate_length: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_predicate_length = max_predicate_length;
                Self::V1(params)
            }
        }
    }

    /// Replace the max predicate data length with the given argument
    pub const fn with_max_predicate_data_length(
        self,
        max_predicate_data_length: u64,
    ) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_predicate_data_length = max_predicate_data_length;
                Self::V1(params)
            }
        }
    }

    /// Replace the max message data length with the given argument
    pub const fn with_max_message_data_length(
        self,
        max_message_data_length: u64,
    ) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_message_data_length = max_message_data_length;
                Self::V1(params)
            }
        }
    }

    /// Replace the max gas per predicate.
    pub const fn with_max_gas_per_predicate(self, max_gas_per_predicate: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_gas_per_predicate = max_gas_per_predicate;
                Self::V1(params)
            }
        }
    }
}

impl PredicateParameters {
    /// Get the maximum predicate length
    pub const fn max_predicate_length(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_predicate_length,
        }
    }

    /// Get the maximum predicate data length
    pub const fn max_predicate_data_length(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_predicate_data_length,
        }
    }

    /// Get the maximum message data length
    pub const fn max_message_data_length(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_message_data_length,
        }
    }

    /// Get the maximum gas per predicate
    pub const fn max_gas_per_predicate(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_gas_per_predicate,
        }
    }
}

impl From<PredicateParametersV1> for PredicateParameters {
    fn from(params: PredicateParametersV1) -> Self {
        Self::V1(params)
    }
}

impl Default for PredicateParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Consensus configurable parameters used for verifying transactions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct PredicateParametersV1 {
    /// Maximum length of predicate, in instructions.
    pub max_predicate_length: u64,
    /// Maximum length of predicate data, in bytes.
    pub max_predicate_data_length: u64,
    /// Maximum length of message data, in bytes.
    pub max_message_data_length: u64,
    /// Maximum gas spent per predicate
    pub max_gas_per_predicate: u64,
}

impl PredicateParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        max_predicate_length: 1024 * 1024,
        max_predicate_data_length: 1024 * 1024,
        max_message_data_length: 1024 * 1024,
        max_gas_per_predicate: MAX_GAS,
    };
}

impl Default for PredicateParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned transaction parameters.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TxParameters {
    /// Version 1 of the transaction parameters.
    V1(TxParametersV1),
}

impl TxParameters {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self::V1(TxParametersV1::DEFAULT);

    /// Transaction memory offset in VM runtime
    pub const fn tx_offset(&self) -> usize {
        Bytes32::LEN // Tx ID
            + WORD_SIZE // Tx size
            // Asset ID/Balance coin input pairs
            + self.max_inputs() as usize * (AssetId::LEN + WORD_SIZE)
    }

    /// Replace the max inputs with the given argument
    pub const fn with_max_inputs(self, max_inputs: u16) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_inputs = max_inputs;
                Self::V1(params)
            }
        }
    }

    /// Replace the max outputs with the given argument
    pub const fn with_max_outputs(self, max_outputs: u16) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_outputs = max_outputs;
                Self::V1(params)
            }
        }
    }

    /// Replace the max witnesses with the given argument
    pub const fn with_max_witnesses(self, max_witnesses: u32) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_witnesses = max_witnesses;
                Self::V1(params)
            }
        }
    }

    /// Replace the max gas per transaction with the given argument
    pub const fn with_max_gas_per_tx(self, max_gas_per_tx: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_gas_per_tx = max_gas_per_tx;
                Self::V1(params)
            }
        }
    }

    /// Replace the max size of the transaction with the given argument
    pub const fn with_max_size(self, max_size: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_size = max_size;
                Self::V1(params)
            }
        }
    }
}

impl TxParameters {
    /// Get the maximum number of inputs
    pub const fn max_inputs(&self) -> u16 {
        match self {
            Self::V1(params) => params.max_inputs,
        }
    }

    /// Get the maximum number of outputs
    pub const fn max_outputs(&self) -> u16 {
        match self {
            Self::V1(params) => params.max_outputs,
        }
    }

    /// Get the maximum number of witnesses
    pub const fn max_witnesses(&self) -> u32 {
        match self {
            Self::V1(params) => params.max_witnesses,
        }
    }

    /// Get the maximum gas per transaction
    pub const fn max_gas_per_tx(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_gas_per_tx,
        }
    }

    /// Get the maximum size in bytes
    pub const fn max_size(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_size,
        }
    }
}

impl Default for TxParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(feature = "builder")]
impl TxParameters {
    pub fn set_max_size(&mut self, max_size: u64) {
        match self {
            Self::V1(params) => params.max_size = max_size,
        }
    }
}

impl From<TxParametersV1> for TxParameters {
    fn from(params: TxParametersV1) -> Self {
        Self::V1(params)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TxParametersV1 {
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

impl TxParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        max_inputs: 255,
        max_outputs: 255,
        max_witnesses: 255,
        max_gas_per_tx: MAX_GAS,
        max_size: MAX_SIZE,
    };
}

impl Default for TxParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned script parameters.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScriptParameters {
    V1(ScriptParametersV1),
}

impl ScriptParameters {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self::V1(ScriptParametersV1::DEFAULT);

    /// Replace the max script length with the given argument
    pub const fn with_max_script_length(self, max_script_length: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_script_length = max_script_length;
                Self::V1(params)
            }
        }
    }

    /// Replace the max script data length with the given argument
    pub const fn with_max_script_data_length(self, max_script_data_length: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_script_data_length = max_script_data_length;
                Self::V1(params)
            }
        }
    }
}

impl ScriptParameters {
    /// Get the maximum script length
    pub const fn max_script_length(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_script_length,
        }
    }

    /// Get the maximum script data length
    pub const fn max_script_data_length(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_script_data_length,
        }
    }
}

impl From<ScriptParametersV1> for ScriptParameters {
    fn from(params: ScriptParametersV1) -> Self {
        Self::V1(params)
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
pub struct ScriptParametersV1 {
    /// Maximum length of script, in instructions.
    pub max_script_length: u64,
    /// Maximum length of script data, in bytes.
    pub max_script_data_length: u64,
}

impl ScriptParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        max_script_length: 1024 * 1024,
        max_script_data_length: 1024 * 1024,
    };
}

impl Default for ScriptParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned contract parameters.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ContractParameters {
    V1(ContractParametersV1),
}

impl ContractParameters {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self::V1(ContractParametersV1::DEFAULT);

    /// Replace the max contract size with the given argument
    pub const fn with_contract_max_size(self, contract_max_size: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.contract_max_size = contract_max_size;
                Self::V1(params)
            }
        }
    }

    /// Replace the max storage slots with the given argument
    pub const fn with_max_storage_slots(self, max_storage_slots: u64) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_storage_slots = max_storage_slots;
                Self::V1(params)
            }
        }
    }
}

impl ContractParameters {
    /// Get the maximum contract size
    pub const fn contract_max_size(&self) -> u64 {
        match self {
            Self::V1(params) => params.contract_max_size,
        }
    }

    /// Get the maximum storage slots
    pub const fn max_storage_slots(&self) -> u64 {
        match self {
            Self::V1(params) => params.max_storage_slots,
        }
    }
}

impl From<ContractParametersV1> for ContractParameters {
    fn from(params: ContractParametersV1) -> Self {
        Self::V1(params)
    }
}

impl Default for ContractParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ContractParametersV1 {
    /// Maximum contract size, in bytes.
    pub contract_max_size: u64,

    /// Maximum number of initial storage slots.
    pub max_storage_slots: u64,
}

impl ContractParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        contract_max_size: 100 * 1024,
        max_storage_slots: 255,
    };
}

impl Default for ContractParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use super::PredicateParameters as PredicateParametersRust;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
    pub struct PredicateParameters(Box<PredicateParametersRust>);

    impl AsRef<PredicateParametersRust> for PredicateParameters {
        fn as_ref(&self) -> &PredicateParametersRust {
            &self.0
        }
    }

    #[wasm_bindgen]
    impl PredicateParameters {
        #[wasm_bindgen(constructor)]
        pub fn typescript_default() -> Self {
            PredicateParameters(PredicateParametersRust::DEFAULT.into())
        }

        #[wasm_bindgen(constructor)]
        pub fn typescript_new(
            max_predicate_length: u64,
            max_predicate_data_length: u64,
            max_message_data_length: u64,
            max_gas_per_predicate: u64,
        ) -> Self {
            let params = PredicateParametersRust::default()
                .with_max_predicate_length(max_predicate_length)
                .with_max_predicate_data_length(max_predicate_data_length)
                .with_max_message_data_length(max_message_data_length)
                .with_max_gas_per_predicate(max_gas_per_predicate);

            PredicateParameters(params.into())
        }
    }
}
