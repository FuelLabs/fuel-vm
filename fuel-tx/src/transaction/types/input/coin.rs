use core::default::Default;

use crate::{
    TxPointer,
    UtxoId,
};
use derivative::Derivative;
use fuel_types::{
    Address,
    AssetId,
    Word,
};

use super::predicate::Predicate;

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct CoinCommon {
    pub utxo_id: UtxoId,
    pub owner: Address,
    pub amount: Word,
    pub asset_id: AssetId,
    pub tx_pointer: TxPointer,
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct CoinSigned {
    #[serde(flatten)]
    pub common: CoinCommon,
    pub witness_index: u16,
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct CoinPredicate {
    #[serde(flatten)]
    pub common: CoinCommon,
    #[serde(flatten)]
    pub predicate: Predicate,
}

impl CoinSigned {
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcoin>.
    pub fn prepare_sign(&mut self) {
        self.common.tx_pointer = Default::default();
    }
}

impl CoinPredicate {
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcoin>.
    pub fn prepare_sign(&mut self) {
        self.common.tx_pointer = Default::default();
        self.predicate.prepare_sign();
    }
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct CoinFull {
    #[serde(flatten)]
    pub common: CoinCommon,
    pub witness_index: u16,
    #[serde(flatten)]
    pub predicate: Predicate,
}
