use fuel_tx::{
    ConsensusParams,
    ContractParameters,
    FeeParameters,
    PredicateParameters,
    ScriptParameters,
    TxParameters,
};

use fuel_types::ChainId;

// override default settings to reduce testing overhead
pub const CONTRACT_PARAMS: ContractParameters =
    ContractParameters::DEFAULT.with_max_storage_slots(1024);

pub const SCRIPT_PARAMS: ScriptParameters = ScriptParameters::DEFAULT
    .with_max_script_length(1024)
    .with_max_script_data_length(1024);

pub const TX_PARAMS: TxParameters = TxParameters::DEFAULT
    .with_max_inputs(16)
    .with_max_outputs(16)
    .with_max_witnesses(16);

pub const PREDICATE_PARAMS: PredicateParameters = PredicateParameters::DEFAULT;

pub const FEE_PARAMS: FeeParameters = FeeParameters::DEFAULT;

pub const CHAIN_ID: ChainId = ChainId::new(0);

pub const PARAMS: ConsensusParams = ConsensusParams {
    tx_params: &TX_PARAMS,
    predicate_params: &PREDICATE_PARAMS,
    script_params: &SCRIPT_PARAMS,
    contract_params: &CONTRACT_PARAMS,
    fee_params: &FEE_PARAMS,
};

mod input;
mod output;
mod transaction;
