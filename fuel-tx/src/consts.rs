use fuel_types::{
    AssetId,
    bytes::WORD_SIZE,
};

/// Size of balance entry, i.e. asset id and associated balance.
pub const BALANCE_ENTRY_SIZE: usize = AssetId::LEN + WORD_SIZE;
