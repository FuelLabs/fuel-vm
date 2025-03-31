use core::num::NonZeroU64;

/// Versioned gas price metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GasPriceMetadata {
    V1(GasPriceMetadataV1),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GasPriceMetadataV1 {
    pub new_exec_gas_price: u64,
    pub min_exec_gas_price: u64,
    pub exec_gas_price_change_percent: u16,
    pub l2_block_fullness_threshold_percent: u8,
    // TODO:We don't need this after we implement
    // https://github.com/FuelLabs/fuel-core/issues/2481
    pub gas_price_factor: NonZeroU64,
    pub min_da_gas_price: u64,
    pub max_da_gas_price: u64,
    pub max_da_gas_price_change_percent: u16,
    pub da_p_component: i64,
    pub da_d_component: i64,
    pub normal_range_size: u16,
    pub capped_range_size: u16,
    pub decrease_range_size: u16,
    pub block_activity_threshold: u8,
}

impl Default for GasPriceMetadata {
    fn default() -> Self {
        Self::V1(GasPriceMetadataV1 {
            new_exec_gas_price: 100,
            min_exec_gas_price: 0,
            exec_gas_price_change_percent: 10,
            l2_block_fullness_threshold_percent: 0,
            gas_price_factor: NonZeroU64::new(100).unwrap(),
            min_da_gas_price: 0,
            max_da_gas_price: 1,
            max_da_gas_price_change_percent: 0,
            da_p_component: 0,
            da_d_component: 0,
            normal_range_size: 0,
            capped_range_size: 0,
            decrease_range_size: 0,
            block_activity_threshold: 0,
        })
    }
}
