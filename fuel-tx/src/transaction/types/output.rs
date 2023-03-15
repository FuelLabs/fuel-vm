use fuel_crypto::Hasher;
use fuel_types::{bytes, MemLayout, MemLocType};
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId, Word};

use core::mem;

#[cfg(feature = "std")]
use fuel_types::bytes::{SizedBytes, WORD_SIZE};

#[cfg(feature = "std")]
use std::io;

mod consts;
mod repr;
mod sizes;

use consts::*;
use sizes::*;

pub use repr::OutputRepr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum_macros::EnumCount)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Output {
    Coin {
        to: Address,
        amount: Word,
        asset_id: AssetId,
    },

    Contract {
        input_index: u8,
        balance_root: Bytes32,
        state_root: Bytes32,
    },

    Change {
        to: Address,
        amount: Word,
        asset_id: AssetId,
    },

    Variable {
        to: Address,
        amount: Word,
        asset_id: AssetId,
    },

    ContractCreated {
        contract_id: ContractId,
        state_root: Bytes32,
    },
}

impl Default for Output {
    fn default() -> Self {
        Self::ContractCreated {
            contract_id: Default::default(),
            state_root: Default::default(),
        }
    }
}

impl bytes::SizedBytes for Output {
    fn serialized_size(&self) -> usize {
        match self {
            Self::Coin { .. } | Self::Change { .. } | Self::Variable { .. } => OUTPUT_CCV_SIZE,

            Self::Contract { .. } => OUTPUT_CONTRACT_SIZE,

            Self::ContractCreated { .. } => OUTPUT_CONTRACT_CREATED_SIZE,
        }
    }
}

impl Output {
    pub const fn repr(&self) -> OutputRepr {
        OutputRepr::from_output(self)
    }

    pub const fn coin(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Coin { to, amount, asset_id }
    }

    pub const fn contract(input_index: u8, balance_root: Bytes32, state_root: Bytes32) -> Self {
        Self::Contract {
            input_index,
            balance_root,
            state_root,
        }
    }

    pub const fn change(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Change { to, amount, asset_id }
    }

    pub const fn variable(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Variable { to, amount, asset_id }
    }

    pub const fn contract_created(contract_id: ContractId, state_root: Bytes32) -> Self {
        Self::ContractCreated {
            contract_id,
            state_root,
        }
    }

    pub const fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Output::Coin { asset_id, .. } | Output::Change { asset_id, .. } | Output::Variable { asset_id, .. } => {
                Some(asset_id)
            }
            _ => None,
        }
    }

    pub const fn to(&self) -> Option<&Address> {
        match self {
            Output::Coin { to, .. } | Output::Change { to, .. } | Output::Variable { to, .. } => Some(to),
            _ => None,
        }
    }

    pub const fn amount(&self) -> Option<Word> {
        match self {
            Output::Coin { amount, .. } | Output::Change { amount, .. } | Output::Variable { amount, .. } => {
                Some(*amount)
            }
            _ => None,
        }
    }

    pub const fn input_index(&self) -> Option<u8> {
        match self {
            Output::Contract { input_index, .. } => Some(*input_index),
            _ => None,
        }
    }

    pub const fn balance_root(&self) -> Option<&Bytes32> {
        match self {
            Output::Contract { balance_root, .. } => Some(balance_root),
            _ => None,
        }
    }

    pub const fn state_root(&self) -> Option<&Bytes32> {
        match self {
            Output::Contract { state_root, .. } | Output::ContractCreated { state_root, .. } => Some(state_root),
            _ => None,
        }
    }

    pub const fn contract_id(&self) -> Option<&ContractId> {
        match self {
            Output::ContractCreated { contract_id, .. } => Some(contract_id),
            _ => None,
        }
    }

    pub const fn is_coin(&self) -> bool {
        matches!(self, Self::Coin { .. })
    }

    pub const fn is_variable(&self) -> bool {
        matches!(self, Self::Variable { .. })
    }

    pub const fn is_contract(&self) -> bool {
        matches!(self, Self::Contract { .. })
    }

    pub const fn is_contract_created(&self) -> bool {
        matches!(self, Self::ContractCreated { .. })
    }

    pub fn message_id(sender: &Address, recipient: &Address, nonce: &Bytes32, amount: Word, data: &[u8]) -> MessageId {
        let message_id = *Hasher::default()
            .chain(sender)
            .chain(recipient)
            .chain(nonce)
            .chain(amount.to_be_bytes())
            .chain(data)
            .finalize();

        message_id.into()
    }

    pub fn message_nonce(txid: &Bytes32, idx: Word) -> Bytes32 {
        Hasher::default().chain(txid).chain([idx as u8]).finalize()
    }

    pub fn message_digest(data: &[u8]) -> Bytes32 {
        Hasher::hash(data)
    }

    /// Empties fields that should be zero during the signing.
    pub(crate) fn prepare_sign(&mut self) {
        match self {
            Output::Contract {
                balance_root,
                state_root,
                ..
            } => {
                mem::take(balance_root);
                mem::take(state_root);
            }

            Output::Change { amount, .. } => {
                mem::take(amount);
            }

            Output::Variable {
                to, amount, asset_id, ..
            } => {
                mem::take(to);
                mem::take(amount);
                mem::take(asset_id);
            }

            _ => (),
        }
    }

    /// Prepare the output for VM initialization for script execution
    #[cfg(feature = "std")]
    pub fn prepare_init_script(&mut self) {
        match self {
            Output::Change { amount, .. } => {
                mem::take(amount);
            }

            Output::Variable { to, amount, asset_id } => {
                mem::take(to);
                mem::take(amount);
                mem::take(asset_id);
            }

            _ => (),
        }
    }

    /// Prepare the output for VM initialization for predicate verification
    pub fn prepare_init_predicate(&mut self) {
        self.prepare_sign()
    }
}

#[cfg(feature = "std")]
impl io::Read for Output {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let identifier: OutputRepr = self.into();

        match self {
            Self::Coin { to, amount, asset_id }
            | Self::Change { to, amount, asset_id }
            | Self::Variable { to, amount, asset_id } => {
                type S = CoinSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), identifier as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.to), to);
                bytes::store_number_at(buf, S::layout(S::LAYOUT.amount), *amount);

                bytes::store_at(buf, S::layout(S::LAYOUT.asset_id), asset_id);
            }

            Self::Contract {
                input_index,
                balance_root,
                state_root,
            } => {
                type S = ContractSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), identifier as u8);

                bytes::store_number_at(buf, S::layout(S::LAYOUT.input_index), *input_index);
                bytes::store_at(buf, S::layout(S::LAYOUT.balance_root), balance_root);

                bytes::store_at(buf, S::layout(S::LAYOUT.state_root), state_root);
            }

            Self::ContractCreated {
                contract_id,
                state_root,
            } => {
                type S = ContractCreatedSizes;
                let buf: &mut [_; S::LEN] = buf
                    .get_mut(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                bytes::store_number_at(buf, S::layout(S::LAYOUT.repr), identifier as u8);

                bytes::store_at(buf, S::layout(S::LAYOUT.contract_id), contract_id);

                bytes::store_at(buf, S::layout(S::LAYOUT.state_root), state_root);
            }
        }

        Ok(n)
    }
}

#[cfg(feature = "std")]
impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let identifier: &[_; WORD_SIZE] = buf
            .get(..WORD_SIZE)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let identifier = bytes::restore_word(bytes::from_array(identifier));
        let identifier = OutputRepr::try_from(identifier)?;

        match identifier {
            OutputRepr::Coin | OutputRepr::Change | OutputRepr::Variable if buf.len() < OUTPUT_CCV_SIZE - WORD_SIZE => {
                Err(bytes::eof())
            }

            OutputRepr::Contract if buf.len() < OUTPUT_CONTRACT_SIZE - WORD_SIZE => Err(bytes::eof()),

            OutputRepr::ContractCreated if buf.len() < OUTPUT_CONTRACT_CREATED_SIZE - WORD_SIZE => Err(bytes::eof()),

            OutputRepr::Coin | OutputRepr::Change | OutputRepr::Variable => {
                type S = CoinSizes;
                let buf: &[_; S::LEN] = buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let to = bytes::restore_at(buf, S::layout(S::LAYOUT.to));
                let amount = bytes::restore_number_at(buf, S::layout(S::LAYOUT.amount));
                let asset_id = bytes::restore_at(buf, S::layout(S::LAYOUT.asset_id));

                let to = to.into();
                let asset_id = asset_id.into();

                match identifier {
                    OutputRepr::Coin => *self = Self::Coin { to, amount, asset_id },
                    OutputRepr::Change => *self = Self::Change { to, amount, asset_id },
                    OutputRepr::Variable => *self = Self::Variable { to, amount, asset_id },

                    _ => unreachable!(),
                }

                Ok(OUTPUT_CCV_SIZE)
            }

            OutputRepr::Contract => {
                type S = ContractSizes;
                let buf: &[_; S::LEN] = buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let input_index = bytes::restore_u8_at(buf, S::layout(S::LAYOUT.input_index));
                let balance_root = bytes::restore_at(buf, S::layout(S::LAYOUT.balance_root));
                let state_root = bytes::restore_at(buf, S::layout(S::LAYOUT.state_root));

                let balance_root = balance_root.into();
                let state_root = state_root.into();

                *self = Self::Contract {
                    input_index,
                    balance_root,
                    state_root,
                };

                Ok(OUTPUT_CONTRACT_SIZE)
            }

            OutputRepr::ContractCreated => {
                type S = ContractCreatedSizes;
                let buf: &[_; S::LEN] = buf
                    .get(..S::LEN)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(bytes::eof())?;

                let contract_id = bytes::restore_at(buf, S::layout(S::LAYOUT.contract_id));
                let state_root = bytes::restore_at(buf, S::layout(S::LAYOUT.state_root));

                let contract_id = contract_id.into();
                let state_root = state_root.into();

                *self = Self::ContractCreated {
                    contract_id,
                    state_root,
                };

                Ok(OUTPUT_CONTRACT_CREATED_SIZE)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
