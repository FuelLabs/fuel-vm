use crate::{
    TxPointer,
    UtxoId,
};
use fuel_types::{
    Bytes32,
    ContractId,
};

/// It is a full representation of the contract input from the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcontract>.
///
/// The specification defines the layout of the [`Contract`] in the serialized form for
/// the `fuel-vm`.
#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen(js_name = InputContract))]
pub struct Contract {
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub utxo_id: UtxoId,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub balance_root: Bytes32,
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub state_root: Bytes32,
    /// Pointer to transction that last modified the contract state.
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub tx_pointer: TxPointer,
    pub contract_id: ContractId,
}

impl Contract {
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcontract>.
    pub fn prepare_sign(&mut self) {
        self.utxo_id = Default::default();
        self.balance_root = Default::default();
        self.state_root = Default::default();
        self.tx_pointer = Default::default();
    }
}

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

#[cfg(feature = "random")]
impl Distribution<Contract> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Contract {
        Contract {
            utxo_id: rng.gen(),
            balance_root: rng.gen(),
            state_root: rng.gen(),
            tx_pointer: rng.gen(),
            contract_id: rng.gen(),
        }
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use super::*;

    use crate::{
        TxPointer,
        UtxoId,
    };
    use fuel_types::{
        Bytes32,
        ContractId,
    };

    #[wasm_bindgen(js_class = InputContract)]
    impl Contract {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new(
            utxo_id: UtxoId,
            balance_root: Bytes32,
            state_root: Bytes32,
            tx_pointer: TxPointer,
            contract_id: ContractId,
        ) -> Self {
            Self {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            }
        }
    }
}
