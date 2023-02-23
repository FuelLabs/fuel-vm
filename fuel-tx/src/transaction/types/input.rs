use crate::{TxPointer, UtxoId};
use alloc::vec::Vec;
use coin::*;
use consts::*;
use contract::*;
use fuel_crypto::{Hasher, PublicKey};
use fuel_types::bytes;
use fuel_types::bytes::{SizedBytes, WORD_SIZE};
use fuel_types::{bytes, MemLayout, MemLocType};
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId, Word};
use message::*;

#[cfg(feature = "std")]
use std::io;

pub mod coin;
mod consts;
pub mod contract;
pub mod message;
mod repr;
mod sizes;

pub use repr::InputRepr;

#[cfg(all(test, feature = "std"))]
mod ser_de_tests;

pub trait AsField<Type> {
    fn as_field(&self) -> Option<&Type>;

    fn as_mut_field(&mut self) -> Option<&mut Type>;
}

impl<Type> AsField<Type> for () {
    #[inline(always)]
    fn as_field(&self) -> Option<&Type> {
        None
    }

    fn as_mut_field(&mut self) -> Option<&mut Type> {
        None
    }
}

impl AsField<u8> for u8 {
    #[inline(always)]
    fn as_field(&self) -> Option<&u8> {
        Some(self)
    }

    fn as_mut_field(&mut self) -> Option<&mut u8> {
        Some(self)
    }
}

impl AsField<Vec<u8>> for Vec<u8> {
    #[inline(always)]
    fn as_field(&self) -> Option<&Vec<u8>> {
        Some(self)
    }

    fn as_mut_field(&mut self) -> Option<&mut Vec<u8>> {
        Some(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Input {
    CoinSigned(CoinSigned),
    CoinPredicate(CoinPredicate),
    Contract(Contract),
    MessageSigned(MessageSigned),
    MessagePredicate(MessagePredicate),
}

impl Default for Input {
    fn default() -> Self {
        Self::contract(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }
}

impl bytes::SizedBytes for Input {
    fn serialized_size(&self) -> usize {
        match self {
            Self::CoinSigned(coin) => WORD_SIZE + coin.serialized_size(),
            Self::CoinPredicate(coin) => WORD_SIZE + coin.serialized_size(),
            Self::Contract(contract) => WORD_SIZE + contract.serialized_size(),
            Self::MessageSigned(message) => WORD_SIZE + message.serialized_size(),
            Self::MessagePredicate(message) => WORD_SIZE + message.serialized_size(),
        }
    }
}

impl Input {
    pub const fn repr(&self) -> InputRepr {
        InputRepr::from_input(self)
    }

    pub fn owner(pk: &PublicKey) -> Address {
        let owner: [u8; Address::LEN] = pk.hash().into();

        owner.into()
    }

    pub const fn coin_predicate(
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        tx_pointer: TxPointer,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::CoinPredicate(CoinPredicate {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index: (),
            maturity,
            predicate,
            predicate_data,
        })
    }

    pub const fn coin_signed(
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        tx_pointer: TxPointer,
        witness_index: u8,
        maturity: Word,
    ) -> Self {
        Self::CoinSigned(CoinSigned {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            maturity,
            predicate: (),
            predicate_data: (),
        })
    }

    pub const fn contract(
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        tx_pointer: TxPointer,
        contract_id: ContractId,
    ) -> Self {
        Self::Contract(Contract {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        })
    }

    pub const fn message_signed(
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        witness_index: u8,
        data: Vec<u8>,
    ) -> Self {
        Self::MessageSigned(MessageSigned {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
            predicate: (),
            predicate_data: (),
        })
    }

    pub const fn message_predicate(
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        data: Vec<u8>,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::MessagePredicate(MessagePredicate {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index: (),
            data,
            predicate,
            predicate_data,
        })
    }

    pub const fn utxo_id(&self) -> Option<&UtxoId> {
        match self {
            Self::CoinSigned(CoinSigned { utxo_id, .. })
            | Self::CoinPredicate(CoinPredicate { utxo_id, .. })
            | Self::Contract(Contract { utxo_id, .. }) => Some(utxo_id),
            Self::MessageSigned { .. } => None,
            Self::MessagePredicate { .. } => None,
        }
    }

    pub const fn input_owner(&self) -> Option<&Address> {
        match self {
            Self::CoinSigned(CoinSigned { owner, .. }) | Self::CoinPredicate(CoinPredicate { owner, .. }) => {
                Some(owner)
            }
            Self::MessageSigned { .. } | Self::MessagePredicate { .. } | Self::Contract(_) => None,
        }
    }

    pub const fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Input::CoinSigned(CoinSigned { asset_id, .. }) | Input::CoinPredicate(CoinPredicate { asset_id, .. }) => {
                Some(asset_id)
            }
            Input::MessageSigned(_) | Input::MessagePredicate(_) => Some(&AssetId::BASE),
            Input::Contract { .. } => None,
        }
    }

    pub const fn contract_id(&self) -> Option<&ContractId> {
        match self {
            Self::Contract(Contract { contract_id, .. }) => Some(contract_id),
            _ => None,
        }
    }

    pub const fn amount(&self) -> Option<Word> {
        match self {
            Input::CoinSigned(CoinSigned { amount, .. })
            | Input::CoinPredicate(CoinPredicate { amount, .. })
            | Input::MessageSigned(MessageSigned { amount, .. })
            | Input::MessagePredicate(MessagePredicate { amount, .. }) => Some(*amount),
            Input::Contract { .. } => None,
        }
    }

    pub const fn witness_index(&self) -> Option<u8> {
        match self {
            Input::CoinSigned(CoinSigned { witness_index, .. })
            | Input::MessageSigned(MessageSigned { witness_index, .. }) => Some(*witness_index),
            Input::CoinPredicate(_) | Input::Contract { .. } | Input::MessagePredicate(_) => None,
        }
    }

    pub const fn maturity(&self) -> Option<Word> {
        match self {
            Input::CoinSigned(CoinSigned { maturity, .. }) | Input::CoinPredicate(CoinPredicate { maturity, .. }) => {
                Some(*maturity)
            }
            Input::Contract { .. } | Input::MessageSigned(_) | Input::MessagePredicate(_) => None,
        }
    }

    pub fn predicate_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(_) => InputRepr::Coin.coin_predicate_offset(),
            Input::MessagePredicate(MessagePredicate { data, .. }) => {
                InputRepr::Message.data_offset().map(|o| o + bytes::padded_len(data))
            }
            Input::CoinSigned(_) | Input::Contract { .. } | Input::MessageSigned(_) => None,
        }
    }

    pub fn predicate_data_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate, .. })
            | Input::MessagePredicate(MessagePredicate { predicate, .. }) => {
                self.predicate_offset().map(|o| o + bytes::padded_len(predicate))
            }
            Input::CoinSigned(_) | Input::Contract { .. } | Input::MessageSigned(_) => None,
        }
    }

    pub fn predicate_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate, .. })
            | Input::MessagePredicate(MessagePredicate { predicate, .. }) => Some(predicate.len()),
            Input::CoinSigned(_) | Input::MessageSigned(_) => Some(0),
            Input::Contract { .. } => None,
        }
    }

    pub fn predicate_data_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate_data, .. })
            | Input::MessagePredicate(MessagePredicate { predicate_data, .. }) => Some(predicate_data.len()),
            Input::CoinSigned(_) | Input::MessageSigned(_) => Some(0),
            Input::Contract { .. } => None,
        }
    }

    pub const fn message_id(&self) -> Option<&MessageId> {
        match self {
            Self::MessagePredicate(MessagePredicate { message_id, .. })
            | Self::MessageSigned(MessageSigned { message_id, .. }) => Some(message_id),
            _ => None,
        }
    }

    pub const fn tx_pointer(&self) -> Option<&TxPointer> {
        match self {
            Input::CoinSigned(CoinSigned { tx_pointer, .. })
            | Input::CoinPredicate(CoinPredicate { tx_pointer, .. })
            | Input::Contract(Contract { tx_pointer, .. }) => Some(tx_pointer),
            _ => None,
        }
    }

    pub fn input_data(&self) -> Option<&[u8]> {
        match self {
            Input::MessageSigned(MessageSigned { data, .. })
            | Input::MessagePredicate(MessagePredicate { data, .. }) => Some(data),
            _ => None,
        }
    }

    pub fn input_predicate(&self) -> Option<&[u8]> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate, .. })
            | Input::MessagePredicate(MessagePredicate { predicate, .. }) => Some(predicate),

            _ => None,
        }
    }

    pub fn input_predicate_data(&self) -> Option<&[u8]> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate_data, .. })
            | Input::MessagePredicate(MessagePredicate { predicate_data, .. }) => Some(predicate_data),

            _ => None,
        }
    }

    /// Return a tuple containing the predicate and its data if the input is of
    /// type `CoinPredicate` or `MessagePredicate`
    pub fn predicate(&self) -> Option<(&[u8], &[u8])> {
        match self {
            Input::CoinPredicate(CoinPredicate {
                predicate,
                predicate_data,
                ..
            })
            | Input::MessagePredicate(MessagePredicate {
                predicate,
                predicate_data,
                ..
            }) => Some((predicate.as_slice(), predicate_data.as_slice())),

            _ => None,
        }
    }

    pub const fn is_coin(&self) -> bool {
        self.is_coin_signed() | self.is_coin_predicate()
    }

    pub const fn is_coin_signed(&self) -> bool {
        matches!(self, Input::CoinSigned(_))
    }

    pub const fn is_coin_predicate(&self) -> bool {
        matches!(self, Input::CoinPredicate(_))
    }

    pub const fn is_message(&self) -> bool {
        self.is_message_signed() | self.is_message_predicate()
    }

    pub const fn is_message_signed(&self) -> bool {
        matches!(self, Input::MessageSigned(_))
    }

    pub const fn is_message_predicate(&self) -> bool {
        matches!(self, Input::MessagePredicate(_))
    }

    pub const fn is_contract(&self) -> bool {
        matches!(self, Input::Contract { .. })
    }

    pub const fn coin_predicate_offset() -> usize {
        INPUT_COIN_FIXED_SIZE
    }

    pub const fn message_data_offset() -> usize {
        INPUT_MESSAGE_FIXED_SIZE
    }

    pub const fn balance_root(&self) -> Option<&Bytes32> {
        match self {
            Input::Contract(Contract { balance_root, .. }) => Some(balance_root),
            _ => None,
        }
    }

    pub const fn state_root(&self) -> Option<&Bytes32> {
        match self {
            Input::Contract(Contract { state_root, .. }) => Some(state_root),
            _ => None,
        }
    }

    pub const fn sender(&self) -> Option<&Address> {
        match self {
            Input::MessageSigned(MessageSigned { sender, .. })
            | Input::MessagePredicate(MessagePredicate { sender, .. }) => Some(sender),
            _ => None,
        }
    }

    pub const fn recipient(&self) -> Option<&Address> {
        match self {
            Input::MessageSigned(MessageSigned { recipient, .. })
            | Input::MessagePredicate(MessagePredicate { recipient, .. }) => Some(recipient),
            _ => None,
        }
    }

    pub const fn nonce(&self) -> Option<Word> {
        match self {
            Input::MessageSigned(MessageSigned { nonce, .. })
            | Input::MessagePredicate(MessagePredicate { nonce, .. }) => Some(*nonce),
            _ => None,
        }
    }

    /// Empties fields that should be zero during the signing.
    pub(crate) fn prepare_sign(&mut self) {
        match self {
            Input::CoinSigned(coin) => coin.prepare_sign(),
            Input::CoinPredicate(coin) => coin.prepare_sign(),
            Input::Contract(contract) => contract.prepare_sign(),
            Input::MessageSigned(message) => message.prepare_sign(),
            Input::MessagePredicate(message) => message.prepare_sign(),
        }
    }

    pub fn compute_message_id(
        sender: &Address,
        recipient: &Address,
        nonce: Word,
        amount: Word,
        data: &[u8],
    ) -> MessageId {
        let message_id = *Hasher::default()
            .chain(sender)
            .chain(recipient)
            .chain(nonce.to_be_bytes())
            .chain(amount.to_be_bytes())
            .chain(data)
            .finalize();

        message_id.into()
    }

    pub fn predicate_owner<P>(predicate: P) -> Address
    where
        P: AsRef<[u8]>,
    {
        use crate::Contract;

        let root = Contract::root_from_code(predicate);

        (*root).into()
    }

    #[cfg(feature = "std")]
    pub fn is_predicate_owner_valid<P>(owner: &Address, predicate: P) -> bool
    where
        P: AsRef<[u8]>,
    {
        owner == &Self::predicate_owner(predicate)
    }

    /// Prepare the output for VM predicate execution
    pub fn prepare_init_predicate(&mut self) {
        self.prepare_sign()
    }
}

#[cfg(feature = "std")]
impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let serialized_size = self.serialized_size();
        if buf.len() < serialized_size {
            return Err(bytes::eof());
        }

        match self {
            Self::CoinSigned(coin) => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Coin as Word);
                let _ = coin.read(buf)?;
            }
            Self::CoinPredicate(coin) => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Coin as Word);
                let _ = coin.read(buf)?;
            }

            Self::Contract(contract) => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Contract as Word);
                let _ = contract.read(buf)?;
            }

            Self::MessageSigned(message) => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Message as Word);
                let _ = message.read(buf)?;
            }
            Self::MessagePredicate(message) => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Message as Word);
                let _ = message.read(buf)?;
            }
        }

        Ok(serialized_size)
    }
}

#[cfg(feature = "std")]
impl io::Write for Input {
    fn write(&mut self, full_buf: &[u8]) -> io::Result<usize> {
        let identifier: &[_; WORD_SIZE] = full_buf
            .get(..WORD_SIZE)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        // Safety: buf len is checked
        let identifier = bytes::restore_word(bytes::from_array(identifier));
        let identifier = InputRepr::try_from(identifier)?;

        match identifier {
            InputRepr::Coin => {
                let mut coin = CoinFull::default();
                let n = WORD_SIZE + CoinFull::write(&mut coin, buf)?;

                *self = if coin.predicate.is_empty() {
                    Self::CoinSigned(coin.into_signed())
                } else {
                    Self::CoinPredicate(coin.into_predicate())
                };

                Ok(n)
            }

            InputRepr::Contract => {
                let mut contract = Contract::default();
                let n = WORD_SIZE + Contract::write(&mut contract, buf)?;

                *self = Self::Contract(contract);

                Ok(n)
            }

            InputRepr::Message => {
                let mut message = MessageFull::default();
                let n = WORD_SIZE + MessageFull::write(&mut message, buf)?;

                *self = if message.predicate.is_empty() {
                    Self::MessageSigned(message.into_signed())
                } else {
                    Self::MessagePredicate(message.into_predicate())
                };

                Ok(n)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
