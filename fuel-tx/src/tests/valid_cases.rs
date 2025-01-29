use crate::{
    ConsensusParameters,
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

pub fn test_params() -> ConsensusParameters {
    ConsensusParameters::new(
        TX_PARAMS,
        PREDICATE_PARAMS,
        SCRIPT_PARAMS,
        CONTRACT_PARAMS,
        FEE_PARAMS,
        CHAIN_ID,
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    )
}

mod input;
mod output;
mod transaction;
