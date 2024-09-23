use fuel_types::{
    bytes::WORD_SIZE,
    Address,
    AssetId,
    Bytes32,
    ChainId,
};

pub mod gas;

pub use gas::{
    DependentCost,
    GasCostNotDefined,
    GasCosts,
    GasCostsValues,
};

use crate::consts::BALANCE_ENTRY_SIZE;

#[cfg(feature = "test-helpers")]
const MAX_GAS: u64 = 100_000_000;
#[cfg(feature = "test-helpers")]
const MAX_SIZE: u64 = 110 * 1024;

#[derive(Debug)]
pub struct SettingBlockTransactionSizeLimitNotSupported;
#[cfg(feature = "std")]
impl std::fmt::Display for SettingBlockTransactionSizeLimitNotSupported {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "setting block transaction size limit is not supported")
    }
}
#[cfg(feature = "std")]
impl std::error::Error for SettingBlockTransactionSizeLimitNotSupported {}

/// A versioned set of consensus parameters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConsensusParameters {
    /// Version 1 of the consensus parameters
    V1(ConsensusParametersV1),
    V2(ConsensusParametersV2),
}

#[cfg(feature = "test-helpers")]
impl Default for ConsensusParameters {
    fn default() -> Self {
        Self::standard()
    }
}

impl ConsensusParameters {
    #[cfg(feature = "test-helpers")]
    /// Constructor for the `ConsensusParameters` with Standard values.
    pub fn standard() -> Self {
        ConsensusParametersV2::standard().into()
    }

    #[cfg(feature = "test-helpers")]
    /// Constructor for the `ConsensusParameters` with Standard values around `ChainId`.
    pub fn standard_with_id(chain_id: ChainId) -> Self {
        ConsensusParametersV2::standard_with_id(chain_id).into()
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
        block_transaction_size_limit: u64,
        privileged_address: Address,
    ) -> Self {
        Self::V2(ConsensusParametersV2 {
            tx_params,
            predicate_params,
            script_params,
            contract_params,
            fee_params,
            chain_id,
            gas_costs,
            base_asset_id,
            block_gas_limit,
            block_transaction_size_limit,
            privileged_address,
        })
    }

    /// Get the transaction parameters
    pub const fn tx_params(&self) -> &TxParameters {
        match self {
            Self::V1(params) => &params.tx_params,
            Self::V2(params) => &params.tx_params,
        }
    }

    /// Get the predicate parameters
    pub const fn predicate_params(&self) -> &PredicateParameters {
        match self {
            Self::V1(params) => &params.predicate_params,
            Self::V2(params) => &params.predicate_params,
        }
    }

    /// Get the script parameters
    pub const fn script_params(&self) -> &ScriptParameters {
        match self {
            Self::V1(params) => &params.script_params,
            Self::V2(params) => &params.script_params,
        }
    }

    /// Get the contract parameters
    pub const fn contract_params(&self) -> &ContractParameters {
        match self {
            Self::V1(params) => &params.contract_params,
            Self::V2(params) => &params.contract_params,
        }
    }

    /// Get the fee parameters
    pub const fn fee_params(&self) -> &FeeParameters {
        match self {
            Self::V1(params) => &params.fee_params,
            Self::V2(params) => &params.fee_params,
        }
    }

    /// Get the chain ID
    pub const fn chain_id(&self) -> ChainId {
        match self {
            Self::V1(params) => params.chain_id,
            Self::V2(params) => params.chain_id,
        }
    }

    /// Get the gas costs
    pub const fn gas_costs(&self) -> &GasCosts {
        match self {
            Self::V1(params) => &params.gas_costs,
            Self::V2(params) => &params.gas_costs,
        }
    }

    /// Get the base asset ID
    pub const fn base_asset_id(&self) -> &AssetId {
        match self {
            Self::V1(params) => &params.base_asset_id,
            Self::V2(params) => &params.base_asset_id,
        }
    }

    /// Get the block gas limit
    pub const fn block_gas_limit(&self) -> u64 {
        match self {
            Self::V1(params) => params.block_gas_limit,
            Self::V2(params) => params.block_gas_limit,
        }
    }

    /// Get the block transaction size limit
    pub fn block_transaction_size_limit(&self) -> u64 {
        match self {
            Self::V1(_) => {
                // In V1 there was no limit on the transaction size. For the sake of
                // backwards compatibility we allow for a largest limit possible.
                u64::MAX
            }
            Self::V2(params) => params.block_transaction_size_limit,
        }
    }

    /// Get the privileged address
    pub const fn privileged_address(&self) -> &Address {
        match self {
            Self::V1(params) => &params.privileged_address,
            Self::V2(params) => &params.privileged_address,
        }
    }
}

impl ConsensusParameters {
    /// Set the transaction parameters.
    pub fn set_tx_params(&mut self, tx_params: TxParameters) {
        match self {
            Self::V1(params) => params.tx_params = tx_params,
            Self::V2(params) => params.tx_params = tx_params,
        }
    }

    /// Set the predicate parameters.
    pub fn set_predicate_params(&mut self, predicate_params: PredicateParameters) {
        match self {
            Self::V1(params) => params.predicate_params = predicate_params,
            Self::V2(params) => params.predicate_params = predicate_params,
        }
    }

    /// Set the script parameters.
    pub fn set_script_params(&mut self, script_params: ScriptParameters) {
        match self {
            Self::V1(params) => params.script_params = script_params,
            Self::V2(params) => params.script_params = script_params,
        }
    }

    /// Set the contract parameters.
    pub fn set_contract_params(&mut self, contract_params: ContractParameters) {
        match self {
            Self::V1(params) => params.contract_params = contract_params,
            Self::V2(params) => params.contract_params = contract_params,
        }
    }

    /// Set the fee parameters.
    pub fn set_fee_params(&mut self, fee_params: FeeParameters) {
        match self {
            Self::V1(params) => params.fee_params = fee_params,
            Self::V2(params) => params.fee_params = fee_params,
        }
    }

    /// Set the chain ID.
    pub fn set_chain_id(&mut self, chain_id: ChainId) {
        match self {
            Self::V1(params) => params.chain_id = chain_id,
            Self::V2(params) => params.chain_id = chain_id,
        }
    }

    /// Set the gas costs.
    pub fn set_gas_costs(&mut self, gas_costs: GasCosts) {
        match self {
            Self::V1(params) => params.gas_costs = gas_costs,
            Self::V2(params) => params.gas_costs = gas_costs,
        }
    }

    /// Set the base asset ID.
    pub fn set_base_asset_id(&mut self, base_asset_id: AssetId) {
        match self {
            Self::V1(params) => params.base_asset_id = base_asset_id,
            Self::V2(params) => params.base_asset_id = base_asset_id,
        }
    }

    /// Set the block gas limit.
    pub fn set_block_gas_limit(&mut self, block_gas_limit: u64) {
        match self {
            Self::V1(params) => params.block_gas_limit = block_gas_limit,
            Self::V2(params) => params.block_gas_limit = block_gas_limit,
        }
    }

    /// Set the block transaction size limit.
    pub fn set_block_transaction_size_limit(
        &mut self,
        block_transaction_size_limit: u64,
    ) -> Result<(), SettingBlockTransactionSizeLimitNotSupported> {
        match self {
            Self::V1(_) => Err(SettingBlockTransactionSizeLimitNotSupported),
            Self::V2(params) => {
                params.block_transaction_size_limit = block_transaction_size_limit;
                Ok(())
            }
        }
    }

    /// Set the privileged address.
    pub fn set_privileged_address(&mut self, privileged_address: Address) {
        match self {
            Self::V1(params) => params.privileged_address = privileged_address,
            Self::V2(params) => params.privileged_address = privileged_address,
        }
    }
}

/// A collection of parameters for convenience
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// The privileged address(user or predicate) that can perform permissioned
    /// operations(like upgrading the network).
    pub privileged_address: Address,
}

#[cfg(feature = "test-helpers")]
impl ConsensusParametersV1 {
    /// Constructor for the `ConsensusParameters` with Standard values.
    pub fn standard() -> Self {
        Self::standard_with_id(ChainId::default())
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
            privileged_address: Default::default(),
        }
    }
}

#[cfg(feature = "test-helpers")]
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

/// A collection of parameters for convenience
/// The difference with [`ConsensusParametersV1`]:
/// - `block_transaction_size_limit` has been added.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ConsensusParametersV2 {
    pub tx_params: TxParameters,
    pub predicate_params: PredicateParameters,
    pub script_params: ScriptParameters,
    pub contract_params: ContractParameters,
    pub fee_params: FeeParameters,
    pub chain_id: ChainId,
    pub gas_costs: GasCosts,
    pub base_asset_id: AssetId,
    pub block_gas_limit: u64,
    pub block_transaction_size_limit: u64,
    /// The privileged address(user or predicate) that can perform permissioned
    /// operations(like upgrading the network).
    pub privileged_address: Address,
}

#[cfg(feature = "test-helpers")]
impl ConsensusParametersV2 {
    const DEFAULT_BLOCK_TRANSACTION_SIZE_LIMIT: u64 = 126 * 1024;

    /// Constructor for the `ConsensusParameters` with Standard values.
    pub fn standard() -> Self {
        Self::standard_with_id(ChainId::default())
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
            block_transaction_size_limit: Self::DEFAULT_BLOCK_TRANSACTION_SIZE_LIMIT,
            privileged_address: Default::default(),
        }
    }
}

#[cfg(feature = "test-helpers")]
impl Default for ConsensusParametersV2 {
    fn default() -> Self {
        Self::standard()
    }
}

impl From<ConsensusParametersV2> for ConsensusParameters {
    fn from(params: ConsensusParametersV2) -> Self {
        Self::V2(params)
    }
}

/// The versioned fee parameters.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum FeeParameters {
    V1(FeeParametersV1),
}

impl FeeParameters {
    #[cfg(feature = "test-helpers")]
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

#[cfg(feature = "test-helpers")]
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
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FeeParametersV1 {
    /// Factor to convert between gas and transaction assets value.
    pub gas_price_factor: u64,
    /// A fixed ratio linking metered bytes to gas price
    pub gas_per_byte: u64,
}

#[cfg(feature = "test-helpers")]
impl FeeParametersV1 {
    /// Default fee parameters just for tests.
    pub const DEFAULT: Self = FeeParametersV1 {
        gas_price_factor: 1_000_000_000,
        gas_per_byte: 4,
    };
}

#[cfg(feature = "test-helpers")]
impl Default for FeeParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned predicate parameters.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum PredicateParameters {
    V1(PredicateParametersV1),
}

impl PredicateParameters {
    #[cfg(feature = "test-helpers")]
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

#[cfg(feature = "test-helpers")]
impl Default for PredicateParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Consensus configurable parameters used for verifying transactions
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
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

#[cfg(feature = "test-helpers")]
impl PredicateParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        max_predicate_length: 1024 * 1024,
        max_predicate_data_length: 1024 * 1024,
        max_message_data_length: 1024 * 1024,
        max_gas_per_predicate: MAX_GAS,
    };
}

#[cfg(feature = "test-helpers")]
impl Default for PredicateParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned transaction parameters.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TxParameters {
    /// Version 1 of the transaction parameters.
    V1(TxParametersV1),
}

impl TxParameters {
    #[cfg(feature = "test-helpers")]
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self::V1(TxParametersV1::DEFAULT);

    /// Transaction memory offset in VM runtime
    pub const fn tx_offset(&self) -> usize {
        let Some(balances_size) =
            (self.max_inputs() as usize).checked_mul(BALANCE_ENTRY_SIZE)
        else {
            panic!(
                "Consensus parameters shouldn't allow max_inputs to cause overflow here"
            );
        };

        balances_size.saturating_add(
            Bytes32::LEN // Tx ID
            + WORD_SIZE // Tx size
            + AssetId::LEN, // Base asset ID
        )
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

    /// Replace the max bytecode subsections with the given argument
    pub const fn with_max_bytecode_subsections(
        self,
        max_bytecode_subsections: u16,
    ) -> Self {
        match self {
            Self::V1(mut params) => {
                params.max_bytecode_subsections = max_bytecode_subsections;
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

    /// Get the maximum number of bytecode subsections.
    pub const fn max_bytecode_subsections(&self) -> u16 {
        match self {
            Self::V1(params) => params.max_bytecode_subsections,
        }
    }
}

#[cfg(feature = "test-helpers")]
impl Default for TxParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(feature = "test-helpers")]
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

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
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
    /// Maximum number of bytecode subsections.
    pub max_bytecode_subsections: u16,
}

#[cfg(feature = "test-helpers")]
impl TxParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        max_inputs: 255,
        max_outputs: 255,
        max_witnesses: 255,
        max_gas_per_tx: MAX_GAS,
        max_size: MAX_SIZE,
        max_bytecode_subsections: 255,
    };
}

#[cfg(feature = "test-helpers")]
impl Default for TxParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned script parameters.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ScriptParameters {
    V1(ScriptParametersV1),
}

impl ScriptParameters {
    #[cfg(feature = "test-helpers")]
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

#[cfg(feature = "test-helpers")]
impl Default for ScriptParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ScriptParametersV1 {
    /// Maximum length of script, in instructions.
    pub max_script_length: u64,
    /// Maximum length of script data, in bytes.
    pub max_script_data_length: u64,
}

#[cfg(feature = "test-helpers")]
impl ScriptParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        max_script_length: 1024 * 1024,
        max_script_data_length: 1024 * 1024,
    };
}

#[cfg(feature = "test-helpers")]
impl Default for ScriptParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Versioned contract parameters.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ContractParameters {
    V1(ContractParametersV1),
}

impl ContractParameters {
    #[cfg(feature = "test-helpers")]
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

#[cfg(feature = "test-helpers")]
impl Default for ContractParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ContractParametersV1 {
    /// Maximum contract size, in bytes.
    pub contract_max_size: u64,

    /// Maximum number of initial storage slots.
    pub max_storage_slots: u64,
}

#[cfg(feature = "test-helpers")]
impl ContractParametersV1 {
    /// Default parameters just for testing.
    pub const DEFAULT: Self = Self {
        contract_max_size: 100 * 1024,
        max_storage_slots: 255,
    };
}

#[cfg(feature = "test-helpers")]
impl Default for ContractParametersV1 {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use super::{
        PredicateParameters as PredicateParametersRust,
        PredicateParametersV1,
    };

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
    pub struct PredicateParameters(alloc::boxed::Box<PredicateParametersRust>);

    impl AsRef<PredicateParametersRust> for PredicateParameters {
        fn as_ref(&self) -> &PredicateParametersRust {
            &self.0
        }
    }

    #[wasm_bindgen]
    impl PredicateParameters {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new(
            max_predicate_length: u64,
            max_predicate_data_length: u64,
            max_message_data_length: u64,
            max_gas_per_predicate: u64,
        ) -> Self {
            let params: PredicateParametersRust = PredicateParametersV1 {
                max_predicate_length,
                max_predicate_data_length,
                max_message_data_length,
                max_gas_per_predicate,
            }
            .into();

            PredicateParameters(params.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus_parameters::{
        ConsensusParametersV2,
        SettingBlockTransactionSizeLimitNotSupported,
    };

    use super::{
        ConsensusParameters,
        ConsensusParametersV1,
    };

    #[test]
    fn error_when_setting_block_size_limit_in_consensus_parameters_v1() {
        let mut consensus_params: ConsensusParameters =
            ConsensusParametersV1::default().into();

        let result = consensus_params.set_block_transaction_size_limit(0);

        assert!(matches!(
            result,
            Err(SettingBlockTransactionSizeLimitNotSupported)
        ))
    }

    #[test]
    fn ok_when_setting_block_size_limit_in_consensus_parameters_v2() {
        let mut consensus_params: ConsensusParameters =
            ConsensusParametersV2::default().into();

        let result = consensus_params.set_block_transaction_size_limit(0);

        assert!(matches!(result, Ok(())))
    }
}
