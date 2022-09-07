use super::consts::*;
use super::Input;

#[cfg(feature = "std")]
use fuel_types::Word;

#[cfg(feature = "std")]
use std::io;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputRepr {
    Coin = 0x00,
    Contract = 0x01,
    Message = 0x02,
}

impl InputRepr {
    pub const fn utxo_id_offset(&self) -> Option<usize> {
        match self {
            Self::Coin | Self::Contract => Some(INPUT_UTXO_ID_OFFSET),
            Self::Message => None,
        }
    }

    pub const fn owner_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_OWNER_OFFSET),
            Self::Message => None,
            Self::Contract => None,
        }
    }

    pub const fn asset_id_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_ASSET_ID_OFFSET),
            Self::Message | Self::Contract => None,
        }
    }

    pub const fn data_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_FIXED_SIZE),
            Self::Coin | Self::Contract => None,
        }
    }

    pub const fn coin_predicate_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_FIXED_SIZE),
            Self::Message | Self::Contract => None,
        }
    }

    pub const fn contract_balance_root_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(INPUT_CONTRACT_BALANCE_ROOT_OFFSET),
            Self::Message | Self::Coin => None,
        }
    }

    pub const fn contract_state_root_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(INPUT_CONTRACT_STATE_ROOT_OFFSET),
            Self::Message | Self::Coin => None,
        }
    }

    pub const fn contract_id_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(INPUT_CONTRACT_ID_OFFSET),
            Self::Message | Self::Coin => None,
        }
    }

    pub const fn message_id_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_ID_OFFSET),
            Self::Contract | Self::Coin => None,
        }
    }

    pub const fn message_sender_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_SENDER_OFFSET),
            Self::Contract | Self::Coin => None,
        }
    }

    pub const fn message_recipient_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_RECIPIENT_OFFSET),
            Self::Contract | Self::Coin => None,
        }
    }

    pub const fn tx_pointer_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_TX_POINTER_OFFSET),
            Self::Contract => Some(INPUT_CONTRACT_TX_POINTER_OFFSET),
            Self::Message => None,
        }
    }

    pub const fn from_input(input: &Input) -> Self {
        match input {
            Input::CoinSigned { .. } | Input::CoinPredicate { .. } => InputRepr::Coin,
            Input::Contract { .. } => InputRepr::Contract,
            Input::MessageSigned { .. } | Input::MessagePredicate { .. } => InputRepr::Message,
        }
    }
}

impl From<&Input> for InputRepr {
    fn from(input: &Input) -> Self {
        Self::from_input(input)
    }
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
