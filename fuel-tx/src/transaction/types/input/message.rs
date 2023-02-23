use crate::transaction::types::input::consts::INPUT_COIN_FIXED_SIZE;
use crate::transaction::types::input::AsField;
use crate::{TxPointer, UtxoId};
use fuel_types::bytes::Deserializable;
use fuel_types::bytes::{SizedBytes, WORD_SIZE};
use fuel_types::{bytes, Address, AssetId, Word};

pub type MessageFull = Message<Full>;
pub type MessageSigned = Message<Signed>;
pub type MessagePredicate = Message<Predicate>;

type Empty = ();

mod private {
    pub trait Seal {}

    impl Seal for super::Full {}
    impl Seal for super::Signed {}
    impl Seal for super::Predicate {}
}

pub trait MessageSpecification: private::Seal {
    type Witness: AsField<u8>;
    type Predicate: AsField<Vec<u8>>;
    type PredicateData: AsField<Vec<u8>>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signed;

impl MessageSpecification for Signed {
    type Witness = u8;
    type Predicate = Empty;
    type PredicateData = Empty;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Predicate;

impl MessageSpecification for Predicate {
    type Witness = Empty;
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Full;

impl MessageSpecification for Full {
    type Witness = u8;
    type Predicate = Vec<u8>;
    type PredicateData = Vec<u8>;
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Message<Specification>
where
    Specification: MessageSpecification,
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

impl<Specification> SizedBytes for Message<Specification>
where
    Specification: MessageSpecification,
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
impl<Specification> std::io::Read for Message<Specification>
where
    Specification: MessageSpecification,
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
impl<Specification> std::io::Write for Message<Specification>
where
    Specification: MessageSpecification,
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

impl Message<Full> {
    pub fn into_signed(self) -> Message<Signed> {
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

        Message {
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

    pub fn into_predicate(self) -> Message<Predicate> {
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

        Message {
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
