use crate::input::sizes::MessageSizes;
use crate::transaction::types::input::AsField;
use fuel_types::bytes::SizedBytes;
use fuel_types::{bytes, Address, MemLayout, MemLocType, MessageId, Word};

pub type FullMessage = Message<specifications::Full>;
pub type MessageDataSigned = Message<specifications::MessageData<specifications::Signed>>;
pub type MessageDataPredicate = Message<specifications::MessageData<specifications::Predicate>>;
pub type MessageCoinSigned = Message<specifications::MessageCoin<specifications::Signed>>;
pub type MessageCoinPredicate = Message<specifications::MessageCoin<specifications::Predicate>>;

type Empty = ();

mod private {
    pub trait Seal {}

    impl Seal for super::specifications::Full {}
    impl Seal for super::specifications::MessageData<super::specifications::Signed> {}
    impl Seal for super::specifications::MessageData<super::specifications::Predicate> {}
    impl Seal for super::specifications::MessageCoin<super::specifications::Signed> {}
    impl Seal for super::specifications::MessageCoin<super::specifications::Predicate> {}
}

/// Specifies the message based on the usage context. See [`Message`].
pub trait MessageSpecification: private::Seal {
    type Witness: AsField<u8>;
    type Data: AsField<Vec<u8>>;
    type Predicate: AsField<Vec<u8>>;
    type PredicateData: AsField<Vec<u8>>;
}

pub mod specifications {
    use super::{Empty, MessageSpecification};

    /// The type means that the message should be signed by the `recipient`, and the
    /// signature(witness) should be stored under the `witness_index` index in the `witnesses`
    /// vector of the [`crate::Transaction`].
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Signed;

    /// The type means that the message is not signed, and the `owner` is a `predicate` bytecode.
    /// The merkle root from the `predicate` should be equal to the `owner`.
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Predicate;

    /// The retrayable message metadata. It is a message that can't be used as a coin to pay for
    /// fees but can be used to pass metadata to the contract. It may have a non-zero `value`
    /// that will be transferred to the contract as a native asset during the execution.
    /// If the execution of the transaction fails, the metadata is not consumed and can be
    /// used later until successful execution.
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MessageData<UsageRules>(core::marker::PhantomData<UsageRules>);

    impl MessageSpecification for MessageData<Signed> {
        type Witness = u8;
        type Data = Vec<u8>;
        type Predicate = Empty;
        type PredicateData = Empty;
    }

    impl MessageSpecification for MessageData<Predicate> {
        type Witness = Empty;
        type Data = Vec<u8>;
        type Predicate = Vec<u8>;
        type PredicateData = Vec<u8>;
    }

    /// The spendable message acts as a standard coin.
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MessageCoin<UsageRules>(core::marker::PhantomData<UsageRules>);

    impl MessageSpecification for MessageCoin<Signed> {
        type Witness = u8;
        type Data = Empty;
        type Predicate = Empty;
        type PredicateData = Empty;
    }

    impl MessageSpecification for MessageCoin<Predicate> {
        type Witness = Empty;
        type Data = Empty;
        type Predicate = Vec<u8>;
        type PredicateData = Vec<u8>;
    }

    /// The type is used to represent the full message. It is used during the deserialization of
    /// the message to determine the final type.
    /// If the `data` field is empty, it should be transformed into [`MessageData`]. Otherwise
    /// into [`MessageCoin`].
    /// If the `predicate` is empty, the usage rules should be [`Signed`], else [`Predicate`].
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Full;

    impl MessageSpecification for Full {
        type Witness = u8;
        type Data = Vec<u8>;
        type Predicate = Vec<u8>;
        type PredicateData = Vec<u8>;
    }
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
/// Sub-messages from [`specifications`]:
/// - [`specifications::MessageData`] with [`specifications::Signed`] usage rules.
/// - [`specifications::MessageData`] with [`specifications::Predicate`] usage rules.
/// - [`specifications::Full`].
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Message<Specification>
where
    Specification: MessageSpecification,
{
    /// The sender from the L1 chain.
    pub sender: Address,
    /// The receiver on the `Fuel` chain.
    pub recipient: Address,
    pub amount: Word,
    pub nonce: Word,
    pub witness_index: Specification::Witness,
    pub data: Specification::Data,
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

    pub fn message_id(&self) -> MessageId {
        let Self {
            sender,
            recipient,
            amount,
            nonce,
            data,
            ..
        } = self;
        if let Some(data) = data.as_field() {
            compute_message_id(sender, recipient, *nonce, *amount, data)
        } else {
            compute_message_id(sender, recipient, *nonce, *amount, &[])
        }
    }
}

impl<Specification> SizedBytes for Message<Specification>
where
    Specification: MessageSpecification,
{
    #[inline(always)]
    fn serialized_size(&self) -> usize {
        let data_size = if let Some(data) = self.data.as_field() {
            bytes::padded_len(data.as_slice())
        } else {
            0
        };
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

        MessageSizes::LEN + data_size + predicate_size + predicate_date_size
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

        let data_size = if let Some(data) = data.as_field() {
            data.len()
        } else {
            0
        };
        bytes::store_number_at(buf, S::layout(S::LAYOUT.data_len), data_size as Word);

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

        let buf = full_buf.get_mut(LEN..).ok_or(bytes::eof())?;
        let buf = if let Some(data) = data.as_field() {
            let (_, buf) = bytes::store_raw_bytes(buf, data.as_slice())?;
            buf
        } else {
            buf
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
    fn write(&mut self, full_buf: &[u8]) -> std::io::Result<usize> {
        type S = MessageSizes;
        const LEN: usize = MessageSizes::LEN;
        let buf: &[_; LEN] = full_buf
            .get(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;
        let mut n = LEN;

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
        if let Some(data_field) = self.data.as_mut_field() {
            *data_field = data;
        }

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

impl FullMessage {
    pub fn into_message_data_signed(self) -> MessageDataSigned {
        let Self {
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
            ..
        } = self;

        Message {
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

    pub fn into_message_data_predicate(self) -> MessageDataPredicate {
        let Self {
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

    pub fn into_coin_signed(self) -> MessageCoinSigned {
        let Self {
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            ..
        } = self;

        Message {
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data: (),
            predicate: (),
            predicate_data: (),
        }
    }

    pub fn into_coin_predicate(self) -> MessageCoinPredicate {
        let Self {
            sender,
            recipient,
            amount,
            nonce,
            predicate,
            predicate_data,
            ..
        } = self;

        Message {
            sender,
            recipient,
            amount,
            nonce,
            witness_index: (),
            data: (),
            predicate,
            predicate_data,
        }
    }
}

pub fn compute_message_id(sender: &Address, recipient: &Address, nonce: Word, amount: Word, data: &[u8]) -> MessageId {
    let hasher = fuel_crypto::Hasher::default()
        .chain(sender)
        .chain(recipient)
        .chain(nonce.to_be_bytes())
        .chain(amount.to_be_bytes())
        .chain(data);

    (*hasher.finalize()).into()
}
