use crate::transaction::types::input::consts::INPUT_COIN_FIXED_SIZE;
use crate::transaction::types::input::AsField;
use crate::{TxPointer, UtxoId};
use fuel_types::bytes::Deserializable;
use fuel_types::bytes::{SizedBytes, WORD_SIZE};
use fuel_types::{bytes, Address, AssetId, Word};

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
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signed;

impl CoinSpecification for Signed {
    type Witness = u8;
    type Predicate = Empty;
    type PredicateData = Empty;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Predicate;

impl CoinSpecification for Predicate {
    type Witness = Empty;
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Full;

impl CoinSpecification for Full {
    type Witness = u8;
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
}

/// It is a full representation of the coin from the specification:
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputcoin.
///
/// The specification defines the layout of the [`Coin`] in the serialized form for the `fuel-vm`.
/// But on the business logic level, we don't use all fields at the same time. It is why in the
/// [`super::Input`] the coin is represented by several forms based on the usage context.
/// Leaving some fields empty reduces the memory consumption by the structure and erases the
/// empty useless fields.
///
/// The [`CoinSpecification`] trait specifies the sub-coin for the corresponding usage context.
/// It allows us to write the common logic of all sub-coins without the overhead and duplication.
///
/// Sub-coin:
/// - [`Signed`] - means that the coin should be signed by the `owner`,
///     and the signature(witness) should be stored under the `witness_index` index
///     in the `witnesses` vector of the [`crate::Transaction`].
/// - [`Predicate`] - means that the coin is not signed, and the `owner` is a `predicate` bytecode.
///     The merkle root from the `predicate` should be equal to the `owner`.
/// - [`Full`] - is used during the deserialization of the coin.
///     It should be transformed into [`Signed`] or [`Predicate`] sub-coin.
///     If the `predicate` is empty, it is [`Signed`], else [`Predicate`].
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Coin<Specification>
where
    Specification: CoinSpecification,
{
    pub utxo_id: UtxoId,
    pub owner: Address,
    pub amount: Word,
    pub asset_id: AssetId,
    pub tx_pointer: TxPointer,
    pub witness_index: Specification::Witness,
    pub maturity: Word,
    pub predicate: Specification::Predicate,
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
        let predicate_date_size = if let Some(predicate_data) = self.predicate_data.as_field() {
            bytes::padded_len(predicate_data.as_slice())
        } else {
            0
        };

        INPUT_COIN_FIXED_SIZE - WORD_SIZE + predicate_size + predicate_date_size
    }
}

#[cfg(feature = "std")]
impl<Specification> std::io::Read for Coin<Specification>
where
    Specification: CoinSpecification,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let serialized_size = self.serialized_size();
        if buf.len() < serialized_size {
            return Err(bytes::eof());
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
        } = self;

        let n = utxo_id.read(buf)?;
        let buf = &mut buf[n..];

        let buf = bytes::store_array_unchecked(buf, owner);
        let buf = bytes::store_number_unchecked(buf, *amount);
        let buf = bytes::store_array_unchecked(buf, asset_id);

        let n = tx_pointer.read(buf)?;
        let buf = &mut buf[n..];

        let witness_index = if let Some(witness_index) = witness_index.as_field() {
            *witness_index as Word
        } else {
            0 as Word
        };

        let buf = bytes::store_number_unchecked(buf, witness_index);
        let buf = bytes::store_number_unchecked(buf, *maturity);

        let buf = if let Some(predicate) = predicate.as_field() {
            bytes::store_number_unchecked(buf, predicate.len() as Word)
        } else {
            bytes::store_number_unchecked(buf, 0u64)
        };

        let buf = if let Some(predicate_data) = predicate_data.as_field() {
            bytes::store_number_unchecked(buf, predicate_data.len() as Word)
        } else {
            bytes::store_number_unchecked(buf, 0u64)
        };

        let buf = if let Some(predicate) = predicate.as_field() {
            let (_, buf) = bytes::store_raw_bytes(buf, predicate.as_slice())?;
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
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut n = INPUT_COIN_FIXED_SIZE - WORD_SIZE;

        if buf.len() < n {
            return Err(bytes::eof());
        }

        let utxo_id = UtxoId::from_bytes(buf)?;
        let buf = &buf[utxo_id.serialized_size()..];
        self.utxo_id = utxo_id;

        // Safety: buf len is checked
        let (owner, buf) = unsafe { bytes::restore_array_unchecked(buf) };
        let owner = owner.into();
        self.owner = owner;

        let (amount, buf) = unsafe { bytes::restore_number_unchecked(buf) };
        self.amount = amount;

        let (asset_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
        let asset_id = asset_id.into();
        self.asset_id = asset_id;

        let tx_pointer = TxPointer::from_bytes(buf)?;
        let buf = &buf[tx_pointer.serialized_size()..];
        self.tx_pointer = tx_pointer;

        let (witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
        if let Some(witness_index_field) = self.witness_index.as_mut_field() {
            *witness_index_field = witness_index;
        }

        let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };
        self.maturity = maturity;

        let (predicate_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
        let (predicate_data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };

        let (size, predicate, buf) = bytes::restore_raw_bytes(buf, predicate_len)?;
        n += size;
        if let Some(predicate_field) = self.predicate.as_mut_field() {
            *predicate_field = predicate;
        }

        let (size, predicate_data, _) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
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
        }
    }
}
