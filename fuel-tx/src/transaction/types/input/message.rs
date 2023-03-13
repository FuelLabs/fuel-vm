use crate::input::sizes::MessageSizes;
use crate::transaction::types::input::AsField;
use fuel_types::bytes::SizedBytes;
use fuel_types::{bytes, Address, MemLayout, MemLocType, MessageId, Word};

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

/// Specifies the message based on the usage context. See [`Message`].
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

/// It is a full representation of the message from the specification:
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputmessage.
///
/// The specification defines the layout of the [`Message`] in the serialized form for
/// the `fuel-vm`. But on the business logic level, we don't use all fields at the same time.
/// It is why in the [`super::Input`] the message is represented by several forms based on
/// the usage context. Leaving some fields empty reduces the memory consumption by the
/// structure and erases the empty useless fields.
///
/// The [`MessageSpecification`] trait specifies the sub-messages for the corresponding
/// usage context. It allows us to write the common logic of all sub-messages without the overhead
/// and duplication.
///
/// Sub-messages:
/// - [`Signed`] - means that the message should be signed by the `recipient`,
///     and the signature(witness) should be stored under the `witness_index` index
///     in the `witnesses` vector of the [`crate::Transaction`].
/// - [`Predicate`] - means that the message is not signed, and the `owner` is
///     a `predicate` bytecode. The merkle root from the `predicate` should be equal to the `owner`.
/// - [`Full`] - is used during the deserialization of the message.
///     It should be transformed into [`Signed`] or [`Predicate`] sub-message.
///     If the `predicate` is empty, it is [`Signed`], else [`Predicate`].
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Message<Specification>
where
    Specification: MessageSpecification,
{
    pub message_id: MessageId,
    /// The sender from the L1 chain.
    pub sender: Address,
    /// The receiver on the `Fuel` chain.
    pub recipient: Address,
    pub amount: Word,
    pub nonce: Word,
    pub witness_index: Specification::Witness,
    pub data: Vec<u8>,
    pub predicate: Specification::Predicate,
    pub predicate_data: Specification::PredicateData,
}

impl<Specification> Message<Specification>
where
    Specification: MessageSpecification,
{
    /// It is empty, because specification says nothing:
    /// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputmessage
    pub fn prepare_sign(&mut self) {}
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

        MessageSizes::LEN + bytes::padded_len(self.data.as_slice()) + predicate_size + predicate_date_size
    }
}

#[cfg(feature = "std")]
impl<Specification> std::io::Read for Message<Specification>
where
    Specification: MessageSpecification,
{
    fn read(&mut self, full_buf: &mut [u8]) -> std::io::Result<usize> {
        let serialized_size = self.serialized_size();
        if full_buf.len() < serialized_size {
            return Err(bytes::eof());
        }

        let Self {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
            predicate,
            predicate_data,
        } = self;
        type S = MessageSizes;
        const LEN: usize = MessageSizes::LEN;
        let buf: &mut [_; LEN] = full_buf
            .get_mut(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        bytes::store_at(buf, S::layout(S::LAYOUT.message_id), message_id);
        bytes::store_at(buf, S::layout(S::LAYOUT.sender), sender);
        bytes::store_at(buf, S::layout(S::LAYOUT.recipient), recipient);

        bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
        bytes::store_number_at(buf, S::layout(S::LAYOUT.nonce), *nonce);

        let witness_index = if let Some(witness_index) = witness_index.as_field() {
            *witness_index
        } else {
            0
        };
        bytes::store_number_at(buf, S::layout(S::LAYOUT.witness_index), witness_index);

        bytes::store_number_at(buf, S::layout(S::LAYOUT.data_len), data.len() as Word);

        let predicate_len = if let Some(predicate) = predicate.as_field() {
            predicate.len()
        } else {
            0
        };
        bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_len), predicate_len as Word);

        let predicate_data_len = if let Some(predicate_data) = predicate_data.as_field() {
            predicate_data.len()
        } else {
            0
        };
        bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_data_len), predicate_data_len as Word);

        let (_, buf) = bytes::store_raw_bytes(full_buf.get_mut(LEN..).ok_or(bytes::eof())?, data.as_slice())?;

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
    fn write(&mut self, full_buf: &[u8]) -> std::io::Result<usize> {
        type S = MessageSizes;
        const LEN: usize = MessageSizes::LEN;
        let buf: &[_; LEN] = full_buf
            .get(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;
        let mut n = LEN;

        let message_id = bytes::restore_at(buf, S::layout(S::LAYOUT.message_id));
        self.message_id = message_id.into();
        let sender = bytes::restore_at(buf, S::layout(S::LAYOUT.sender));
        self.sender = sender.into();
        let recipient = bytes::restore_at(buf, S::layout(S::LAYOUT.recipient));
        self.recipient = recipient.into();

        let amount = bytes::restore_number_at(buf, S::layout(S::LAYOUT.amount));
        self.amount = amount;
        let nonce = bytes::restore_number_at(buf, S::layout(S::LAYOUT.nonce));
        self.nonce = nonce;
        let witness_index = bytes::restore_u8_at(buf, S::layout(S::LAYOUT.witness_index));
        if let Some(witness_index_field) = self.witness_index.as_mut_field() {
            *witness_index_field = witness_index;
        }

        let data_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.data_len));
        let predicate_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_len));
        let predicate_data_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_data_len));

        let (size, data, buf) = bytes::restore_raw_bytes(full_buf.get(LEN..).ok_or(bytes::eof())?, data_len)?;
        n += size;
        self.data = data;

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
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
            ..
        } = self;

        Message {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
            predicate: (),
            predicate_data: (),
        }
    }

    pub fn into_predicate(self) -> Message<Predicate> {
        let Self {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            data,
            predicate,
            predicate_data,
            ..
        } = self;

        Message {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index: (),
            data,
            predicate,
            predicate_data,
        }
    }
}
