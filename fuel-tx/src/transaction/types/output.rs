use fuel_crypto::Hasher;
use fuel_types::bytes::{self, WORD_SIZE};
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId, Word};

#[cfg(feature = "std")]
use fuel_types::bytes::SizedBytes;

#[cfg(feature = "std")]
use std::io;

const OUTPUT_COIN_SIZE: usize = WORD_SIZE // Identifier
    + Address::LEN // To
    + WORD_SIZE // Amount
    + AssetId::LEN; // AssetId

const OUTPUT_MESSAGE_SIZE: usize = WORD_SIZE // Identifier
    + Address::LEN // Recipient
    + WORD_SIZE; // Amount

const OUTPUT_CONTRACT_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Input index
    + Bytes32::LEN // Balance root
    + Bytes32::LEN; // State root

const OUTPUT_CONTRACT_CREATED_SIZE: usize = WORD_SIZE // Identifier
    + ContractId::LEN // Contract Id
    + Bytes32::LEN;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OutputRepr {
    Coin = 0x00,
    Contract = 0x01,
    Message = 0x02,
    Change = 0x03,
    Variable = 0x04,
    ContractCreated = 0x05,
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

impl From<&mut Output> for OutputRepr {
    fn from(o: &mut Output) -> Self {
        match o {
            Output::Coin { .. } => Self::Coin,
            Output::Contract { .. } => Self::Contract,
            Output::Message { .. } => Self::Message,
            Output::Change { .. } => Self::Change,
            Output::Variable { .. } => Self::Variable,
            Output::ContractCreated { .. } => Self::ContractCreated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    Message {
        recipient: Address,
        amount: Word,
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
            Self::Coin { .. } | Self::Change { .. } | Self::Variable { .. } => OUTPUT_COIN_SIZE,

            Self::Message { .. } => OUTPUT_MESSAGE_SIZE,

            Self::Contract { .. } => OUTPUT_CONTRACT_SIZE,

            Self::ContractCreated { .. } => OUTPUT_CONTRACT_CREATED_SIZE,
        }
    }
}

impl Output {
    pub const fn coin(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Coin {
            to,
            amount,
            asset_id,
        }
    }

    pub const fn contract(input_index: u8, balance_root: Bytes32, state_root: Bytes32) -> Self {
        Self::Contract {
            input_index,
            balance_root,
            state_root,
        }
    }

    pub const fn message(recipient: Address, amount: Word) -> Self {
        Self::Message { recipient, amount }
    }

    pub const fn change(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Change {
            to,
            amount,
            asset_id,
        }
    }

    pub const fn variable(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Variable {
            to,
            amount,
            asset_id,
        }
    }

    pub const fn contract_created(contract_id: ContractId, state_root: Bytes32) -> Self {
        Self::ContractCreated {
            contract_id,
            state_root,
        }
    }

    pub const fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Self::Coin { asset_id, .. } => Some(asset_id),
            Self::Change { asset_id, .. } => Some(asset_id),
            Self::Variable { asset_id, .. } => Some(asset_id),
            _ => None,
        }
    }

    pub fn message_id(
        sender: &Address,
        recipient: &Address,
        nonce: &Bytes32,
        amount: Word,
        data: &[u8],
    ) -> MessageId {
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

    /// Prepare the output for VM initialization
    pub fn prepare_init(&mut self) {
        const ZERO_MESSAGE: Output = Output::message(Address::zeroed(), 0);
        const ZERO_VARIABLE: Output = Output::variable(Address::zeroed(), 0, AssetId::zeroed());

        match self {
            Output::Message { .. } => *self = ZERO_MESSAGE,
            Output::Variable { .. } => *self = ZERO_VARIABLE,
            _ => (),
        }
    }
}

#[cfg(feature = "std")]
impl io::Read for Output {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let identifier: OutputRepr = self.into();
        buf = bytes::store_number_unchecked(buf, identifier as Word);

        match self {
            Self::Coin {
                to,
                amount,
                asset_id,
            }
            | Self::Change {
                to,
                amount,
                asset_id,
            }
            | Self::Variable {
                to,
                amount,
                asset_id,
            } => {
                buf = bytes::store_array_unchecked(buf, to);
                buf = bytes::store_number_unchecked(buf, *amount);

                bytes::store_array_unchecked(buf, asset_id);
            }

            Self::Message { recipient, amount } => {
                buf = bytes::store_array_unchecked(buf, recipient);

                bytes::store_number_unchecked(buf, *amount);
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

            Self::ContractCreated {
                contract_id,
                state_root,
            } => {
                buf = bytes::store_array_unchecked(buf, contract_id);

                bytes::store_array_unchecked(buf, state_root);
            }
        }

        Ok(n)
    }
}

#[cfg(feature = "std")]
impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        // Bounds safely checked
        let (identifier, buf): (Word, _) = unsafe { bytes::restore_number_unchecked(buf) };
        let identifier = OutputRepr::try_from(identifier)?;

        match identifier {
            OutputRepr::Coin | OutputRepr::Change | OutputRepr::Variable
                if buf.len() < OUTPUT_COIN_SIZE - WORD_SIZE =>
            {
                Err(bytes::eof())
            }

            OutputRepr::Message if buf.len() < OUTPUT_MESSAGE_SIZE - WORD_SIZE => Err(bytes::eof()),

            OutputRepr::Contract if buf.len() < OUTPUT_CONTRACT_SIZE - WORD_SIZE => {
                Err(bytes::eof())
            }

            OutputRepr::ContractCreated if buf.len() < OUTPUT_CONTRACT_CREATED_SIZE - WORD_SIZE => {
                Err(bytes::eof())
            }

            OutputRepr::Coin | OutputRepr::Change | OutputRepr::Variable => {
                // Safety: buf len is checked
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (asset_id, _) = unsafe { bytes::restore_array_unchecked(buf) };

                let to = to.into();
                let asset_id = asset_id.into();

                match identifier {
                    OutputRepr::Coin => {
                        *self = Self::Coin {
                            to,
                            amount,
                            asset_id,
                        }
                    }
                    OutputRepr::Change => {
                        *self = Self::Change {
                            to,
                            amount,
                            asset_id,
                        }
                    }
                    OutputRepr::Variable => {
                        *self = Self::Variable {
                            to,
                            amount,
                            asset_id,
                        }
                    }

                    _ => unreachable!(),
                }

                Ok(OUTPUT_COIN_SIZE)
            }

            OutputRepr::Message => {
                // Safety: buf len is checked
                let (recipient, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, _) = unsafe { bytes::restore_number_unchecked(buf) };

                let recipient = recipient.into();

                *self = Self::Message { recipient, amount };

                Ok(OUTPUT_MESSAGE_SIZE)
            }

            OutputRepr::Contract => {
                // Safety: buf len is checked
                let (input_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
                let (balance_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (state_root, _) = unsafe { bytes::restore_array_unchecked(buf) };

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
                // Safety: buf len is checked
                let (contract_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (state_root, _) = unsafe { bytes::restore_array_unchecked(buf) };

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
