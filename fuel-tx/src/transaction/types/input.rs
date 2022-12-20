use fuel_data::bytes::{self, SizedBytes};
use fuel_data::{Address, Bytes32, Color, ContractId, Word};

use std::convert::TryFrom;
use std::{io, mem};

const WORD_SIZE: usize = mem::size_of::<Word>();

const INPUT_COIN_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + Bytes32::LEN // UTXO Id
    + Address::LEN // Owner
    + WORD_SIZE // Amount
    + Color::LEN // Color
    + WORD_SIZE // Witness index
    + WORD_SIZE // Maturity
    + WORD_SIZE // Predicate size
    + WORD_SIZE; // Predicate data size

const INPUT_CONTRACT_SIZE: usize = WORD_SIZE // Identifier
    + Bytes32::LEN // UTXO Id
    + Bytes32::LEN // Balance root
    + Bytes32::LEN // State root
    + ContractId::LEN; // Contract address

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
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Input {
    Coin {
        utxo_id: Bytes32,
        owner: Address,
        amount: Word,
        color: Color,
        witness_index: u8,
        maturity: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    },

    Contract {
        utxo_id: Bytes32,
        balance_root: Bytes32,
        state_root: Bytes32,
        contract_id: ContractId,
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
            Self::Coin {
                predicate,
                predicate_data,
                ..
            } => {
                INPUT_COIN_FIXED_SIZE
                    + bytes::padded_len(predicate.as_slice())
                    + bytes::padded_len(predicate_data.as_slice())
            }

            _ => INPUT_CONTRACT_SIZE,
        }
    }
}

impl Input {
    pub const fn coin(
        utxo_id: Bytes32,
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

    pub const fn contract(
        utxo_id: Bytes32,
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

    pub const fn utxo_id(&self) -> &Bytes32 {
        match self {
            Self::Coin { utxo_id, .. } => &utxo_id,
            Self::Contract { utxo_id, .. } => &utxo_id,
        }
    }

    /// Return a tuple containing the predicate and its data if the input is of type `Coin`
    pub fn predicate(&self) -> Option<(&[u8], &[u8])> {
        match self {
            Input::Coin {
                predicate,
                predicate_data,
                ..
            } => Some((predicate.as_slice(), predicate_data.as_slice())),

            _ => None,
        }
    }

    pub const fn is_coin(&self) -> bool {
        match self {
            Input::Coin { .. } => true,
            _ => false,
        }
    }

    pub const fn coin_predicate_offset() -> usize {
        INPUT_COIN_FIXED_SIZE
    }

    pub fn coin_predicate_data_offset(&self) -> Option<usize> {
        match self {
            Input::Coin { predicate, .. } => {
                Some(Self::coin_predicate_offset() + bytes::padded_len(predicate.as_slice()))
            }

            _ => None,
        }
    }
}

impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

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
                let buf = bytes::store_number_unchecked(buf, InputRepr::Coin as Word);
                let buf = bytes::store_array_unchecked(buf, utxo_id);
                let buf = bytes::store_array_unchecked(buf, owner);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, color);
                let buf = bytes::store_number_unchecked(buf, *witness_index);
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
                let buf = bytes::store_array_unchecked(buf, utxo_id);
                let buf = bytes::store_array_unchecked(buf, balance_root);
                let buf = bytes::store_array_unchecked(buf, state_root);
                bytes::store_array_unchecked(buf, contract_id);
            }
        }

        Ok(n)
    }
}

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
                let (utxo_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (owner, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (color, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
                let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };

                let (predicate_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (predicate_data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };

                let (size, predicate, buf) = bytes::restore_raw_bytes(buf, predicate_len)?;
                n += size;

                let (size, predicate_data, _) = bytes::restore_raw_bytes(buf, predicate_data_len)?;
                n += size;

                let utxo_id = utxo_id.into();
                let owner = owner.into();
                let color = color.into();

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
                // Safety: checked buffer len
                let (utxo_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (balance_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (state_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (contract_id, _) = unsafe { bytes::restore_array_unchecked(buf) };

                let utxo_id = utxo_id.into();
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
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
