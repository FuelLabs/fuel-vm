use crate::{
    input::fmt_as_field,
    transaction::types::input::AsField,
};
use alloc::vec::Vec;
use derivative::Derivative;
use fuel_types::{
    Address,
    MessageId,
    Nonce,
    Word,
};

pub type FullMessage = Message<specifications::Full>;
pub type MessageDataSigned = Message<specifications::MessageData<specifications::Signed>>;
pub type MessageDataPredicate =
    Message<specifications::MessageData<specifications::Predicate>>;
pub type MessageCoinSigned = Message<specifications::MessageCoin<specifications::Signed>>;
pub type MessageCoinPredicate =
    Message<specifications::MessageCoin<specifications::Predicate>>;

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
    type Data: AsField<Vec<u8>>;
    type Predicate: AsField<Vec<u8>>;
    type PredicateData: AsField<Vec<u8>>;
    type PredicateGasUsed: AsField<Word>;
    type Witness: AsField<u8>;
}

pub mod specifications {
    use alloc::vec::Vec;

    use super::MessageSpecification;
    use crate::input::Empty;
    use fuel_types::Word;

    /// The type means that the message should be signed by the `recipient`, and the
    /// signature(witness) should be stored under the `witness_index` index in the
    /// `witnesses` vector of the [`crate::Transaction`].
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Signed;

    /// The type means that the message is not signed, and the `owner` is a `predicate`
    /// bytecode. The merkle root from the `predicate` should be equal to the `owner`.
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Predicate;

    /// The retrayable message metadata. It is a message that can't be used as a coin to
    /// pay for fees but can be used to pass metadata to the contract. It may have a
    /// non-zero `value` that will be transferred to the contract as a native asset
    /// during the execution. If the execution of the transaction fails, the metadata
    /// is not consumed and can be used later until successful execution.
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MessageData<UsageRules>(core::marker::PhantomData<UsageRules>);

    impl MessageSpecification for MessageData<Signed> {
        type Data = Vec<u8>;
        type Predicate = Empty<Vec<u8>>;
        type PredicateData = Empty<Vec<u8>>;
        type PredicateGasUsed = Empty<Word>;
        type Witness = u8;
    }

    impl MessageSpecification for MessageData<Predicate> {
        type Data = Vec<u8>;
        type Predicate = Vec<u8>;
        type PredicateData = Vec<u8>;
        type PredicateGasUsed = Word;
        type Witness = Empty<u8>;
    }

    /// The spendable message acts as a standard coin.
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MessageCoin<UsageRules>(core::marker::PhantomData<UsageRules>);

    impl MessageSpecification for MessageCoin<Signed> {
        type Data = Empty<Vec<u8>>;
        type Predicate = Empty<Vec<u8>>;
        type PredicateData = Empty<Vec<u8>>;
        type PredicateGasUsed = Empty<Word>;
        type Witness = u8;
    }

    impl MessageSpecification for MessageCoin<Predicate> {
        type Data = Empty<Vec<u8>>;
        type Predicate = Vec<u8>;
        type PredicateData = Vec<u8>;
        type PredicateGasUsed = Word;
        type Witness = Empty<u8>;
    }

    /// The type is used to represent the full message. It is used during the
    /// deserialization of the message to determine the final type.
    /// If the `data` field is empty, it should be transformed into [`MessageData`].
    /// Otherwise into [`MessageCoin`].
    /// If the `predicate` is empty, the usage rules should be [`Signed`], else
    /// [`Predicate`].
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Full;

    impl MessageSpecification for Full {
        type Data = Vec<u8>;
        type Predicate = Vec<u8>;
        type PredicateData = Vec<u8>;
        type PredicateGasUsed = Word;
        type Witness = u8;
    }
}

/// It is a full representation of the message from the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputmessage>.
///
/// The specification defines the layout of the [`Message`] in the serialized form for
/// the `fuel-vm`. But on the business logic level, we don't use all fields at the same
/// time. It is why in the [`super::Input`] the message is represented by several forms
/// based on the usage context. Leaving some fields empty reduces the memory consumption
/// by the structure and erases the empty useless fields.
///
/// The [`MessageSpecification`] trait specifies the sub-messages for the corresponding
/// usage context. It allows us to write the common logic of all sub-messages without the
/// overhead and duplication.
///
/// Sub-messages from [`specifications`]:
/// - [`specifications::MessageData`] with [`specifications::Signed`] usage rules.
/// - [`specifications::MessageData`] with [`specifications::Predicate`] usage rules.
/// - [`specifications::Full`].
#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct Message<Specification>
where
    Specification: MessageSpecification,
{
    /// The sender from the L1 chain.
    pub sender: Address,
    /// The receiver on the `Fuel` chain.
    pub recipient: Address,
    pub amount: Word,
    pub nonce: Nonce,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub witness_index: Specification::Witness,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub predicate_gas_used: Specification::PredicateGasUsed,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub data: Specification::Data,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub predicate: Specification::Predicate,
    #[derivative(Debug(format_with = "fmt_as_field"))]
    pub predicate_data: Specification::PredicateData,
}

impl<Specification> Message<Specification>
where
    Specification: MessageSpecification,
{
    pub fn prepare_sign(&mut self) {
        if let Some(predicate_gas_used_field) = self.predicate_gas_used.as_mut_field() {
            *predicate_gas_used_field = Default::default();
        }
    }

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
            compute_message_id(sender, recipient, nonce, *amount, data)
        } else {
            compute_message_id(sender, recipient, nonce, *amount, &[])
        }
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
            ..Default::default()
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
            predicate_gas_used,
            ..
        } = self;

        Message {
            sender,
            recipient,
            amount,
            nonce,
            data,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..Default::default()
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
            ..Default::default()
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
            predicate_gas_used,
            ..
        } = self;

        Message {
            sender,
            recipient,
            amount,
            nonce,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..Default::default()
        }
    }
}

impl MessageCoinSigned {
    pub fn into_full(self) -> FullMessage {
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
            ..Default::default()
        }
    }
}

impl MessageCoinPredicate {
    pub fn into_full(self) -> FullMessage {
        let Self {
            sender,
            recipient,
            amount,
            nonce,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..
        } = self;

        Message {
            sender,
            recipient,
            amount,
            nonce,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..Default::default()
        }
    }
}

impl MessageDataPredicate {
    pub fn into_full(self) -> FullMessage {
        let Self {
            sender,
            recipient,
            amount,
            nonce,
            data,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..
        } = self;

        Message {
            sender,
            recipient,
            amount,
            nonce,
            data,
            predicate,
            predicate_data,
            predicate_gas_used,
            ..Default::default()
        }
    }
}

impl MessageDataSigned {
    pub fn into_full(self) -> FullMessage {
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
            ..Default::default()
        }
    }
}

pub fn compute_message_id(
    sender: &Address,
    recipient: &Address,
    nonce: &Nonce,
    amount: Word,
    data: &[u8],
) -> MessageId {
    let hasher = fuel_crypto::Hasher::default()
        .chain(sender)
        .chain(recipient)
        .chain(nonce)
        .chain(amount.to_be_bytes())
        .chain(data);

    (*hasher.finalize()).into()
}
