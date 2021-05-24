use super::{Address, Color, ContractAddress, Hash};
use crate::bytes::{self, SizedBytes};

use fuel_asm::Word;

use std::convert::TryFrom;
use std::{io, mem};

const ADDRESS_SIZE: usize = mem::size_of::<Address>();
const COLOR_SIZE: usize = mem::size_of::<Color>();
const CONTRACT_ADDRESS_SIZE: usize = mem::size_of::<ContractAddress>();
const HASH_SIZE: usize = mem::size_of::<Hash>();
const WORD_SIZE: usize = mem::size_of::<Word>();

const OUTPUT_COIN_SIZE: usize = WORD_SIZE // Identifier
    + ADDRESS_SIZE // To
    + WORD_SIZE // Amount
    + COLOR_SIZE; // Color

const OUTPUT_CONTRACT_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Input index
    + HASH_SIZE // Balance root
    + HASH_SIZE; // State root

const OUTPUT_CONTRACT_CREATED_SIZE: usize = WORD_SIZE // Identifier
    + CONTRACT_ADDRESS_SIZE; // Contract Id

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OutputRepr {
    Coin = 0x00,
    Contract = 0x01,
    Withdrawal = 0x02,
    Change = 0x03,
    Variable = 0x04,
    ContractCreated = 0x05,
}

impl TryFrom<Word> for OutputRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Coin),
            0x01 => Ok(Self::Contract),
            0x02 => Ok(Self::Withdrawal),
            0x03 => Ok(Self::Change),
            0x04 => Ok(Self::Variable),
            0x05 => Ok(Self::ContractCreated),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}

impl From<&mut Output> for OutputRepr {
    fn from(o: &mut Output) -> Self {
        match o {
            Output::Coin { .. } => Self::Coin,
            Output::Contract { .. } => Self::Contract,
            Output::Withdrawal { .. } => Self::Withdrawal,
            Output::Change { .. } => Self::Change,
            Output::Variable { .. } => Self::Variable,
            Output::ContractCreated { .. } => Self::ContractCreated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Output {
    Coin {
        to: Address,
        amount: Word,
        color: Color,
    },

    Contract {
        input_index: u8,
        balance_root: Hash,
        state_root: Hash,
    },

    Withdrawal {
        to: Address,
        amount: Word,
        color: Color,
    },

    Change {
        to: Address,
        amount: Word,
        color: Color,
    },

    Variable {
        to: Address,
        amount: Word,
        color: Color,
    },

    ContractCreated {
        contract_id: ContractAddress,
    },
}

impl Default for Output {
    fn default() -> Self {
        Self::ContractCreated {
            contract_id: Default::default(),
        }
    }
}

impl bytes::SizedBytes for Output {
    fn serialized_size(&self) -> usize {
        match self {
            Self::Coin { .. } | Self::Withdrawal { .. } | Self::Change { .. } | Self::Variable { .. } => {
                OUTPUT_COIN_SIZE
            }

            Self::Contract { .. } => OUTPUT_CONTRACT_SIZE,

            Self::ContractCreated { .. } => OUTPUT_CONTRACT_CREATED_SIZE,
        }
    }
}

impl Output {
    pub const fn coin(to: Address, amount: Word, color: Color) -> Self {
        Self::Coin { to, amount, color }
    }

    pub const fn contract(input_index: u8, balance_root: Hash, state_root: Hash) -> Self {
        Self::Contract {
            input_index,
            balance_root,
            state_root,
        }
    }

    pub const fn withdrawal(to: Address, amount: Word, color: Color) -> Self {
        Self::Withdrawal { to, amount, color }
    }

    pub const fn change(to: Address, amount: Word, color: Color) -> Self {
        Self::Change { to, amount, color }
    }

    pub const fn variable(to: Address, amount: Word, color: Color) -> Self {
        Self::Variable { to, amount, color }
    }

    pub const fn contract_created(contract_id: ContractAddress) -> Self {
        Self::ContractCreated { contract_id }
    }

    pub const fn color(&self) -> Option<&Color> {
        match self {
            Self::Coin { color, .. } => Some(color),
            Self::Withdrawal { color, .. } => Some(color),
            Self::Change { color, .. } => Some(color),
            Self::Variable { color, .. } => Some(color),
            _ => None,
        }
    }
}

impl io::Read for Output {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let identifier: OutputRepr = self.into();
        buf = bytes::store_number_unchecked(buf, identifier as Word);

        match self {
            Self::Coin { to, amount, color }
            | Self::Withdrawal { to, amount, color }
            | Self::Change { to, amount, color }
            | Self::Variable { to, amount, color } => {
                buf = bytes::store_array_unchecked(buf, to);
                buf = bytes::store_number_unchecked(buf, *amount);
                bytes::store_array_unchecked(buf, color);
            }

            Self::Contract {
                input_index,
                balance_root,
                state_root,
            } => {
                buf = bytes::store_number_unchecked(buf, *input_index);
                buf = bytes::store_array_unchecked(buf, balance_root);
                bytes::store_array_unchecked(buf, state_root);
            }

            Self::ContractCreated { contract_id } => {
                bytes::store_array_unchecked(buf, contract_id);
            }
        }

        Ok(n)
    }
}

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        let (identifier, buf): (Word, _) = bytes::restore_number_unchecked(buf);
        let identifier = OutputRepr::try_from(identifier)?;

        match identifier {
            OutputRepr::Coin | OutputRepr::Withdrawal | OutputRepr::Change | OutputRepr::Variable
                if buf.len() < OUTPUT_COIN_SIZE - WORD_SIZE =>
            {
                Err(bytes::eof())
            }

            OutputRepr::Contract if buf.len() < OUTPUT_CONTRACT_SIZE - WORD_SIZE => Err(bytes::eof()),

            OutputRepr::ContractCreated if buf.len() < OUTPUT_CONTRACT_CREATED_SIZE - WORD_SIZE => Err(bytes::eof()),

            OutputRepr::Coin | OutputRepr::Withdrawal | OutputRepr::Change | OutputRepr::Variable => {
                let (to, buf) = bytes::restore_array_unchecked(buf);
                let (amount, buf) = bytes::restore_number_unchecked(buf);
                let (color, _) = bytes::restore_array_unchecked(buf);

                match identifier {
                    OutputRepr::Coin => *self = Self::Coin { to, amount, color },
                    OutputRepr::Withdrawal => *self = Self::Withdrawal { to, amount, color },
                    OutputRepr::Change => *self = Self::Change { to, amount, color },
                    OutputRepr::Variable => *self = Self::Variable { to, amount, color },

                    _ => unreachable!(),
                }

                Ok(OUTPUT_COIN_SIZE)
            }

            OutputRepr::Contract => {
                let (input_index, buf) = bytes::restore_u8_unchecked(buf);
                let (balance_root, buf) = bytes::restore_array_unchecked(buf);
                let (state_root, _) = bytes::restore_array_unchecked(buf);

                *self = Self::Contract {
                    input_index,
                    balance_root,
                    state_root,
                };

                Ok(OUTPUT_CONTRACT_SIZE)
            }

            OutputRepr::ContractCreated => {
                let (contract_id, _) = bytes::restore_array_unchecked(buf);
                *self = Self::ContractCreated { contract_id };

                Ok(OUTPUT_CONTRACT_CREATED_SIZE)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
