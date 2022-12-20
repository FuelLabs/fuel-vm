use crate::UtxoId;

use fuel_crypto::{Hasher, PublicKey};
use fuel_types::bytes::{self, WORD_SIZE};
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId, Word};

#[cfg(feature = "std")]
use fuel_types::bytes::SizedBytes;

use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::io;

const INPUT_COIN_FIXED_SIZE: usize = WORD_SIZE // Identifier  
    + Bytes32::LEN // UtxoId tx_id
    + WORD_SIZE // UtxoId output_index
    + Address::LEN // Owner
    + WORD_SIZE // Amount
    + AssetId::LEN // AssetId
    + WORD_SIZE // Witness index
    + WORD_SIZE // Maturity
    + WORD_SIZE // Predicate size
    + WORD_SIZE; // Predicate data size

const INPUT_CONTRACT_SIZE: usize = WORD_SIZE // Identifier
    + Bytes32::LEN // UtxoId tx_id
    + WORD_SIZE // UtxoId output_index
    + Bytes32::LEN // Balance root
    + Bytes32::LEN // State root
    + ContractId::LEN; // Contract address

const INPUT_MESSAGE_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + MessageId::LEN // message_id
    + Address::LEN // sender
    + Address::LEN // recipient
    + WORD_SIZE //amount
    + WORD_SIZE // nonce
    + Address::LEN // owner
    + WORD_SIZE // witness_index
    + WORD_SIZE // Data size
    + WORD_SIZE // Predicate size
    + WORD_SIZE; // Predicate data size

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg(feature = "std")]
enum InputRepr {
    Coin = 0x00,
    Contract = 0x01,
    Message = 0x02,
}

#[cfg(feature = "std")]
impl TryFrom<Word> for InputRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Coin),
            0x01 => Ok(Self::Contract),
            0x02 => Ok(Self::Message),
            id => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("The provided input identifier ({}) is invalid!", id),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Input {
    CoinSigned {
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        witness_index: u8,
        maturity: Word,
    },

    CoinPredicate {
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    },

    Contract {
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        contract_id: ContractId,
    },

    MessageSigned {
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        owner: Address,
        witness_index: u8,
        data: Vec<u8>,
    },

    MessagePredicate {
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        owner: Address,
        data: Vec<u8>,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    },
}

impl Default for Input {
    fn default() -> Self {
        Self::Contract {
            utxo_id: Default::default(),
            balance_root: Default::default(),
            state_root: Default::default(),
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

            Self::MessageSigned { data, .. } => {
                INPUT_MESSAGE_FIXED_SIZE + bytes::padded_len(data.as_slice())
            }

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
    pub fn owner(pk: &PublicKey) -> Address {
        let owner: [u8; Address::LEN] = pk.hash().into();

        owner.into()
    }

    pub const fn coin_predicate(
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::CoinPredicate {
            utxo_id,
            owner,
            amount,
            asset_id,
            maturity,
            predicate,
            predicate_data,
        }
    }

    pub const fn coin_signed(
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        witness_index: u8,
        maturity: Word,
    ) -> Self {
        Self::CoinSigned {
            utxo_id,
            owner,
            amount,
            asset_id,
            witness_index,
            maturity,
        }
    }

    pub const fn contract(
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        contract_id: ContractId,
    ) -> Self {
        Self::Contract {
            utxo_id,
            balance_root,
            state_root,
            contract_id,
        }
    }

    pub const fn message_signed(
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Word,
        owner: Address,
        witness_index: u8,
        data: Vec<u8>,
    ) -> Self {
        Self::MessageSigned {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            owner,
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
        owner: Address,
        data: Vec<u8>,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::MessagePredicate {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            owner,
            data,
            predicate,
            predicate_data,
        }
    }

    pub const fn utxo_id(&self) -> Option<&UtxoId> {
        match self {
            Self::CoinSigned { utxo_id, .. } => Some(utxo_id),
            Self::CoinPredicate { utxo_id, .. } => Some(utxo_id),
            Self::Contract { utxo_id, .. } => Some(utxo_id),
            Self::MessageSigned { .. } => None,
            Self::MessagePredicate { .. } => None,
        }
    }

    pub const fn contract_id(&self) -> Option<&ContractId> {
        match self {
            Self::Contract { contract_id, .. } => Some(contract_id),
            _ => None,
        }
    }

    pub const fn message_id(&self) -> Option<&MessageId> {
        match self {
            Self::MessagePredicate { message_id, .. } | Self::MessageSigned { message_id, .. } => {
                Some(message_id)
            }
            _ => None,
        }
    }

    /// Return a tuple containing the predicate and its data if the input is of
    /// type `CoinPredicate`
    pub fn predicate(&self) -> Option<(&[u8], &[u8])> {
        match self {
            Input::CoinPredicate {
                predicate,
                predicate_data,
                ..
            } => Some((predicate.as_slice(), predicate_data.as_slice())),

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

    pub const fn coin_predicate_offset() -> usize {
        INPUT_COIN_FIXED_SIZE
    }

    pub const fn message_data_offset() -> usize {
        INPUT_MESSAGE_FIXED_SIZE
    }

    pub fn coin_predicate_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate { predicate, .. } => Some(bytes::padded_len(predicate.as_slice())),

            _ => None,
        }
    }

    pub fn coin_predicate_data_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate { predicate, .. } => {
                Some(Self::coin_predicate_offset() + bytes::padded_len(predicate.as_slice()))
            }

            _ => None,
        }
    }

    pub fn compute_message_id(
        sender: &Address,
        recipient: &Address,
        nonce: Word,
        owner: &Address,
        amount: Word,
        data: &[u8],
    ) -> MessageId {
        let message_id = *Hasher::default()
            .chain(sender)
            .chain(recipient)
            .chain(nonce.to_be_bytes())
            .chain(owner)
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
}

#[cfg(feature = "std")]
impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        match self {
            Self::CoinSigned {
                utxo_id,
                owner,
                amount,
                asset_id,
                witness_index,
                maturity,
            } => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Coin as Word);
                let buf = bytes::store_array_unchecked(buf, utxo_id.tx_id());
                let buf = bytes::store_number_unchecked(buf, utxo_id.output_index() as Word);
                let buf = bytes::store_array_unchecked(buf, owner);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, asset_id);
                let buf = bytes::store_number_unchecked(buf, *witness_index);
                let buf = bytes::store_number_unchecked(buf, *maturity);

                // Predicate len zeroed for signed coin
                let buf = bytes::store_number_unchecked(buf, 0u64);

                // Predicate data len zeroed for signed coin
                bytes::store_number_unchecked(buf, 0u64);
            }

            Self::CoinPredicate {
                utxo_id,
                owner,
                amount,
                asset_id,
                maturity,
                predicate,
                predicate_data,
            } => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Coin as Word);
                let buf = bytes::store_array_unchecked(buf, utxo_id.tx_id());
                let buf = bytes::store_number_unchecked(buf, utxo_id.output_index() as Word);
                let buf = bytes::store_array_unchecked(buf, owner);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, asset_id);

                // Witness index zeroed for coin predicate
                let buf = bytes::store_number_unchecked(buf, 0u64);
                let buf = bytes::store_number_unchecked(buf, *maturity);

                let buf = bytes::store_number_unchecked(buf, predicate.len() as Word);
                let buf = bytes::store_number_unchecked(buf, predicate_data.len() as Word);

                let (_, buf) = bytes::store_raw_bytes(buf, predicate.as_slice())?;

                bytes::store_raw_bytes(buf, predicate_data.as_slice())?;
            }

            Self::Contract {
                utxo_id,
                balance_root,
                state_root,
                contract_id,
            } => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Contract as Word);
                let buf = bytes::store_array_unchecked(buf, utxo_id.tx_id());
                let buf = bytes::store_number_unchecked(buf, utxo_id.output_index() as Word);
                let buf = bytes::store_array_unchecked(buf, balance_root);
                let buf = bytes::store_array_unchecked(buf, state_root);

                bytes::store_array_unchecked(buf, contract_id);
            }

            Self::MessageSigned {
                message_id,
                sender,
                recipient,
                amount,
                nonce,
                owner,
                witness_index,
                data,
            } => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Message as Word);
                let buf = bytes::store_array_unchecked(buf, message_id);
                let buf = bytes::store_array_unchecked(buf, sender);
                let buf = bytes::store_array_unchecked(buf, recipient);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_number_unchecked(buf, *nonce);
                let buf = bytes::store_array_unchecked(buf, owner);
                let buf = bytes::store_number_unchecked(buf, *witness_index);
                let buf = bytes::store_number_unchecked(buf, data.len() as Word);

                // predicate + data are empty for signed message
                let buf = bytes::store_number_unchecked(buf, 0 as Word);
                let buf = bytes::store_number_unchecked(buf, 0 as Word);

                bytes::store_raw_bytes(buf, data.as_slice())?;
            }

            Self::MessagePredicate {
                message_id,
                sender,
                recipient,
                amount,
                nonce,
                owner,
                data,
                predicate,
                predicate_data,
            } => {
                let witness_index = 0 as Word;

                let buf = bytes::store_number_unchecked(buf, InputRepr::Message as Word);
                let buf = bytes::store_array_unchecked(buf, message_id);
                let buf = bytes::store_array_unchecked(buf, sender);
                let buf = bytes::store_array_unchecked(buf, recipient);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_number_unchecked(buf, *nonce);
                let buf = bytes::store_array_unchecked(buf, owner);
                let buf = bytes::store_number_unchecked(buf, witness_index);
                let buf = bytes::store_number_unchecked(buf, data.len() as Word);
                let buf = bytes::store_number_unchecked(buf, predicate.len() as Word);
                let buf = bytes::store_number_unchecked(buf, predicate_data.len() as Word);

                let (_, buf) = bytes::store_raw_bytes(buf, data.as_slice())?;
                let (_, buf) = bytes::store_raw_bytes(buf, predicate.as_slice())?;

                bytes::store_raw_bytes(buf, predicate_data.as_slice())?;
            }
        }

        Ok(n)
    }
}

#[cfg(feature = "std")]
impl io::Write for Input {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        // Safety: buf len is checked
        let (identifier, buf): (Word, _) = unsafe { bytes::restore_number_unchecked(buf) };
        let identifier = InputRepr::try_from(identifier)?;

        match identifier {
            InputRepr::Coin if buf.len() < INPUT_COIN_FIXED_SIZE - WORD_SIZE => Err(bytes::eof()),

            InputRepr::Coin => {
                let mut n = INPUT_COIN_FIXED_SIZE;

                // Safety: buf len is checked
                let (tx_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (output_index, buf) = unsafe { bytes::restore_number_unchecked::<Word>(buf) };
                let (owner, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (asset_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
                let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };

                let (predicate_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (predicate_data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };

                let (size, predicate, buf) = bytes::restore_raw_bytes(buf, predicate_len)?;
                n += size;

                let (size, predicate_data, _) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
                n += size;

                let utxo_id = UtxoId::new(tx_id.into(), output_index as u8);
                let owner = owner.into();
                let asset_id = asset_id.into();

                *self = if predicate.is_empty() {
                    Self::CoinSigned {
                        utxo_id,
                        owner,
                        amount,
                        asset_id,
                        witness_index,
                        maturity,
                    }
                } else {
                    Self::CoinPredicate {
                        utxo_id,
                        owner,
                        amount,
                        asset_id,
                        maturity,
                        predicate,
                        predicate_data,
                    }
                };

                Ok(n)
            }

            InputRepr::Contract if buf.len() < INPUT_CONTRACT_SIZE - WORD_SIZE => Err(bytes::eof()),

            InputRepr::Contract => {
                // Safety: checked buffer len
                let (tx_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (output_index, buf) = unsafe { bytes::restore_number_unchecked::<Word>(buf) };
                let (balance_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (state_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (contract_id, _) = unsafe { bytes::restore_array_unchecked(buf) };

                let utxo_id = UtxoId::new(tx_id.into(), output_index as u8);
                let balance_root = balance_root.into();
                let state_root = state_root.into();
                let contract_id = contract_id.into();

                *self = Self::Contract {
                    utxo_id,
                    balance_root,
                    state_root,
                    contract_id,
                };

                Ok(INPUT_CONTRACT_SIZE)
            }

            InputRepr::Message if buf.len() < INPUT_MESSAGE_FIXED_SIZE - WORD_SIZE => {
                Err(bytes::eof())
            }

            InputRepr::Message => {
                let mut n = INPUT_MESSAGE_FIXED_SIZE;

                // Safety: buf len is checked
                let (message_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (sender, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (recipient, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (nonce, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (owner, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };

                let (data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (predicate_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (predicate_data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };

                let (size, data, buf) = bytes::restore_raw_bytes(buf, data_len)?;
                n += size;

                let (size, predicate, buf) = bytes::restore_raw_bytes(buf, predicate_len)?;
                n += size;

                let (size, predicate_data, _) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
                n += size;

                let message_id = message_id.into();
                let sender = sender.into();
                let recipient = recipient.into();
                let owner = owner.into();

                *self = if predicate.is_empty() {
                    Self::message_signed(
                        message_id,
                        sender,
                        recipient,
                        amount,
                        nonce,
                        owner,
                        witness_index,
                        data,
                    )
                } else {
                    Self::message_predicate(
                        message_id,
                        sender,
                        recipient,
                        amount,
                        nonce,
                        owner,
                        data,
                        predicate,
                        predicate_data,
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
