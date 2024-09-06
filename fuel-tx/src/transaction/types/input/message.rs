use alloc::vec::Vec;
use derivative::Derivative;
use fuel_types::{
    Address,
    MessageId,
    Nonce,
    Word,
};

use super::predicate::Predicate;

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
pub struct MessageCommon {
    /// The sender from the L1 chain.
    pub sender: Address,
    /// The receiver on the `Fuel` chain.
    pub recipient: Address,
    pub amount: Word,
    pub nonce: Nonce,
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct MessageDataSigned {
    pub common: MessageCommon,
    pub witness_index: u16,
    pub data: Vec<u8>,
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct MessageDataPredicate {
    pub common: MessageCommon,
    pub data: Vec<u8>,
    #[serde(flatten)]
    pub predicate: Predicate,
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct MessageCoinSigned {
    pub common: MessageCommon,
    pub witness_index: u16,
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct MessageCoinPredicate {
    #[serde(flatten)]
    pub common: MessageCommon,
    #[serde(flatten)]
    pub predicate: Predicate,
}

impl MessageCommon {
    pub fn message_id_for_data(&self, data: &[u8]) -> MessageId {
        compute_message_id(
            &self.sender,
            &self.recipient,
            &self.nonce,
            self.amount,
            data,
        )
    }
}

impl MessageDataSigned {
    pub fn prepare_sign(&mut self) {}

    pub fn message_id(&self) -> MessageId {
        self.common.message_id_for_data(&self.data)
    }
}

impl MessageDataPredicate {
    pub fn prepare_sign(&mut self) {
        self.predicate.prepare_sign();
    }

    pub fn message_id(&self) -> MessageId {
        self.common.message_id_for_data(&self.data)
    }
}

impl MessageCoinSigned {
    pub fn prepare_sign(&mut self) {}

    pub fn message_id(&self) -> MessageId {
        self.common.message_id_for_data(&[])
    }
}

impl MessageCoinPredicate {
    pub fn prepare_sign(&mut self) {
        self.predicate.prepare_sign();
    }

    pub fn message_id(&self) -> MessageId {
        self.common.message_id_for_data(&[])
    }
}

#[derive(Default, Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct MessageFull {
    #[serde(flatten)]
    pub common: MessageCommon,
    pub witness_index: u16,
    pub data: Vec<u8>,
    #[serde(flatten)]
    pub predicate: Predicate,
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
