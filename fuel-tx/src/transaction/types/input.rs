use crate::{
    TxPointer,
    UtxoId,
};
use alloc::{
    string::ToString,
    vec::Vec,
};
use coin::*;
use consts::*;
use contract::*;
use core::{
    fmt,
    fmt::Formatter,
};
use fuel_crypto::{
    Hasher,
    PublicKey,
};
use fuel_types::{
    bytes,
    canonical::{
        Deserialize,
        Serialize,
        SerializedSizeFixed,
    },
    fmt_truncated_hex,
    Address,
    AssetId,
    BlockHeight,
    Bytes32,
    ChainId,
    ContractId,
    MessageId,
    Nonce,
    Word,
};
use message::*;
use num_enum::TryFromPrimitive;

pub mod coin;
mod consts;
pub mod contract;
pub mod message;
mod repr;

pub use repr::InputRepr;

#[cfg(all(test, feature = "std"))]
mod ser_de_tests;

pub trait AsField<Type>: AsFieldFmt {
    fn as_field(&self) -> Option<&Type>;

    fn as_mut_field(&mut self) -> Option<&mut Type>;
}

impl<Type> AsField<Type> for () {
    #[inline(always)]
    fn as_field(&self) -> Option<&Type> {
        None
    }

    fn as_mut_field(&mut self) -> Option<&mut Type> {
        None
    }
}

impl AsFieldFmt for () {
    fn fmt_as_field(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("()")
    }
}

impl AsField<u8> for u8 {
    #[inline(always)]
    fn as_field(&self) -> Option<&u8> {
        Some(self)
    }

    fn as_mut_field(&mut self) -> Option<&mut u8> {
        Some(self)
    }
}

impl AsFieldFmt for u8 {
    fn fmt_as_field(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

impl AsField<u64> for u64 {
    #[inline(always)]
    fn as_field(&self) -> Option<&u64> {
        Some(self)
    }

    fn as_mut_field(&mut self) -> Option<&mut u64> {
        Some(self)
    }
}

impl AsFieldFmt for u64 {
    fn fmt_as_field(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

impl AsField<Vec<u8>> for Vec<u8> {
    #[inline(always)]
    fn as_field(&self) -> Option<&Vec<u8>> {
        Some(self)
    }

    fn as_mut_field(&mut self) -> Option<&mut Vec<u8>> {
        Some(self)
    }
}

impl AsFieldFmt for Vec<u8> {
    fn fmt_as_field(&self, f: &mut Formatter) -> fmt::Result {
        fmt_truncated_hex::<16>(self, f)
    }
}

pub trait AsFieldFmt {
    fn fmt_as_field(&self, f: &mut Formatter) -> fmt::Result;
}

pub fn fmt_as_field<T>(field: &T, f: &mut Formatter) -> fmt::Result
where
    T: AsFieldFmt,
{
    field.fmt_as_field(f)
}

fn input_serialized_size_static_helper(value: &Input) -> usize {
    fuel_types::canonical::add_sizes(
        8, // Discriminant
        match value.clone() {
            Input::CoinSigned(_) => CoinFull::SIZE_STATIC,
            Input::CoinPredicate(_) => CoinFull::SIZE_STATIC,
            Input::Contract(_) => Contract::SIZE_STATIC,
            Input::MessageCoinSigned(_) => FullMessage::SIZE_STATIC,
            Input::MessageCoinPredicate(_) => FullMessage::SIZE_STATIC,
            Input::MessageDataSigned(_) => FullMessage::SIZE_STATIC,
            Input::MessageDataPredicate(_) => FullMessage::SIZE_STATIC,
        },
    )
}

fn input_serialized_size_dynamic_helper(value: &Input) -> usize {
    match value.clone() {
        Input::CoinSigned(coin) => coin.into_full().size_dynamic(),
        Input::CoinPredicate(coin) => coin.into_full().size_dynamic(),
        Input::Contract(contract) => contract.size_dynamic(),
        Input::MessageCoinSigned(message) => message.into_full().size_dynamic(),
        Input::MessageCoinPredicate(message) => message.into_full().size_dynamic(),
        Input::MessageDataSigned(message) => message.into_full().size_dynamic(),
        Input::MessageDataPredicate(message) => message.into_full().size_dynamic(),
    }
}

fn input_serialize_helper<O: fuel_types::canonical::Output + ?Sized>(
    value: &Input,
    output: &mut O,
) -> Result<(), fuel_types::canonical::Error> {
    let discr: u64 = InputRepr::from(value).into();
    discr.encode(output)?;
    match value.clone() {
        Input::CoinSigned(coin) => coin.into_full().encode(output),
        Input::CoinPredicate(coin) => coin.into_full().encode(output),
        Input::Contract(contract) => contract.encode(output),
        Input::MessageCoinSigned(message) => message.into_full().encode(output),
        Input::MessageCoinPredicate(message) => message.into_full().encode(output),
        Input::MessageDataSigned(message) => message.into_full().encode(output),
        Input::MessageDataPredicate(message) => message.into_full().encode(output),
    }
}

fn input_deserialize_helper<I: fuel_types::canonical::Input + ?Sized>(
    discr: u64,
    data: &mut I,
) -> Result<Input, fuel_types::canonical::Error> {
    Ok(
        match InputRepr::try_from_primitive(discr)
            .map_err(|_| fuel_types::canonical::Error::UnknownDiscriminant)?
        {
            InputRepr::Coin => {
                let coin = CoinFull::decode(data)?;
                if coin.predicate.is_empty() {
                    Input::CoinSigned(coin.into_signed())
                } else {
                    Input::CoinPredicate(coin.into_predicate())
                }
            }
            InputRepr::Contract => {
                let contract = Contract::decode(data)?;
                Input::Contract(contract)
            }
            InputRepr::Message => {
                let message = FullMessage::decode(data)?;
                match (message.data.is_empty(), message.predicate.is_empty()) {
                    (true, true) => Input::MessageCoinSigned(message.into_coin_signed()),
                    (true, false) => {
                        Input::MessageCoinPredicate(message.into_coin_predicate())
                    }
                    (false, true) => {
                        Input::MessageDataSigned(message.into_message_data_signed())
                    }
                    (false, false) => {
                        Input::MessageDataPredicate(message.into_message_data_predicate())
                    }
                }
            }
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, strum_macros::EnumCount)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Deserialize, Serialize)]
#[canonical(serialized_size_static_with = input_serialized_size_static_helper)]
#[canonical(serialized_size_dynamic_with = input_serialized_size_dynamic_helper)]
#[canonical(serialize_with = input_serialize_helper)]
#[canonical(deserialize_with = input_deserialize_helper)]
#[canonical(SIZE_NO_DYNAMIC = CoinFull::SIZE_NO_DYNAMIC && Contract::SIZE_NO_DYNAMIC && FullMessage::SIZE_NO_DYNAMIC)]
pub enum Input {
    CoinSigned(CoinSigned),
    CoinPredicate(CoinPredicate),
    Contract(Contract),
    MessageCoinSigned(MessageCoinSigned),
    MessageCoinPredicate(MessageCoinPredicate),
    MessageDataSigned(MessageDataSigned),
    MessageDataPredicate(MessageDataPredicate),
}

impl Default for Input {
    fn default() -> Self {
        Self::contract(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
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
        maturity: BlockHeight,
        predicate_gas_used: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::CoinPredicate(CoinPredicate {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index: (),
            maturity,
            predicate_gas_used,
            predicate,
            predicate_data,
        })
    }

    pub const fn coin_signed(
        utxo_id: UtxoId,
        owner: Address,
        amount: Word,
        asset_id: AssetId,
        tx_pointer: TxPointer,
        witness_index: u8,
        maturity: BlockHeight,
    ) -> Self {
        Self::CoinSigned(CoinSigned {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            maturity,
            predicate_gas_used: (),
            predicate: (),
            predicate_data: (),
        })
    }

    pub const fn contract(
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        tx_pointer: TxPointer,
        contract_id: ContractId,
    ) -> Self {
        Self::Contract(Contract {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        })
    }

    pub const fn message_coin_signed(
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Nonce,
        witness_index: u8,
    ) -> Self {
        Self::MessageCoinSigned(MessageCoinSigned {
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            predicate_gas_used: (),
            data: (),
            predicate: (),
            predicate_data: (),
        })
    }

    pub const fn message_coin_predicate(
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Nonce,
        predicate_gas_used: Word,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::MessageCoinPredicate(MessageCoinPredicate {
            sender,
            recipient,
            amount,
            nonce,
            witness_index: (),
            predicate_gas_used,
            data: (),
            predicate,
            predicate_data,
        })
    }

    pub const fn message_data_signed(
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Nonce,
        witness_index: u8,
        data: Vec<u8>,
    ) -> Self {
        Self::MessageDataSigned(MessageDataSigned {
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
            predicate: (),
            predicate_data: (),
            predicate_gas_used: (),
        })
    }

    pub const fn message_data_predicate(
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Nonce,
        predicate_gas_used: Word,
        data: Vec<u8>,
        predicate: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Self {
        Self::MessageDataPredicate(MessageDataPredicate {
            sender,
            recipient,
            amount,
            nonce,
            witness_index: (),
            predicate_gas_used,
            data,
            predicate,
            predicate_data,
        })
    }

    pub const fn utxo_id(&self) -> Option<&UtxoId> {
        match self {
            Self::CoinSigned(CoinSigned { utxo_id, .. })
            | Self::CoinPredicate(CoinPredicate { utxo_id, .. })
            | Self::Contract(Contract { utxo_id, .. }) => Some(utxo_id),
            Self::MessageCoinSigned(_) => None,
            Self::MessageCoinPredicate(_) => None,
            Self::MessageDataSigned(_) => None,
            Self::MessageDataPredicate(_) => None,
        }
    }

    pub const fn input_owner(&self) -> Option<&Address> {
        match self {
            Self::CoinSigned(CoinSigned { owner, .. })
            | Self::CoinPredicate(CoinPredicate { owner, .. }) => Some(owner),
            Self::MessageCoinSigned(_)
            | Self::MessageCoinPredicate(_)
            | Self::MessageDataSigned(_)
            | Self::MessageDataPredicate(_)
            | Self::Contract(_) => None,
        }
    }

    pub const fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Input::CoinSigned(CoinSigned { asset_id, .. })
            | Input::CoinPredicate(CoinPredicate { asset_id, .. }) => Some(asset_id),
            Input::MessageCoinSigned(_)
            | Input::MessageCoinPredicate(_)
            | Input::MessageDataSigned(_)
            | Input::MessageDataPredicate(_) => Some(&AssetId::BASE),
            Input::Contract(_) => None,
        }
    }

    pub const fn contract_id(&self) -> Option<&ContractId> {
        match self {
            Self::Contract(Contract { contract_id, .. }) => Some(contract_id),
            _ => None,
        }
    }

    pub const fn amount(&self) -> Option<Word> {
        match self {
            Input::CoinSigned(CoinSigned { amount, .. })
            | Input::CoinPredicate(CoinPredicate { amount, .. })
            | Input::MessageCoinSigned(MessageCoinSigned { amount, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { amount, .. })
            | Input::MessageDataSigned(MessageDataSigned { amount, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { amount, .. }) => {
                Some(*amount)
            }
            Input::Contract(_) => None,
        }
    }

    pub const fn witness_index(&self) -> Option<u8> {
        match self {
            Input::CoinSigned(CoinSigned { witness_index, .. })
            | Input::MessageCoinSigned(MessageCoinSigned { witness_index, .. })
            | Input::MessageDataSigned(MessageDataSigned { witness_index, .. }) => {
                Some(*witness_index)
            }
            Input::CoinPredicate(_)
            | Input::Contract(_)
            | Input::MessageCoinPredicate(_)
            | Input::MessageDataPredicate(_) => None,
        }
    }

    pub const fn maturity(&self) -> Option<BlockHeight> {
        match self {
            Input::CoinSigned(CoinSigned { maturity, .. })
            | Input::CoinPredicate(CoinPredicate { maturity, .. }) => Some(*maturity),
            Input::Contract(_)
            | Input::MessageCoinSigned(_)
            | Input::MessageCoinPredicate(_)
            | Input::MessageDataSigned(_)
            | Input::MessageDataPredicate(_) => None,
        }
    }

    pub fn predicate_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(_) => InputRepr::Coin.coin_predicate_offset(),
            Input::MessageCoinPredicate(_) => InputRepr::Message.data_offset(),
            Input::MessageDataPredicate(MessageDataPredicate { data, .. }) => {
                InputRepr::Message
                    .data_offset()
                    .map(|o| o + bytes::padded_len(data))
            }
            Input::CoinSigned(_)
            | Input::Contract(_)
            | Input::MessageCoinSigned(_)
            | Input::MessageDataSigned(_) => None,
        }
    }

    pub fn predicate_data_offset(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { predicate, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { predicate, .. }) => self
                .predicate_offset()
                .map(|o| o + bytes::padded_len(predicate)),
            Input::CoinSigned(_)
            | Input::Contract(_)
            | Input::MessageCoinSigned(_)
            | Input::MessageDataSigned(_) => None,
        }
    }

    pub fn predicate_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { predicate, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { predicate, .. }) => {
                Some(predicate.len())
            }
            Input::CoinSigned(_)
            | Input::MessageCoinSigned(_)
            | Input::MessageDataSigned(_) => Some(0),
            Input::Contract(_) => None,
        }
    }

    pub fn predicate_data_len(&self) -> Option<usize> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate_data, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate {
                predicate_data, ..
            })
            | Input::MessageDataPredicate(MessageDataPredicate {
                predicate_data, ..
            }) => Some(predicate_data.len()),
            Input::CoinSigned(_)
            | Input::MessageCoinSigned(_)
            | Input::MessageDataSigned(_) => Some(0),
            Input::Contract(_) => None,
        }
    }

    pub fn predicate_gas_used(&self) -> Option<Word> {
        match self {
            Input::CoinPredicate(CoinPredicate {
                predicate_gas_used, ..
            })
            | Input::MessageCoinPredicate(MessageCoinPredicate {
                predicate_gas_used,
                ..
            })
            | Input::MessageDataPredicate(MessageDataPredicate {
                predicate_gas_used,
                ..
            }) => Some(*predicate_gas_used),
            Input::CoinSigned(_)
            | Input::MessageCoinSigned(_)
            | Input::MessageDataSigned(_)
            | Input::Contract(_) => None,
        }
    }

    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            Self::MessageCoinSigned(message) => Some(message.message_id()),
            Self::MessageCoinPredicate(message) => Some(message.message_id()),
            Self::MessageDataPredicate(message) => Some(message.message_id()),
            Self::MessageDataSigned(message) => Some(message.message_id()),
            _ => None,
        }
    }

    pub const fn tx_pointer(&self) -> Option<&TxPointer> {
        match self {
            Input::CoinSigned(CoinSigned { tx_pointer, .. })
            | Input::CoinPredicate(CoinPredicate { tx_pointer, .. })
            | Input::Contract(Contract { tx_pointer, .. }) => Some(tx_pointer),
            _ => None,
        }
    }

    pub fn input_data(&self) -> Option<&[u8]> {
        match self {
            Input::MessageDataSigned(MessageDataSigned { data, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { data, .. }) => {
                Some(data)
            }
            _ => None,
        }
    }

    pub fn input_data_len(&self) -> Option<usize> {
        match self {
            Input::MessageDataSigned(MessageDataSigned { data, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { data, .. }) => {
                Some(data.len())
            }
            Input::MessageCoinSigned(_) | Input::MessageCoinPredicate(_) => Some(0),
            _ => None,
        }
    }

    pub fn input_predicate(&self) -> Option<&[u8]> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { predicate, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { predicate, .. }) => {
                Some(predicate)
            }

            _ => None,
        }
    }

    pub fn input_predicate_data(&self) -> Option<&[u8]> {
        match self {
            Input::CoinPredicate(CoinPredicate { predicate_data, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate {
                predicate_data, ..
            })
            | Input::MessageDataPredicate(MessageDataPredicate {
                predicate_data, ..
            }) => Some(predicate_data),

            _ => None,
        }
    }

    /// Return a tuple containing the predicate, its data and used gas if the input is of
    /// type `CoinPredicate` or `MessageCoinPredicate` or `MessageDataPredicate`
    pub fn predicate(&self) -> Option<(&[u8], &[u8], &Word)> {
        match self {
            Input::CoinPredicate(CoinPredicate {
                predicate,
                predicate_data,
                predicate_gas_used,
                ..
            })
            | Input::MessageCoinPredicate(MessageCoinPredicate {
                predicate,
                predicate_data,
                predicate_gas_used,
                ..
            })
            | Input::MessageDataPredicate(MessageDataPredicate {
                predicate,
                predicate_data,
                predicate_gas_used,
                ..
            }) => Some((
                predicate.as_slice(),
                predicate_data.as_slice(),
                predicate_gas_used,
            )),

            _ => None,
        }
    }

    pub const fn is_coin(&self) -> bool {
        self.is_coin_signed() | self.is_coin_predicate()
    }

    pub const fn is_coin_signed(&self) -> bool {
        matches!(self, Input::CoinSigned(_))
    }

    pub const fn is_coin_predicate(&self) -> bool {
        matches!(self, Input::CoinPredicate(_))
    }

    pub const fn is_message(&self) -> bool {
        self.is_message_coin_signed()
            | self.is_message_coin_predicate()
            | self.is_message_data_signed()
            | self.is_message_data_predicate()
    }

    pub const fn is_message_coin_signed(&self) -> bool {
        matches!(self, Input::MessageCoinSigned(_))
    }

    pub const fn is_message_coin_predicate(&self) -> bool {
        matches!(self, Input::MessageCoinPredicate(_))
    }

    pub const fn is_message_data_signed(&self) -> bool {
        matches!(self, Input::MessageDataSigned(_))
    }

    pub const fn is_message_data_predicate(&self) -> bool {
        matches!(self, Input::MessageDataPredicate(_))
    }

    pub const fn is_contract(&self) -> bool {
        matches!(self, Input::Contract(_))
    }

    pub const fn coin_predicate_offset() -> usize {
        INPUT_COIN_FIXED_SIZE
    }

    pub const fn message_data_offset() -> usize {
        INPUT_MESSAGE_FIXED_SIZE
    }

    pub const fn balance_root(&self) -> Option<&Bytes32> {
        match self {
            Input::Contract(Contract { balance_root, .. }) => Some(balance_root),
            _ => None,
        }
    }

    pub const fn state_root(&self) -> Option<&Bytes32> {
        match self {
            Input::Contract(Contract { state_root, .. }) => Some(state_root),
            _ => None,
        }
    }

    pub const fn sender(&self) -> Option<&Address> {
        match self {
            Input::MessageCoinSigned(MessageCoinSigned { sender, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { sender, .. })
            | Input::MessageDataSigned(MessageDataSigned { sender, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { sender, .. }) => {
                Some(sender)
            }
            _ => None,
        }
    }

    pub const fn recipient(&self) -> Option<&Address> {
        match self {
            Input::MessageCoinSigned(MessageCoinSigned { recipient, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { recipient, .. })
            | Input::MessageDataSigned(MessageDataSigned { recipient, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { recipient, .. }) => {
                Some(recipient)
            }
            _ => None,
        }
    }

    pub const fn nonce(&self) -> Option<&Nonce> {
        match self {
            Input::MessageCoinSigned(MessageCoinSigned { nonce, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { nonce, .. })
            | Input::MessageDataSigned(MessageDataSigned { nonce, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { nonce, .. }) => {
                Some(nonce)
            }
            _ => None,
        }
    }

    /// Empties fields that should be zero during the signing.
    pub(crate) fn prepare_sign(&mut self) {
        match self {
            Input::CoinSigned(coin) => coin.prepare_sign(),
            Input::CoinPredicate(coin) => coin.prepare_sign(),
            Input::Contract(contract) => contract.prepare_sign(),
            Input::MessageCoinSigned(message) => message.prepare_sign(),
            Input::MessageCoinPredicate(message) => message.prepare_sign(),
            Input::MessageDataSigned(message) => message.prepare_sign(),
            Input::MessageDataPredicate(message) => message.prepare_sign(),
        }
    }

    pub fn compute_message_id(
        sender: &Address,
        recipient: &Address,
        nonce: &Nonce,
        amount: Word,
        data: &[u8],
    ) -> MessageId {
        compute_message_id(sender, recipient, nonce, amount, data)
    }

    pub fn predicate_owner<P>(predicate: P, chain_id: &ChainId) -> Address
    where
        P: AsRef<[u8]>,
    {
        use crate::Contract;

        let root = Contract::root_from_code(predicate);

        let mut hasher = Hasher::default();

        hasher.input(ContractId::SEED);
        hasher.input(chain_id.to_be_bytes());
        hasher.input(root);

        (*hasher.digest()).into()
    }

    #[cfg(feature = "std")]
    pub fn is_predicate_owner_valid<P>(
        owner: &Address,
        predicate: P,
        chain_id: &ChainId,
    ) -> bool
    where
        P: AsRef<[u8]>,
    {
        owner == &Self::predicate_owner(predicate, chain_id)
    }

    /// Prepare the output for VM predicate execution
    pub fn prepare_init_predicate(&mut self) {
        self.prepare_sign()
    }
}

#[cfg(all(test, feature = "std"))]
mod snapshot_tests;
