use crate::double_key;
use fuel_storage::Mappable;
use fuel_types::{
    AssetId,
    ContractId,
    Word,
};

/// The storage table for contract's assets balances.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsAssets;

impl Mappable for ContractsAssets {
    type Key = Self::OwnedKey;
    type OwnedKey = ContractsAssetKey;
    type OwnedValue = Self::Value;
    type Value = Word;
}

double_key!(
    ContractsAssetKey,
    ContractId,
    contract_id,
    AssetId,
    asset_id
);
