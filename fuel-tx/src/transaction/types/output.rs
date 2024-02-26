use fuel_crypto::Hasher;
use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    Address,
    AssetId,
    Bytes32,
    ContractId,
    Nonce,
    Word,
};

use core::mem;

mod consts;
pub mod contract;
mod repr;

use contract::Contract;
pub use repr::OutputRepr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum_macros::EnumCount)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Deserialize, Serialize)]
pub enum Output {
    Coin {
        to: Address,
        amount: Word,
        asset_id: AssetId,
    },

    Contract(Contract),

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

impl Output {
    pub const fn repr(&self) -> OutputRepr {
        OutputRepr::from_output(self)
    }

    pub const fn coin(to: Address, amount: Word, asset_id: AssetId) -> Self {
        Self::Coin {
            to,
            amount,
            asset_id,
        }
    }

    pub const fn contract(
        input_index: u8,
        balance_root: Bytes32,
        state_root: Bytes32,
    ) -> Self {
        Self::Contract(Contract {
            input_index,
            balance_root,
            state_root,
        })
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
            Output::Coin { asset_id, .. }
            | Output::Change { asset_id, .. }
            | Output::Variable { asset_id, .. } => Some(asset_id),
            _ => None,
        }
    }

    pub const fn to(&self) -> Option<&Address> {
        match self {
            Output::Coin { to, .. }
            | Output::Change { to, .. }
            | Output::Variable { to, .. } => Some(to),
            _ => None,
        }
    }

    pub const fn amount(&self) -> Option<Word> {
        match self {
            Output::Coin { amount, .. }
            | Output::Change { amount, .. }
            | Output::Variable { amount, .. } => Some(*amount),
            _ => None,
        }
    }

    pub const fn input_index(&self) -> Option<u8> {
        match self {
            Output::Contract(Contract { input_index, .. }) => Some(*input_index),
            _ => None,
        }
    }

    pub const fn balance_root(&self) -> Option<&Bytes32> {
        match self {
            Output::Contract(Contract { balance_root, .. }) => Some(balance_root),
            _ => None,
        }
    }

    pub const fn state_root(&self) -> Option<&Bytes32> {
        match self {
            Output::Contract(Contract { state_root, .. })
            | Output::ContractCreated { state_root, .. } => Some(state_root),
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
        matches!(self, Self::Contract(_))
    }

    pub const fn is_contract_created(&self) -> bool {
        matches!(self, Self::ContractCreated { .. })
    }

    pub fn message_nonce(txid: &Bytes32, idx: Word) -> Nonce {
        (*Hasher::default()
            .chain(txid)
            .chain(idx.to_bytes())
            .finalize())
        .into()
    }

    pub fn message_digest(data: &[u8]) -> Bytes32 {
        Hasher::hash(data)
    }

    /// Empties fields that should be zero during the signing.
    pub(crate) fn prepare_sign(&mut self) {
        match self {
            Output::Contract(contract) => contract.prepare_sign(),

            Output::Change { amount, .. } => {
                mem::take(amount);
            }

            Output::Variable {
                to,
                amount,
                asset_id,
                ..
            } => {
                mem::take(to);
                mem::take(amount);
                mem::take(asset_id);
            }

            _ => (),
        }
    }

    /// Prepare the output for VM initialization for script execution
    /// or predicate verification
    pub fn prepare_init_execute(&mut self) {
        self.prepare_sign() // Currently does the same thing
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use super::*;

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
    pub struct Output(#[wasm_bindgen(skip)] pub Box<crate::Output>);

    #[wasm_bindgen]
    impl Output {
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
        pub fn typescript_from_bytes(value: &[u8]) -> Result<Output, js_sys::Error> {
            use fuel_types::canonical::Deserialize;
            crate::Output::from_bytes(value)
                .map(|v| Output(Box::new(v)))
                .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))
        }

        #[wasm_bindgen]
        pub fn coin(to: Address, amount: Word, asset_id: AssetId) -> Output {
            Output(Box::new(crate::Output::coin(to, amount, asset_id)))
        }

        #[wasm_bindgen]
        pub fn contract(
            input_index: u8,
            balance_root: Bytes32,
            state_root: Bytes32,
        ) -> Output {
            Output(Box::new(crate::Output::contract(
                input_index,
                balance_root,
                state_root,
            )))
        }

        #[wasm_bindgen]
        pub fn change(to: Address, amount: Word, asset_id: AssetId) -> Output {
            Output(Box::new(crate::Output::change(to, amount, asset_id)))
        }

        #[wasm_bindgen]
        pub fn variable(to: Address, amount: Word, asset_id: AssetId) -> Output {
            Output(Box::new(crate::Output::variable(to, amount, asset_id)))
        }

        #[wasm_bindgen]
        pub fn contract_created(contract_id: ContractId, state_root: Bytes32) -> Output {
            Output(Box::new(crate::Output::contract_created(
                contract_id,
                state_root,
            )))
        }
    }
}
