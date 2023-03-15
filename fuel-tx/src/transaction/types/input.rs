use crate::{TxPointer, UtxoId};

use fuel_crypto::{Hasher, PublicKey};
use fuel_types::{bytes, MemLayout, MemLocType};
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId, Word};

use core::mem;

#[cfg(feature = "std")]
use fuel_types::bytes::{Deserializable, SizedBytes, WORD_SIZE};

use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::io;

mod consts;
mod repr;
mod sizes;

use consts::*;
use sizes::*;

pub use repr::InputRepr;

#[cfg(all(test, feature = "std"))]
mod ser_de_tests;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Input {
    CoinSigned {
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        tx_pointer: TxPointer,
        witness_index: u8,
        maturity: Word,
    },

    CoinPredicate {
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        tx_pointer: TxPointer,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
        predicate_gas_used: Word,
    },

    Contract {
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        tx_pointer: TxPointer,
        contract_id: ContractId,
    },

    MessageSigned {
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        witness_index: u8,
        data: Vec<u8>,
    },

    MessagePredicate {
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        data: Vec<u8>,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
        predicate_gas_used: Word,
    },
}

impl Default for Input {
    fn default() -> Self {
        Self::Contract {
            utxo_id: Default::default(),
            balance_root: Default::default(),
            state_root: Default::default(),
            tx_pointer: Default::default(),
            contract_id: Default::default(),
        }
    }
}

impl bytes::SizedBytes for Input {
    fn serialized_size(&self) -> usize {
        match self {
            Self::CoinSigned { .. } => INPUT_COIN_FIXED_SIZE,

            Self::CoinPredicate {
                predicate,
                predicate_data,
                ..
            } => {
                INPUT_COIN_FIXED_SIZE
                    + bytes::padded_len(predicate.as_slice())
                    + bytes::padded_len(predicate_data.as_slice())
            }

            Self::Contract { .. } => INPUT_CONTRACT_SIZE,

            Self::MessageSigned { data, .. } => INPUT_MESSAGE_FIXED_SIZE + bytes::padded_len(data.as_slice()),

            Self::MessagePredicate {
                data,
                predicate,
                predicate_data,
                ..
            } => {
                INPUT_MESSAGE_FIXED_SIZE
                    + bytes::padded_len(data.as_slice())
                    + bytes::padded_len(predicate.as_slice())
                    + bytes::padded_len(predicate_data.as_slice())
            }
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
        predicate_gas_used: Word,
    ) -> Self {
        Self::CoinPredicate {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            maturity,
            predicate,
            predicate_data,
            predicate_gas_used,
        }
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
        Self::CoinSigned {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            maturity,
        }
    }

    pub const fn contract(
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        tx_pointer: TxPointer,
        contract_id: ContractId,
    ) -> Self {
        Self::Contract {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        }
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
        Self::MessageSigned {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
        }
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
        predicate_gas_used: Word,
    ) -> Self {
        Self::MessagePredicate {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            data,
            predicate,
            predicate_data,
            predicate_gas_used,
        }
    }

    pub const fn utxo_id(&self) -> Option<&UtxoId> {
        match self {
            Self::CoinSigned { utxo_id, .. } | Self::CoinPredicate { utxo_id, .. } | Self::Contract { utxo_id, .. } => {
                Some(utxo_id)
            }
            Self::MessageSigned { .. } => None,
            Self::MessagePredicate { .. } => None,
        }
    }

    pub const fn input_owner(&self) -> Option<&Address> {
        match self {
            Self::CoinSigned { owner, .. } | Self::CoinPredicate { owner, .. } => Some(owner),
            Self::MessageSigned { .. } | Self::MessagePredicate { .. } | Self::Contract { .. } => None,
        }
    }

    pub const fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Input::CoinSigned { asset_id, .. } | Input::CoinPredicate { asset_id, .. } => Some(asset_id),
            Input::MessageSigned { .. } | Input::MessagePredicate { .. } => Some(&AssetId::BASE),
            Input::Contract { .. } => None,
        }
    }

    pub const fn contract_id(&self) -> Option<&ContractId> {
        match self {
            Self::Contract { contract_id, .. } => Some(contract_id),
            _ => None,
        }
    }

    pub const fn amount(&self) -> Option<Word> {
        match self {
            Input::CoinSigned { amount, .. }
            | Input::CoinPredicate { amount, .. }
            | Input::MessageSigned { amount, .. }
            | Input::MessagePredicate { amount, .. } => Some(*amount),
            Input::Contract { .. } => None,
        }
    }

    pub const fn witness_index(&self) -> Option<u8> {
        match self {
            Input::CoinSigned { witness_index, .. } | Input::MessageSigned { witness_index, .. } => {
                Some(*witness_index)
            }
            Input::CoinPredicate { .. } | Input::Contract { .. } | Input::MessagePredicate { .. } => None,
        }
    }

    pub const fn maturity(&self) -> Option<Word> {
        match self {
            Input::CoinSigned { maturity, .. } | Input::CoinPredicate { maturity, .. } => Some(*maturity),
            Input::Contract { .. } | Input::MessageSigned { .. } | Input::MessagePredicate { .. } => None,
        }
    }

    pub fn predicate_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate { .. } => InputRepr::Coin.coin_predicate_offset(),
            Input::MessagePredicate { data, .. } => {
                InputRepr::Message.data_offset().map(|o| o + bytes::padded_len(data))
            }
            Input::CoinSigned { .. } | Input::Contract { .. } | Input::MessageSigned { .. } => None,
        }
    }

    pub fn predicate_data_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate { predicate, .. } | Input::MessagePredicate { predicate, .. } => {
                self.predicate_offset().map(|o| o + bytes::padded_len(predicate))
            }
            Input::CoinSigned { .. } | Input::Contract { .. } | Input::MessageSigned { .. } => None,
        }
    }

    pub fn predicate_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate { predicate, .. } | Input::MessagePredicate { predicate, .. } => Some(predicate.len()),
            Input::CoinSigned { .. } | Input::MessageSigned { .. } => Some(0),
            Input::Contract { .. } => None,
        }
    }

    pub fn predicate_data_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate { predicate_data, .. } | Input::MessagePredicate { predicate_data, .. } => {
                Some(predicate_data.len())
            }
            Input::CoinSigned { .. } | Input::MessageSigned { .. } => Some(0),
            Input::Contract { .. } => None,
        }
    }

    pub const fn message_id(&self) -> Option<&MessageId> {
        match self {
            Self::MessagePredicate { message_id, .. } | Self::MessageSigned { message_id, .. } => Some(message_id),
            _ => None,
        }
    }

    pub const fn tx_pointer(&self) -> Option<&TxPointer> {
        match self {
            Input::CoinSigned { tx_pointer, .. }
            | Input::CoinPredicate { tx_pointer, .. }
            | Input::Contract { tx_pointer, .. } => Some(tx_pointer),
            _ => None,
        }
    }

    pub fn input_data(&self) -> Option<&[u8]> {
        match self {
            Input::MessageSigned { data, .. } | Input::MessagePredicate { data, .. } => Some(data),
            _ => None,
        }
    }

    pub fn input_predicate(&self) -> Option<&[u8]> {
        match self {
            Input::CoinPredicate { predicate, .. } | Input::MessagePredicate { predicate, .. } => Some(predicate),

            _ => None,
        }
    }

    pub fn input_predicate_data(&self) -> Option<&[u8]> {
        match self {
            Input::CoinPredicate { predicate_data, .. } | Input::MessagePredicate { predicate_data, .. } => {
                Some(predicate_data)
            }

            _ => None,
        }
    }

    pub fn input_predicate_gas_used(&self) -> Option<&Word> {
        match self {
            Input::CoinPredicate { predicate_gas_used, .. } | Input::MessagePredicate { predicate_gas_used, .. } => {
                Some(predicate_gas_used)
            }

            _ => None,
        }
    }

    /// Return a tuple containing the predicate, its data, and the gas used if the input is of
    /// type `CoinPredicate` or `MessagePredicate`
    pub fn predicate(&self) -> Option<(&[u8], &[u8], &Word)> {
        match self {
            Input::CoinPredicate {
                predicate,
                predicate_data,
                predicate_gas_used,
                ..
            }
            | Input::MessagePredicate {
                predicate,
                predicate_data,
                predicate_gas_used,
                ..
            } => Some((predicate.as_slice(), predicate_data.as_slice(), predicate_gas_used)),

            _ => None,
        }
    }

    pub const fn is_coin(&self) -> bool {
        self.is_coin_signed() | self.is_coin_predicate()
    }

    pub const fn is_coin_signed(&self) -> bool {
        matches!(self, Input::CoinSigned { .. })
    }

    pub const fn is_coin_predicate(&self) -> bool {
        matches!(self, Input::CoinPredicate { .. })
    }

    pub const fn is_message(&self) -> bool {
        self.is_message_signed() | self.is_message_predicate()
    }

    pub const fn is_message_signed(&self) -> bool {
        matches!(self, Input::MessageSigned { .. })
    }

    pub const fn is_message_predicate(&self) -> bool {
        matches!(self, Input::MessagePredicate { .. })
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
            Input::Contract { balance_root, .. } => Some(balance_root),
            _ => None,
        }
    }

    pub const fn state_root(&self) -> Option<&Bytes32> {
        match self {
            Input::Contract { state_root, .. } => Some(state_root),
            _ => None,
        }
    }

    pub const fn sender(&self) -> Option<&Address> {
        match self {
            Input::MessageSigned { sender, .. } | Input::MessagePredicate { sender, .. } => Some(sender),
            _ => None,
        }
    }

    pub const fn recipient(&self) -> Option<&Address> {
        match self {
            Input::MessageSigned { recipient, .. } | Input::MessagePredicate { recipient, .. } => Some(recipient),
            _ => None,
        }
    }

    pub const fn nonce(&self) -> Option<Word> {
        match self {
            Input::MessageSigned { nonce, .. } | Input::MessagePredicate { nonce, .. } => Some(*nonce),
            _ => None,
        }
    }

    /// Empties fields that should be zero during the signing.
    pub(crate) fn prepare_sign(&mut self) {
        match self {
            Input::CoinSigned { tx_pointer, .. } | Input::CoinPredicate { tx_pointer, .. } => {
                mem::take(tx_pointer);
            }

            Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                ..
            } => {
                mem::take(utxo_id);
                mem::take(balance_root);
                mem::take(state_root);
                mem::take(tx_pointer);
            }

            _ => (),
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
    fn read(&mut self, full_buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if full_buf.len() < n {
            return Err(bytes::eof());
        }

        match self {
            Self::CoinSigned {
                utxo_id,
                owner,
                amount,
                asset_id,
                tx_pointer,
                witness_index,
                maturity,
            } => {
                type S = CoinSizes;
                const LEN: usize = CoinSizes::LEN;
                let buf: &mut [_; LEN] = full_buf
                    .get_mut(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;
                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), InputRepr::Coin as u8);

                let n = utxo_id.read(&mut buf[S::LAYOUT.utxo_id.range()])?;
                if n != S::LAYOUT.utxo_id.size() {
                    return Err(bytes::eof());
                }

                bytes::store_at(buf, S::layout(S::LAYOUT.owner), owner);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);

                let n = tx_pointer.read(&mut buf[S::LAYOUT.tx_pointer.range()])?;
                if n != S::LAYOUT.tx_pointer.size() {
                    return Err(bytes::eof());
                }

                bytes::store_number_at(buf, S::layout(S::LAYOUT.witness_index), *witness_index);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.maturity), *maturity);

                // Predicate len zeroed for signed coin
                bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_len), 0);

                // Predicate data len zeroed for signed coin
                bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_data_len), 0);
            }

            Self::CoinPredicate {
                utxo_id,
                owner,
                amount,
                asset_id,
                tx_pointer,
                maturity,
                predicate,
                predicate_data,
                predicate_gas_used,
            } => {
                type S = CoinSizes;
                const LEN: usize = CoinSizes::LEN;
                let buf: &mut [_; LEN] = full_buf
                    .get_mut(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;
                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), InputRepr::Coin as u8);

                let n = utxo_id.read(&mut buf[S::LAYOUT.utxo_id.range()])?;
                if n != S::LAYOUT.utxo_id.size() {
                    return Err(bytes::eof());
                }

                bytes::store_at(buf, S::layout(S::LAYOUT.owner), owner);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);

                let n = tx_pointer.read(&mut buf[S::LAYOUT.tx_pointer.range()])?;
                if n != S::LAYOUT.tx_pointer.size() {
                    return Err(bytes::eof());
                }

                // Witness index zeroed for coin predicate
                bytes::store_number_at(buf, S::layout(S::LAYOUT.witness_index), 0);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.maturity), *maturity);

                bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_len), predicate.len() as Word);

                bytes::store_number_at(
                    buf,
                    S::layout(S::LAYOUT.predicate_data_len),
                    predicate_data.len() as Word,
                );

                let (_, buf) =
                    bytes::store_raw_bytes(full_buf.get_mut(LEN..).ok_or(bytes::eof())?, predicate.as_slice())?;

                let (_, buf) = bytes::store_raw_bytes(buf, predicate_data.as_slice())?;

                bytes::store_number_unchecked(buf, predicate_gas_used as Word);
            }

            Self::Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            } => {
                type S = ContractSizes;
                const LEN: usize = ContractSizes::LEN;
                let buf: &mut [_; LEN] = full_buf
                    .get_mut(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), InputRepr::Contract as u8);
                bytes::store_at(buf, S::layout(S::LAYOUT.tx_id), utxo_id.tx_id());
                bytes::store_number_at(buf, S::layout(S::LAYOUT.output_index), utxo_id.output_index() as Word);
                bytes::store_at(buf, S::layout(S::LAYOUT.balance_root), balance_root);
                bytes::store_at(buf, S::layout(S::LAYOUT.state_root), state_root);

                let n = tx_pointer.read(&mut buf[S::LAYOUT.tx_pointer.range()])?;
                if n != S::LAYOUT.tx_pointer.size() {
                    return Err(bytes::eof());
                }

                bytes::store_at(buf, S::layout(S::LAYOUT.contract_id), contract_id);
            }

            Self::MessageSigned {
                message_id,
                sender,
                recipient,
                amount,
                nonce,
                witness_index,
                data,
            } => {
                type S = MessageSizes;
                const LEN: usize = MessageSizes::LEN;
                let buf: &mut [_; LEN] = full_buf
                    .get_mut(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), InputRepr::Message as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.message_id), message_id);
                bytes::store_at(buf, S::layout(S::LAYOUT.sender), sender);
                bytes::store_at(buf, S::layout(S::LAYOUT.recipient), recipient);

                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.nonce), *nonce);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.witness_index), *witness_index);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.data_len), data.len() as Word);

                // predicate + data are empty for signed message
                bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_len), 0);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_data_len), 0);

                bytes::store_raw_bytes(full_buf.get_mut(LEN..).ok_or(bytes::eof())?, data.as_slice())?;
            }

            Self::MessagePredicate {
                message_id,
                sender,
                recipient,
                amount,
                nonce,
                data,
                predicate,
                predicate_data,
                predicate_gas_used,
            } => {
                let witness_index = 0;

                type S = MessageSizes;
                const LEN: usize = MessageSizes::LEN;
                let buf: &mut [_; LEN] = full_buf
                    .get_mut(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), InputRepr::Message as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.message_id), message_id);
                bytes::store_at(buf, S::layout(S::LAYOUT.sender), sender);
                bytes::store_at(buf, S::layout(S::LAYOUT.recipient), recipient);

                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.nonce), *nonce);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.witness_index), witness_index);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.data_len), data.len() as Word);

                bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_len), predicate.len() as Word);
                bytes::store_number_at(
                    buf,
                    S::layout(S::LAYOUT.predicate_data_len),
                    predicate_data.len() as Word,
                );

                let (_, buf) = bytes::store_raw_bytes(full_buf.get_mut(LEN..).ok_or(bytes::eof())?, data.as_slice())?;

                let (_, buf) = bytes::store_raw_bytes(buf, predicate.as_slice())?;

                let (_, buf) = bytes::store_raw_bytes(buf, predicate_data.as_slice())?;
                
				bytes::store_number_at(buf, S::layout(S::LAYOUT.predicate_gas_used), predicate_gas_used as Word);
            }
        }

        Ok(n)
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
                type S = CoinSizes;
                const LEN: usize = CoinSizes::LEN;
                let buf: &[_; LEN] = full_buf
                    .get(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let mut n = INPUT_COIN_FIXED_SIZE;

                let utxo_id = UtxoId::from_bytes(&buf[S::LAYOUT.utxo_id.range()])?;

                let owner = bytes::restore_at(buf, S::layout(S::LAYOUT.owner));
                let amount = bytes::restore_number_at(buf, S::layout(S::LAYOUT.amount));
                let asset_id = bytes::restore_at(buf, S::layout(S::LAYOUT.asset_id));

                let tx_pointer = TxPointer::from_bytes(&buf[S::LAYOUT.tx_pointer.range()])?;

                let witness_index = bytes::restore_u8_at(buf, S::layout(S::LAYOUT.witness_index));
                let maturity = bytes::restore_number_at(buf, S::layout(S::LAYOUT.maturity));

                let predicate_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_len));
                let predicate_data_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_data_len));

                let (size, predicate, buf) =
                    bytes::restore_raw_bytes(full_buf.get(LEN..).ok_or(bytes::eof())?, predicate_len)?;
                n += size;

                let (size, predicate_data, buf) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
                n += size;

                let owner = owner.into();
                let asset_id = asset_id.into();

                *self = if predicate.is_empty() {
                    Self::CoinSigned {
                        utxo_id,
                        owner,
                        amount,
                        asset_id,
                        tx_pointer,
                        witness_index,
                        maturity,
                    }
                } else {
                	let predicate_gas_used = bytes::restore_number_at(buf, S::layout(S::LAYOUT.predicate_gas_used));


                    Self::CoinPredicate {
                        utxo_id,
                        owner,
                        amount,
                        asset_id,
                        tx_pointer,
                        maturity,
                        predicate,
                        predicate_data,
                        predicate_gas_used,
                    }
                };

                Ok(n)
            }

            InputRepr::Contract => {
                type S = ContractSizes;
                const LEN: usize = ContractSizes::LEN;
                let buf: &[_; LEN] = full_buf
                    .get(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let utxo_id =
                    UtxoId::from_bytes(&buf[S::LAYOUT.tx_id.range().start..S::LAYOUT.output_index.range().end])?;

                let balance_root = bytes::restore_at(buf, S::layout(S::LAYOUT.balance_root));
                let state_root = bytes::restore_at(buf, S::layout(S::LAYOUT.state_root));

                let tx_pointer = TxPointer::from_bytes(&buf[S::LAYOUT.tx_pointer.range()])?;

                let contract_id = bytes::restore_at(buf, S::layout(S::LAYOUT.contract_id));

                let balance_root = balance_root.into();
                let state_root = state_root.into();
                let contract_id = contract_id.into();

                *self = Self::Contract {
                    utxo_id,
                    balance_root,
                    state_root,
                    tx_pointer,
                    contract_id,
                };

                Ok(INPUT_CONTRACT_SIZE)
            }

            InputRepr::Message => {
                type S = MessageSizes;
                const LEN: usize = MessageSizes::LEN;
                let buf: &[_; LEN] = full_buf
                    .get(..LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;
                let mut n = INPUT_MESSAGE_FIXED_SIZE;

                // Safety: buf len is checked
                let message_id = bytes::restore_at(buf, S::layout(S::LAYOUT.message_id));
                let sender = bytes::restore_at(buf, S::layout(S::LAYOUT.sender));
                let recipient = bytes::restore_at(buf, S::layout(S::LAYOUT.recipient));

                let amount = bytes::restore_number_at(buf, S::layout(S::LAYOUT.amount));
                let nonce = bytes::restore_number_at(buf, S::layout(S::LAYOUT.nonce));
                let witness_index = bytes::restore_u8_at(buf, S::layout(S::LAYOUT.witness_index));

                let data_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.data_len));
                let predicate_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_len));
                let predicate_data_len = bytes::restore_usize_at(buf, S::layout(S::LAYOUT.predicate_data_len));

                let (size, data, buf) = bytes::restore_raw_bytes(full_buf.get(LEN..).ok_or(bytes::eof())?, data_len)?;
                n += size;

                let (size, predicate, buf) = bytes::restore_raw_bytes(buf, predicate_len)?;
                n += size;

                let (size, predicate_data, buf) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
                n += size;

                let message_id = message_id.into();
                let sender = sender.into();
                let recipient = recipient.into();

                *self = if predicate.is_empty() {
                    Self::message_signed(message_id, sender, recipient, amount, nonce, witness_index, data)
                } else {
                    let predicate_gas_used = bytes::restore_number_at(buf, S::layout(S::LAYOUT.predicate_gas_used));

                    Self::message_predicate(
                        message_id,
                        sender,
                        recipient,
                        amount,
                        nonce,
                        data,
                        predicate,
                        predicate_data,
                        predicate_gas_used,
                    )
                };

                Ok(n)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
