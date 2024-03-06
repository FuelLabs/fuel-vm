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
    canonical,
    canonical::{
        Deserialize,
        Error,
        Output,
        Serialize,
    },
    fmt_truncated_hex,
    Address,
    AssetId,
    Bytes32,
    ContractId,
    MessageId,
    Nonce,
    Word,
};
use message::*;

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

/// The empty field used by sub-types of the specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Empty<Type>(::core::marker::PhantomData<Type>);

impl<Type> Empty<Type> {
    /// Creates `Self`.
    pub const fn new() -> Self {
        Self(::core::marker::PhantomData {})
    }
}

impl<Type> Default for Empty<Type> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Type: Serialize + Default> Serialize for Empty<Type> {
    #[inline(always)]
    fn size_static(&self) -> usize {
        Type::default().size_static()
    }

    #[inline(always)]
    fn size_dynamic(&self) -> usize {
        0
    }

    #[inline(always)]
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        Type::default().encode_static(buffer)
    }
}

impl<Type: Deserialize> Deserialize for Empty<Type> {
    #[inline(always)]
    fn decode_static<I: canonical::Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<Self, Error> {
        Type::decode_static(buffer)?;
        Ok(Default::default())
    }
}

impl<Type> AsFieldFmt for Empty<Type> {
    fn fmt_as_field(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Empty")
    }
}

impl<Type> AsField<Type> for Empty<Type> {
    #[inline(always)]
    fn as_field(&self) -> Option<&Type> {
        None
    }

    fn as_mut_field(&mut self) -> Option<&mut Type> {
        None
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, strum_macros::EnumCount)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            witness_index: Empty::new(),
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
    ) -> Self {
        Self::CoinSigned(CoinSigned {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            predicate_gas_used: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
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
            predicate_gas_used: Empty::new(),
            data: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
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
            witness_index: Empty::new(),
            predicate_gas_used,
            data: Empty::new(),
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
            predicate: Empty::new(),
            predicate_data: Empty::new(),
            predicate_gas_used: Empty::new(),
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
            witness_index: Empty::new(),
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

    pub const fn asset_id<'a>(
        &'a self,
        base_asset_id: &'a AssetId,
    ) -> Option<&'a AssetId> {
        match self {
            Input::CoinSigned(CoinSigned { asset_id, .. })
            | Input::CoinPredicate(CoinPredicate { asset_id, .. }) => Some(asset_id),
            Input::MessageCoinSigned(_)
            | Input::MessageCoinPredicate(_)
            | Input::MessageDataSigned(_)
            | Input::MessageDataPredicate(_) => Some(base_asset_id),
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

    pub fn predicate_owner<P>(predicate: P) -> Address
    where
        P: AsRef<[u8]>,
    {
        use crate::Contract;

        let root = Contract::root_from_code(predicate);

        let mut hasher = Hasher::default();

        hasher.input(ContractId::SEED);
        hasher.input(root);

        (*hasher.digest()).into()
    }

    pub fn is_predicate_owner_valid<P>(owner: &Address, predicate: P) -> bool
    where
        P: AsRef<[u8]>,
    {
        owner == &Self::predicate_owner(predicate)
    }

    /// Prepare the output for script or predicate execution
    pub fn prepare_init_execute(&mut self) {
        self.prepare_sign(); // This currently does the same thing
    }
}

impl Serialize for Input {
    fn size_static(&self) -> usize {
        canonical::add_sizes(
            8, // Discriminant
            match self {
                Input::CoinSigned(coin) => coin.size_static(),
                Input::CoinPredicate(coin) => coin.size_static(),
                Input::Contract(contract) => contract.size_static(),
                Input::MessageCoinSigned(message) => message.size_static(),
                Input::MessageCoinPredicate(message) => message.size_static(),
                Input::MessageDataSigned(message) => message.size_static(),
                Input::MessageDataPredicate(message) => message.size_static(),
            },
        )
    }

    fn size_dynamic(&self) -> usize {
        match self {
            Input::CoinSigned(coin) => coin.size_dynamic(),
            Input::CoinPredicate(coin) => coin.size_dynamic(),
            Input::Contract(contract) => contract.size_dynamic(),
            Input::MessageCoinSigned(message) => message.size_dynamic(),
            Input::MessageCoinPredicate(message) => message.size_dynamic(),
            Input::MessageDataSigned(message) => message.size_dynamic(),
            Input::MessageDataPredicate(message) => message.size_dynamic(),
        }
    }

    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        let discr = InputRepr::from(self);
        discr.encode_static(buffer)?;
        match self {
            Input::CoinSigned(coin) => coin.encode_static(buffer),
            Input::CoinPredicate(coin) => coin.encode_static(buffer),
            Input::Contract(contract) => contract.encode_static(buffer),
            Input::MessageCoinSigned(message) => message.encode_static(buffer),
            Input::MessageCoinPredicate(message) => message.encode_static(buffer),
            Input::MessageDataSigned(message) => message.encode_static(buffer),
            Input::MessageDataPredicate(message) => message.encode_static(buffer),
        }
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        let discr = InputRepr::from(self);
        discr.encode_dynamic(buffer)?;
        match self {
            Input::CoinSigned(coin) => coin.encode_dynamic(buffer),
            Input::CoinPredicate(coin) => coin.encode_dynamic(buffer),
            Input::Contract(contract) => contract.encode_dynamic(buffer),
            Input::MessageCoinSigned(message) => message.encode_dynamic(buffer),
            Input::MessageCoinPredicate(message) => message.encode_dynamic(buffer),
            Input::MessageDataSigned(message) => message.encode_dynamic(buffer),
            Input::MessageDataPredicate(message) => message.encode_dynamic(buffer),
        }
    }
}

impl Deserialize for Input {
    fn decode_static<I: canonical::Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<Self, Error> {
        Ok(
            match <InputRepr as Deserialize>::decode(buffer)
                .map_err(|_| Error::UnknownDiscriminant)?
            {
                InputRepr::Coin => {
                    let coin = CoinFull::decode_static(buffer)?;
                    if coin.predicate.capacity() == 0 {
                        Input::CoinSigned(coin.into_signed())
                    } else {
                        Input::CoinPredicate(coin.into_predicate())
                    }
                }
                InputRepr::Contract => {
                    let contract = Contract::decode_static(buffer)?;
                    Input::Contract(contract)
                }
                InputRepr::Message => {
                    let message = FullMessage::decode_static(buffer)?;
                    match (
                        message.data.capacity() == 0,
                        message.predicate.capacity() == 0,
                    ) {
                        (true, true) => {
                            Input::MessageCoinSigned(message.into_coin_signed())
                        }
                        (true, false) => {
                            Input::MessageCoinPredicate(message.into_coin_predicate())
                        }
                        (false, true) => {
                            Input::MessageDataSigned(message.into_message_data_signed())
                        }
                        (false, false) => Input::MessageDataPredicate(
                            message.into_message_data_predicate(),
                        ),
                    }
                }
            },
        )
    }

    fn decode_dynamic<I: canonical::Input + ?Sized>(
        &mut self,
        buffer: &mut I,
    ) -> Result<(), Error> {
        match self {
            Input::CoinSigned(coin) => coin.decode_dynamic(buffer),
            Input::CoinPredicate(coin) => coin.decode_dynamic(buffer),
            Input::Contract(contract) => contract.decode_dynamic(buffer),
            Input::MessageCoinSigned(message) => message.decode_dynamic(buffer),
            Input::MessageCoinPredicate(message) => message.decode_dynamic(buffer),
            Input::MessageDataSigned(message) => message.decode_dynamic(buffer),
            Input::MessageDataPredicate(message) => message.decode_dynamic(buffer),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod snapshot_tests;

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use super::*;

    use crate::{
        TxPointer,
        UtxoId,
    };
    use fuel_types::{
        Address,
        AssetId,
        Bytes32,
        Word,
    };

    use alloc::{
        boxed::Box,
        format,
        string::String,
        vec::Vec,
    };

    #[derive(Clone, Eq, Hash, PartialEq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[wasm_bindgen]
    pub struct Input(#[wasm_bindgen(skip)] pub Box<crate::Input>);

    #[wasm_bindgen]
    impl Input {
        #[cfg(feature = "serde")]
        #[wasm_bindgen(js_name = toJSON)]
        pub fn to_json(&self) -> String {
            serde_json::to_string(&self.0).expect("unable to json format")
        }

        #[wasm_bindgen(js_name = toString)]
        pub fn typescript_to_string(&self) -> String {
            format!("{:?}", self.0)
        }

        #[wasm_bindgen(js_name = to_bytes)]
        pub fn typescript_to_bytes(&self) -> Vec<u8> {
            use fuel_types::canonical::Serialize;
            self.0.to_bytes()
        }

        #[wasm_bindgen(js_name = from_bytes)]
        pub fn typescript_from_bytes(value: &[u8]) -> Result<Input, js_sys::Error> {
            use fuel_types::canonical::Deserialize;
            crate::Input::from_bytes(value)
                .map(|v| Input(Box::new(v)))
                .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))
        }

        #[wasm_bindgen]
        pub fn coin_predicate(
            utxo_id: UtxoId,
            owner: Address,
            amount: Word,
            asset_id: AssetId,
            tx_pointer: TxPointer,
            predicate_gas_used: Word,
            predicate: Vec<u8>,
            predicate_data: Vec<u8>,
        ) -> Input {
            Input(Box::new(crate::Input::CoinPredicate(CoinPredicate {
                utxo_id,
                owner,
                amount,
                asset_id,
                tx_pointer,
                witness_index: Empty::new(),
                predicate_gas_used,
                predicate,
                predicate_data,
            })))
        }

        #[wasm_bindgen]
        pub fn coin_signed(
            utxo_id: UtxoId,
            owner: Address,
            amount: Word,
            asset_id: AssetId,
            tx_pointer: TxPointer,
            witness_index: u8,
        ) -> Input {
            Input(Box::new(crate::Input::CoinSigned(CoinSigned {
                utxo_id,
                owner,
                amount,
                asset_id,
                tx_pointer,
                witness_index,
                predicate_gas_used: Empty::new(),
                predicate: Empty::new(),
                predicate_data: Empty::new(),
            })))
        }

        #[wasm_bindgen]
        pub fn contract(
            utxo_id: UtxoId,
            balance_root: Bytes32,
            state_root: Bytes32,
            tx_pointer: TxPointer,
            contract_id: ContractId,
        ) -> Input {
            Input(Box::new(crate::Input::Contract(Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            })))
        }

        #[wasm_bindgen]
        pub fn message_coin_signed(
            sender: Address,
            recipient: Address,
            amount: Word,
            nonce: Nonce,
            witness_index: u8,
        ) -> Input {
            Input(Box::new(crate::Input::MessageCoinSigned(
                MessageCoinSigned {
                    sender,
                    recipient,
                    amount,
                    nonce,
                    witness_index,
                    predicate_gas_used: Empty::new(),
                    data: Empty::new(),
                    predicate: Empty::new(),
                    predicate_data: Empty::new(),
                },
            )))
        }

        #[wasm_bindgen]
        pub fn message_coin_predicate(
            sender: Address,
            recipient: Address,
            amount: Word,
            nonce: Nonce,
            predicate_gas_used: Word,
            predicate: Vec<u8>,
            predicate_data: Vec<u8>,
        ) -> Input {
            Input(Box::new(crate::Input::MessageCoinPredicate(
                MessageCoinPredicate {
                    sender,
                    recipient,
                    amount,
                    nonce,
                    witness_index: Empty::new(),
                    predicate_gas_used,
                    data: Empty::new(),
                    predicate,
                    predicate_data,
                },
            )))
        }

        #[wasm_bindgen]
        pub fn message_data_signed(
            sender: Address,
            recipient: Address,
            amount: Word,
            nonce: Nonce,
            witness_index: u8,
            data: Vec<u8>,
        ) -> Input {
            Input(Box::new(crate::Input::MessageDataSigned(
                MessageDataSigned {
                    sender,
                    recipient,
                    amount,
                    nonce,
                    witness_index,
                    data,
                    predicate: Empty::new(),
                    predicate_data: Empty::new(),
                    predicate_gas_used: Empty::new(),
                },
            )))
        }

        #[wasm_bindgen]
        pub fn message_data_predicate(
            sender: Address,
            recipient: Address,
            amount: Word,
            nonce: Nonce,
            predicate_gas_used: Word,
            data: Vec<u8>,
            predicate: Vec<u8>,
            predicate_data: Vec<u8>,
        ) -> Input {
            Input(Box::new(crate::Input::MessageDataPredicate(
                MessageDataPredicate {
                    sender,
                    recipient,
                    amount,
                    nonce,
                    witness_index: Empty::new(),
                    predicate_gas_used,
                    data,
                    predicate,
                    predicate_data,
                },
            )))
        }
    }
}
