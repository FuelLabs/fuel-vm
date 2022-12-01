use super::consts::*;
use super::Output;

#[cfg(feature = "std")]
use fuel_types::Word;

#[cfg(feature = "std")]
use std::io;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputRepr {
    Coin = 0x00,
    Contract = 0x01,
    Message = 0x02,
    Change = 0x03,
    Variable = 0x04,
    ContractCreated = 0x05,
}

impl OutputRepr {
    pub const fn to_offset(&self) -> Option<usize> {
        match self {
            OutputRepr::Coin | OutputRepr::Change | OutputRepr::Variable => {
                Some(OUTPUT_CCV_TO_OFFSET)
            }
            _ => None,
        }
    }

    pub const fn asset_id_offset(&self) -> Option<usize> {
        match self {
            OutputRepr::Coin | OutputRepr::Change | OutputRepr::Variable => {
                Some(OUTPUT_CCV_ASSET_ID_OFFSET)
            }
            _ => None,
        }
    }

    pub const fn contract_balance_root_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(OUTPUT_CONTRACT_BALANCE_ROOT_OFFSET),
            _ => None,
        }
    }

    pub const fn contract_state_root_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(OUTPUT_CONTRACT_STATE_ROOT_OFFSET),
            _ => None,
        }
    }

    pub const fn contract_created_state_root_offset(&self) -> Option<usize> {
        match self {
            Self::ContractCreated => Some(OUTPUT_CONTRACT_CREATED_STATE_ROOT_OFFSET),
            _ => None,
        }
    }

    pub const fn contract_id_offset(&self) -> Option<usize> {
        match self {
            Self::ContractCreated => Some(OUTPUT_CONTRACT_CREATED_ID_OFFSET),
            _ => None,
        }
    }

    pub const fn recipient_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(OUTPUT_MESSAGE_RECIPIENT_OFFSET),
            _ => None,
        }
    }

    pub const fn from_output(output: &Output) -> Self {
        match output {
            Output::Coin { .. } => Self::Coin,
            Output::Contract { .. } => Self::Contract,
            Output::Message { .. } => Self::Message,
            Output::Change { .. } => Self::Change,
            Output::Variable { .. } => Self::Variable,
            Output::ContractCreated { .. } => Self::ContractCreated,
        }
    }
}

#[cfg(feature = "std")]
impl TryFrom<Word> for OutputRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Coin),
            0x01 => Ok(Self::Contract),
            0x02 => Ok(Self::Message),
            0x03 => Ok(Self::Change),
            0x04 => Ok(Self::Variable),
            0x05 => Ok(Self::ContractCreated),
            i => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("The provided output identifier ({}) is invalid!", i),
            )),
        }
    }
}

impl From<&Output> for OutputRepr {
    fn from(o: &Output) -> Self {
        Self::from_output(o)
    }
}

impl From<&mut Output> for OutputRepr {
    fn from(o: &mut Output) -> Self {
        Self::from_output(o)
    }
}
