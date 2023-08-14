use crate::{
    input::{
        fmt_as_field,
        sizes::CoinSizes,
    },
    transaction::types::input::AsField,
    TxPointer,
    UtxoId,
};
use derivative::Derivative;
use fuel_types::{
    bytes,
    bytes::SizedBytes,
    Address,
    AssetId,
    BlockHeight,
    MemLayout,
    MemLocType,
    Word,
};

#[cfg(feature = "std")]
use fuel_types::bytes::Deserializable;

use alloc::vec::Vec;

pub type CoinFull = Coin<Full>;
pub type CoinSigned = Coin<Signed>;
pub type CoinPredicate = Coin<Predicate>;

type Empty = ();

mod private {
    pub trait Seal {}

    impl Seal for super::Full {}
    impl Seal for super::Signed {}
    impl Seal for super::Predicate {}
}

/// Specifies the coin based on the usage context. See [`Coin`].
pub trait CoinSpecification: private::Seal {
    type Witness: AsField<u8>;
    type Predicate: AsField<Vec<u8>>;
    type PredicateData: AsField<Vec<u8>>;
    type PredicateGasUsed: AsField<Word>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signed;

impl CoinSpecification for Signed {
    type Predicate = Empty;
    type PredicateData = Empty;
    type PredicateGasUsed = Empty;
    type Witness = u8;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Predicate;

impl CoinSpecification for Predicate {
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
    type PredicateGasUsed = Word;
    type Witness = Empty;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Full;

impl CoinSpecification for Full {
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
    type PredicateGasUsed = Word;
    type Witness = u8;
}

/// It is a full representation of the coin from the specification:
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputcoin.
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
    pub maturity: BlockHeight,
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
    /// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputcoin.
    pub fn prepare_sign(&mut self) {
        core::mem::take(&mut self.tx_pointer);
        if let Some(predicate_gas_used_field) = self.predicate_gas_used.as_mut_field() {
            core::mem::take(predicate_gas_used_field);
        }
    }
}

impl<Specification> SizedBytes for Coin<Specification>
where
    Specification: CoinSpecification,
{
    #[inline(always)]
    fn serialized_size(&self) -> usize {
        let predicate_size = if let Some(predicate) = self.predicate.as_field() {
            bytes::padded_len(predicate.as_slice())
        } else {
            0
        };
        let predicate_date_size =
            if let Some(predicate_data) = self.predicate_data.as_field() {
                bytes::padded_len(predicate_data.as_slice())
            } else {
                0
            };

        CoinSizes::LEN + predicate_size + predicate_date_size
    }
}

#[cfg(feature = "std")]
impl<Specification> std::io::Read for Coin<Specification>
where
    Specification: CoinSpecification,
{
    fn read(&mut self, full_buf: &mut [u8]) -> std::io::Result<usize> {
        let serialized_size = self.serialized_size();
        if full_buf.len() < serialized_size {
            return Err(bytes::eof())
        }

        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            maturity,
            predicate,
            predicate_data,
            predicate_gas_used,
        } = self;

        type S = CoinSizes;
        const LEN: usize = CoinSizes::LEN;
        let buf: &mut [_; LEN] = full_buf
            .get_mut(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let n = utxo_id.read(&mut buf[S::LAYOUT.utxo_id.range()])?;
        if n != S::LAYOUT.utxo_id.size() {
            return Err(bytes::eof())
        }

        bytes::store_at(buf, S::layout(S::LAYOUT.owner), owner);
        bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
        bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);

        let n = tx_pointer.read(&mut buf[S::LAYOUT.tx_pointer.range()])?;
        if n != S::LAYOUT.tx_pointer.size() {
            return Err(bytes::eof())
        }

        let witness_index = if let Some(witness_index) = witness_index.as_field() {
            *witness_index
        } else {
            // Witness index zeroed for coin predicate
            0
        };

        let predicate_gas_used =
            if let Some(predicate_gas_used) = predicate_gas_used.as_field() {
                *predicate_gas_used
            } else {
                // predicate gas used zeroed for coin predicate
                0
            };

        bytes::store_number_at(buf, S::layout(S::LAYOUT.witness_index), witness_index);
        bytes::store_number_at(buf, S::layout(S::LAYOUT.maturity), **maturity);
        bytes::store_number_at(
            buf,
            S::layout(S::LAYOUT.predicate_gas_used),
            predicate_gas_used as Word,
        );

        let predicate_len = if let Some(predicate) = predicate.as_field() {
            predicate.len()
        } else {
            0
        };

        let predicate_data_len = if let Some(predicate_data) = predicate_data.as_field() {
            predicate_data.len()
        } else {
            0
        };

        bytes::store_number_at(
            buf,
            S::layout(S::LAYOUT.predicate_len),
            predicate_len as Word,
        );

        bytes::store_number_at(
            buf,
            S::layout(S::LAYOUT.predicate_data_len),
            predicate_data_len as Word,
        );

        let buf = if let Some(predicate) = predicate.as_field() {
            let (_, buf) = bytes::store_raw_bytes(
                full_buf.get_mut(LEN..).ok_or(bytes::eof())?,
                predicate.as_slice(),
            )?;
            buf
        } else {
            buf
        };

        if let Some(predicate_data) = predicate_data.as_field() {
            bytes::store_raw_bytes(buf, predicate_data.as_slice())?;
        };

        Ok(serialized_size)
    }
}

#[cfg(feature = "std")]
impl<Specification> std::io::Write for Coin<Specification>
where
    Specification: CoinSpecification,
{
    fn write(&mut self, full_buf: &[u8]) -> std::io::Result<usize> {
        type S = CoinSizes;
        const LEN: usize = CoinSizes::LEN;
        let buf: &[_; LEN] = full_buf
            .get(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let mut n = LEN;

        let utxo_id = UtxoId::from_bytes(&buf[S::LAYOUT.utxo_id.range()])?;
        self.utxo_id = utxo_id;

        let owner = bytes::restore_at(buf, S::layout(S::LAYOUT.owner));
        let owner = owner.into();
        self.owner = owner;

        let amount = bytes::restore_number_at(buf, S::layout(S::LAYOUT.amount));
        self.amount = amount;

        let asset_id = bytes::restore_at(buf, S::layout(S::LAYOUT.asset_id));
        let asset_id = asset_id.into();
        self.asset_id = asset_id;

        let tx_pointer = TxPointer::from_bytes(&buf[S::LAYOUT.tx_pointer.range()])?;
        self.tx_pointer = tx_pointer;

        let witness_index = bytes::restore_u8_at(buf, S::layout(S::LAYOUT.witness_index));
        if let Some(witness_index_field) = self.witness_index.as_mut_field() {
            *witness_index_field = witness_index;
        }
        let maturity = bytes::restore_u32_at(buf, S::layout(S::LAYOUT.maturity)).into();
        self.maturity = maturity;

        let predicate_gas_used =
            bytes::restore_number_at(buf, S::layout(S::LAYOUT.predicate_gas_used));
        if let Some(predicate_gas_used_field) = self.predicate_gas_used.as_mut_field() {
            *predicate_gas_used_field = predicate_gas_used;
        }

        let predicate_len =
            bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_len));
        let predicate_data_len =
            bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_data_len));

        let (size, predicate, buf) = bytes::restore_raw_bytes(
            full_buf.get(LEN..).ok_or(bytes::eof())?,
            predicate_len,
        )?;
        n += size;
        if let Some(predicate_field) = self.predicate.as_mut_field() {
            *predicate_field = predicate;
        }

        let (size, predicate_data, _) =
            bytes::restore_raw_bytes(buf, predicate_data_len)?;
        n += size;
        if let Some(predicate_data_field) = self.predicate_data.as_mut_field() {
            *predicate_data_field = predicate_data;
        }

        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
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
            maturity,
            ..
        } = self;

        Coin {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            maturity,
            predicate: (),
            predicate_data: (),
            predicate_gas_used: (),
        }
    }

    pub fn into_predicate(self) -> Coin<Predicate> {
        let Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            maturity,
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
            witness_index: (),
            maturity,
            predicate,
            predicate_data,
            predicate_gas_used,
        }
    }
}
