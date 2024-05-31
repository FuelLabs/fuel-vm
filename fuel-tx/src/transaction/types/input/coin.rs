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
use derivative::Derivative;
use fuel_types::{
    Address,
    AssetId,
    Word,
};

pub type CoinFull = Coin<Full>;
pub type CoinSigned = Coin<Signed>;
pub type CoinPredicate = Coin<Predicate>;

mod private {
    pub trait Seal {}

    impl Seal for super::Full {}
    impl Seal for super::Signed {}
    impl Seal for super::Predicate {}
}

/// Specifies the coin based on the usage context. See [`Coin`].
pub trait CoinSpecification: private::Seal {
    type Witness: AsField<u16>;
    type Predicate: AsField<Vec<u8>>;
    type PredicateData: AsField<Vec<u8>>;
    type PredicateGasUsed: AsField<Word>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signed;

impl CoinSpecification for Signed {
    type Predicate = Empty<Vec<u8>>;
    type PredicateData = Empty<Vec<u8>>;
    type PredicateGasUsed = Empty<Word>;
    type Witness = u16;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Predicate;

impl CoinSpecification for Predicate {
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
    type PredicateGasUsed = Word;
    type Witness = Empty<u16>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Full;

impl CoinSpecification for Full {
    type Predicate = Vec<u8>;
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
#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct Coin<Specification>
where
    Specification: CoinSpecification,
{
    pub utxo_id: UtxoId,
    pub owner: Address,
    pub amount: Word,
    pub asset_id: AssetId,
    pub tx_pointer: TxPointer,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub witness_index: Specification::Witness,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub predicate_gas_used: Specification::PredicateGasUsed,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub predicate: Specification::Predicate,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub predicate_data: Specification::PredicateData,
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
