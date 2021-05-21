use super::{Address, Color, ContractAddress, Hash};
use crate::bytes;

use fuel_asm::Word;

use std::convert::TryFrom;
use std::{io, mem};

const ADDRESS_SIZE: usize = mem::size_of::<Address>();
const COLOR_SIZE: usize = mem::size_of::<Color>();
const CONTRACT_ADDRESS_SIZE: usize = mem::size_of::<ContractAddress>();
const HASH_SIZE: usize = mem::size_of::<Hash>();
const WORD_SIZE: usize = mem::size_of::<Word>();

const INPUT_COIN_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + HASH_SIZE // UTXO Id
    + ADDRESS_SIZE // Owner
    + WORD_SIZE // Amount
    + COLOR_SIZE // Color
    + WORD_SIZE // Witness index
    + WORD_SIZE // Maturity
    + WORD_SIZE // Predicate size
    + WORD_SIZE; // Predicate data size

const INPUT_CONTRACT_SIZE: usize = WORD_SIZE // Identifier
    + HASH_SIZE // UTXO Id
    + HASH_SIZE // Balance root
    + HASH_SIZE // State root
    + CONTRACT_ADDRESS_SIZE; // Contract address

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum InputRepr {
    Coin = 0x00,
    Contract = 0x01,
}

impl TryFrom<Word> for InputRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Coin),
            0x01 => Ok(Self::Contract),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Input {
    Coin {
        utxo_id: Hash,
        owner: Address,
        amount: Word,
        color: Color,
        witness_index: u8,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    },

    Contract {
        utxo_id: Hash,
        balance_root: Hash,
        state_root: Hash,
        contract_id: ContractAddress,
    },
}

impl Input {
    pub const fn coin(
        utxo_id: Hash,
        owner: Address,
        amount: Word,
        color: Color,
        witness_index: u8,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::Coin {
            utxo_id,
            owner,
            amount,
            color,
            witness_index,
            maturity,
            predicate,
            predicate_data,
        }
    }

    pub const fn contract(utxo_id: Hash, balance_root: Hash, state_root: Hash, contract_id: ContractAddress) -> Self {
        Self::Contract {
            utxo_id,
            balance_root,
            state_root,
            contract_id,
        }
    }

    pub const fn utxo_id(&self) -> &Hash {
        match self {
            Self::Coin { utxo_id, .. } => &utxo_id,
            Self::Contract { utxo_id, .. } => &utxo_id,
        }
    }

    pub fn serialized_size(&self) -> usize {
        match self {
            Self::Coin {
                predicate,
                predicate_data,
                ..
            } => {
                INPUT_COIN_FIXED_SIZE
                    + bytes::padded_len(predicate.as_slice())
                    + bytes::padded_len(predicate_data.as_slice())
            }

            Self::Contract { .. } => INPUT_CONTRACT_SIZE,
        }
    }
}

impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Coin {
                utxo_id,
                owner,
                amount,
                color,
                witness_index,
                maturity,
                predicate,
                predicate_data,
            } => {
                let mut n = INPUT_COIN_FIXED_SIZE;
                if buf.len() < n {
                    return Err(bytes::eof());
                }

                let buf = bytes::store_number_unchecked(buf, InputRepr::Coin as Word);
                let buf = bytes::store_array_unchecked(buf, utxo_id);
                let buf = bytes::store_array_unchecked(buf, owner);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, color);
                let buf = bytes::store_number_unchecked(buf, *witness_index);
                let buf = bytes::store_number_unchecked(buf, *maturity);

                let buf = bytes::store_number_unchecked(buf, predicate.len() as Word);
                let buf = bytes::store_number_unchecked(buf, predicate_data.len() as Word);

                let (size, buf) = bytes::store_raw_bytes(buf, predicate.as_slice())?;
                n += size;

                let (size, _) = bytes::store_raw_bytes(buf, predicate_data.as_slice())?;
                n += size;

                Ok(n)
            }

            Self::Contract { .. } if buf.len() < INPUT_CONTRACT_SIZE => Err(bytes::eof()),

            Self::Contract {
                utxo_id,
                balance_root,
                state_root,
                contract_id,
            } => {
                let buf = bytes::store_number_unchecked(buf, InputRepr::Contract as Word);
                let buf = bytes::store_array_unchecked(buf, utxo_id);
                let buf = bytes::store_array_unchecked(buf, balance_root);
                let buf = bytes::store_array_unchecked(buf, state_root);
                bytes::store_array_unchecked(buf, contract_id);

                Ok(INPUT_CONTRACT_SIZE)
            }
        }
    }
}

impl io::Write for Input {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        let (identifier, buf): (Word, _) = bytes::restore_number_unchecked(buf);
        let identifier = InputRepr::try_from(identifier)?;

        match identifier {
            InputRepr::Coin if buf.len() < INPUT_COIN_FIXED_SIZE - WORD_SIZE => Err(bytes::eof()),

            InputRepr::Coin => {
                let mut n = INPUT_COIN_FIXED_SIZE;

                let (utxo_id, buf) = bytes::restore_array_unchecked(buf);
                let (owner, buf) = bytes::restore_array_unchecked(buf);
                let (amount, buf) = bytes::restore_number_unchecked(buf);
                let (color, buf) = bytes::restore_array_unchecked(buf);
                let (witness_index, buf) = bytes::restore_u8_unchecked(buf);
                let (maturity, buf) = bytes::restore_number_unchecked(buf);

                let (predicate_len, buf) = bytes::restore_usize_unchecked(buf);
                let (predicate_data_len, buf) = bytes::restore_usize_unchecked(buf);

                let (size, predicate, buf) = bytes::restore_raw_bytes(buf, predicate_len)?;
                n += size;

                let (size, predicate_data, _) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
                n += size;

                *self = Self::Coin {
                    utxo_id,
                    owner,
                    amount,
                    color,
                    witness_index,
                    maturity,
                    predicate,
                    predicate_data,
                };

                Ok(n)
            }

            InputRepr::Contract if buf.len() < INPUT_CONTRACT_SIZE - WORD_SIZE => Err(bytes::eof()),

            InputRepr::Contract => {
                let (utxo_id, buf) = bytes::restore_array_unchecked(buf);
                let (balance_root, buf) = bytes::restore_array_unchecked(buf);
                let (state_root, buf) = bytes::restore_array_unchecked(buf);
                let (contract_id, _) = bytes::restore_array_unchecked(buf);

                *self = Self::Contract {
                    utxo_id,
                    balance_root,
                    state_root,
                    contract_id,
                };

                Ok(INPUT_CONTRACT_SIZE)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
