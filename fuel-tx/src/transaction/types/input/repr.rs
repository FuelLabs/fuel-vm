use super::{
    Input,
    consts::*,
};

#[cfg(feature = "chargeable-tx-v2")]
use crate::input::InputV2;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    fuel_types::canonical::Serialize,
    fuel_types::canonical::Deserialize,
)]
#[repr(u64)]
pub enum InputRepr {
    Coin = 0x00,
    Contract = 0x01,
    Message = 0x02,
    #[cfg(feature = "chargeable-tx-v2")]
    CoinV2 = 0x03,
}

impl InputRepr {
    pub const fn utxo_id_offset(&self) -> Option<usize> {
        match self {
            Self::Coin | Self::Contract => Some(INPUT_UTXO_ID_OFFSET),
            Self::Message => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => Some(INPUT_UTXO_ID_OFFSET),
        }
    }

    pub const fn owner_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_OWNER_OFFSET),
            Self::Message => Some(INPUT_MESSAGE_RECIPIENT_OFFSET),
            Self::Contract => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => Some(INPUT_COIN_OWNER_OFFSET),
        }
    }

    pub const fn asset_id_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_ASSET_ID_OFFSET),
            Self::Message | Self::Contract => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => Some(INPUT_COIN_ASSET_ID_OFFSET),
        }
    }

    pub const fn data_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_FIXED_SIZE),
            Self::Coin | Self::Contract => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn coin_predicate_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_FIXED_SIZE),
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => Some(INPUT_COIN_V2_FIXED_SIZE),
            Self::Message | Self::Contract => None,
        }
    }

    pub const fn contract_balance_root_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(INPUT_CONTRACT_BALANCE_ROOT_OFFSET),
            Self::Message | Self::Coin => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn contract_state_root_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(INPUT_CONTRACT_STATE_ROOT_OFFSET),
            Self::Message | Self::Coin => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn contract_id_offset(&self) -> Option<usize> {
        match self {
            Self::Contract => Some(INPUT_CONTRACT_ID_OFFSET),
            Self::Message | Self::Coin => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn message_sender_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_SENDER_OFFSET),
            Self::Contract | Self::Coin => None,

            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn message_recipient_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_MESSAGE_RECIPIENT_OFFSET),
            Self::Contract | Self::Coin => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn message_nonce_offset(&self) -> Option<usize> {
        match self {
            Self::Message => Some(INPUT_NONCE_RECIPIENT_OFFSET),
            Self::Contract | Self::Coin => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => None,
        }
    }

    pub const fn tx_pointer_offset(&self) -> Option<usize> {
        match self {
            Self::Coin => Some(INPUT_COIN_TX_POINTER_OFFSET),
            Self::Contract => Some(INPUT_CONTRACT_TX_POINTER_OFFSET),
            Self::Message => None,
            #[cfg(feature = "chargeable-tx-v2")]
            Self::CoinV2 => Some(INPUT_COIN_TX_POINTER_OFFSET),
        }
    }

    pub const fn from_input(input: &Input) -> Self {
        match input {
            Input::CoinSigned(_) | Input::CoinPredicate(_) => InputRepr::Coin,
            Input::Contract(_) => InputRepr::Contract,
            Input::MessageCoinSigned(_)
            | Input::MessageCoinPredicate(_)
            | Input::MessageDataSigned(_)
            | Input::MessageDataPredicate(_) => InputRepr::Message,
            #[cfg(feature = "chargeable-tx-v2")]
            Input::InputV2(inner) => match inner {
                InputV2::Coin(_) => InputRepr::CoinV2,
                InputV2::Contract(_) => InputRepr::Contract,
                InputV2::Message(_) => InputRepr::Message,
            },
        }
    }
}

impl From<&Input> for InputRepr {
    fn from(input: &Input) -> Self {
        Self::from_input(input)
    }
}
