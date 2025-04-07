use core::default::Default;

use crate::{
    input::{
        fmt_as_field,
        Empty,
    },
    transaction::types::input::AsField,
    TxPointer,
    UtxoId,
};
use alloc::vec::Vec;
use educe::Educe;
#[cfg(feature = "da-compression")]
use fuel_compression::Compressible;
use fuel_types::{
    Address,
    AssetId,
    Word,
};

use super::PredicateCode;

pub type CoinFull = Coin<Full>;
pub type CoinSigned = Coin<Signed>;
pub type CoinPredicate = Coin<Predicate>;

pub type DataCoinFull = DataCoin<Full>;
pub type DataCoinSigned = DataCoin<Signed>;
pub type DataCoinPredicate = DataCoin<Predicate>;

mod private {
    pub trait Seal {}

    impl Seal for super::Full {}
    impl Seal for super::Signed {}
    impl Seal for super::Predicate {}
}

/// Specifies the coin based on the usage context. See [`Coin`].
#[cfg(feature = "da-compression")]
pub trait CoinSpecification: private::Seal {
    type Witness: AsField<u16>
        + for<'a> Compressible<
            Compressed: core::fmt::Debug
                            + PartialEq
                            + Clone
                            + serde::Serialize
                            + serde::Deserialize<'a>,
        >;
    type Predicate: AsField<PredicateCode>
        + for<'a> Compressible<
            Compressed: core::fmt::Debug
                            + PartialEq
                            + Clone
                            + serde::Serialize
                            + serde::Deserialize<'a>,
        >;
    type PredicateData: AsField<Vec<u8>>
        + for<'a> Compressible<
            Compressed: core::fmt::Debug
                            + PartialEq
                            + Clone
                            + serde::Serialize
                            + serde::Deserialize<'a>,
        >;
    type PredicateGasUsed: AsField<Word>
        + for<'a> Compressible<
            Compressed: core::fmt::Debug
                            + PartialEq
                            + Clone
                            + serde::Serialize
                            + serde::Deserialize<'a>,
        >;
}
#[cfg(not(feature = "da-compression"))]
pub trait CoinSpecification: private::Seal {
    type Witness: AsField<u16>;
    type Predicate: AsField<PredicateCode>;
    type PredicateData: AsField<Vec<u8>>;
    type PredicateGasUsed: AsField<Word>;
}

#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
pub struct Signed;

impl CoinSpecification for Signed {
    type Predicate = Empty<PredicateCode>;
    type PredicateData = Empty<Vec<u8>>;
    type PredicateGasUsed = Empty<Word>;
    type Witness = u16;
}

#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
pub struct Predicate;

impl CoinSpecification for Predicate {
    type Predicate = PredicateCode;
    type PredicateData = Vec<u8>;
    type PredicateGasUsed = Word;
    type Witness = Empty<u16>;
}

#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Full;

impl CoinSpecification for Full {
    type Predicate = PredicateCode;
    type PredicateData = Vec<u8>;
    type PredicateGasUsed = Word;
    type Witness = u16;
}

/// It is a full representation of the coin from the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcoin>.
///
/// The specification defines the layout of the [`Coin`] in the serialized form for the
/// `fuel-vm`. But on the business logic level, we don't use all fields at the same time.
/// It is why in the [`super::Input`] the coin is represented by several forms based on
/// the usage context. Leaving some fields empty reduces the memory consumption by the
/// structure and erases the empty useless fields.
///
/// The [`CoinSpecification`] trait specifies the sub-coin for the corresponding usage
/// context. It allows us to write the common logic of all sub-coins without the overhead
/// and duplication.
///
/// Sub-coin:
/// - [`Signed`] - means that the coin should be signed by the `owner`, and the
///   signature(witness) should be stored under the `witness_index` index in the
///   `witnesses` vector of the [`crate::Transaction`].
/// - [`Predicate`] - means that the coin is not signed, and the `owner` is a `predicate`
///   bytecode. The merkle root from the `predicate` should be equal to the `owner`.
/// - [`Full`] - is used during the deserialization of the coin. It should be transformed
///   into [`Signed`] or [`Predicate`] sub-coin. If the `predicate` is empty, it is
///   [`Signed`], else [`Predicate`].
#[derive(Default, Educe, Clone, PartialEq, Eq, Hash)]
#[educe(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "da-compression", derive(fuel_compression::Compress))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct Coin<Specification>
where
    Specification: CoinSpecification,
{
    pub utxo_id: UtxoId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub owner: Address,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub amount: Word,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub asset_id: AssetId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub tx_pointer: TxPointer,
    #[educe(Debug(method(fmt_as_field)))]
    pub witness_index: Specification::Witness,
    /// Exact amount of gas used by the predicate.
    /// If the predicate consumes different amount of gas,
    /// it's considered to be false.
    #[educe(Debug(method(fmt_as_field)))]
    pub predicate_gas_used: Specification::PredicateGasUsed,
    #[educe(Debug(method(fmt_as_field)))]
    pub predicate: Specification::Predicate,
    #[educe(Debug(method(fmt_as_field)))]
    pub predicate_data: Specification::PredicateData,
}

#[derive(Default, Educe, Clone, PartialEq, Eq, Hash)]
#[educe(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "da-compression", derive(fuel_compression::Compress))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct UnverifiedCoin {
    pub utxo_id: UtxoId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub owner: Address,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub amount: Word,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub asset_id: AssetId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub tx_pointer: TxPointer,
}

impl<Specification> Coin<Specification>
where
    Specification: CoinSpecification,
{
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcoin>.
    pub fn prepare_sign(&mut self) {
        self.tx_pointer = Default::default();
        if let Some(predicate_gas_used_field) = self.predicate_gas_used.as_mut_field() {
            *predicate_gas_used_field = Default::default();
        }
    }
}

impl Coin<Full> {
    pub fn into_signed(self) -> Coin<Signed> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            ..
        } = self;

        Coin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            ..Default::default()
        }
    }

    pub fn into_predicate(self) -> Coin<Predicate> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..
        } = self;

        Coin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..Default::default()
        }
    }
}

impl Coin<Signed> {
    pub fn into_full(self) -> Coin<Full> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            ..
        } = self;

        Coin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            ..Default::default()
        }
    }
}

impl Coin<Predicate> {
    pub fn into_full(self) -> Coin<Full> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..
        } = self;

        Coin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..Default::default()
        }
    }
}

#[derive(Default, Educe, Clone, PartialEq, Eq, Hash)]
#[educe(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "da-compression", derive(fuel_compression::Compress))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct DataCoin<Specification>
where
    Specification: CoinSpecification,
{
    pub utxo_id: UtxoId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub owner: Address,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub amount: Word,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub asset_id: AssetId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub tx_pointer: TxPointer,
    #[educe(Debug(method(fmt_as_field)))]
    pub witness_index: Specification::Witness,
    /// Exact amount of gas used by the predicate.
    /// If the predicate consumes different amount of gas,
    /// it's considered to be false.
    #[educe(Debug(method(fmt_as_field)))]
    pub predicate_gas_used: Specification::PredicateGasUsed,
    #[educe(Debug(method(fmt_as_field)))]
    pub predicate: Specification::Predicate,
    #[educe(Debug(method(fmt_as_field)))]
    pub predicate_data: Specification::PredicateData,
    #[educe(Debug(method(fmt_as_field)))]
    pub data: Vec<u8>,
}

#[derive(Default, Educe, Clone, PartialEq, Eq, Hash)]
#[educe(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "da-compression", derive(fuel_compression::Compress))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct UnverifiedDataCoin {
    pub utxo_id: UtxoId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub owner: Address,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub amount: Word,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub asset_id: AssetId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub tx_pointer: TxPointer,
}

impl<Specification> DataCoin<Specification>
where
    Specification: CoinSpecification,
{
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcoin>.
    pub fn prepare_sign(&mut self) {
        self.tx_pointer = Default::default();
        if let Some(predicate_gas_used_field) = self.predicate_gas_used.as_mut_field() {
            *predicate_gas_used_field = Default::default();
        }
    }
}

impl DataCoin<Full> {
    pub fn into_signed(self) -> DataCoin<Signed> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            data,
            ..
        } = self;

        DataCoin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            data,
            ..Default::default()
        }
    }

    pub fn into_predicate(self) -> DataCoin<Predicate> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate,
            predicate_data,
            predicate_gas_used,
            data,
            ..
        } = self;

        DataCoin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate,
            predicate_data,
            predicate_gas_used,
            data,
            ..Default::default()
        }
    }
}
