use fuel_tx::ConsensusParameters;

// override default settings to reduce testing overhead
pub const PARAMS: ConsensusParameters = ConsensusParameters::DEFAULT
    .with_max_storage_slots(1024)
    .with_max_script_length(1024)
    .with_max_script_data_length(1024)
    .with_max_inputs(16)
    .with_max_outputs(16)
    .with_max_witnesses(16);

pub mod valid_cases;
