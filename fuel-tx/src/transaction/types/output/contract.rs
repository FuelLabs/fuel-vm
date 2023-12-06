use fuel_types::Bytes32;

/// It is a full representation of the contract output from the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/output.md#outputcontract>.
///
/// The specification defines the layout of the [`Contract`] in the serialized form for
/// the `fuel-vm`.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen(js_name = OutputContract))]
pub struct Contract {
    /// Index of input contract.
    pub input_index: u8,
    /// Root of amount of coins owned by contract after transaction execution.
    pub balance_root: Bytes32,
    /// State root of contract after transaction execution.
    pub state_root: Bytes32,
}

impl Contract {
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/output.md#outputcontract>.
    pub fn prepare_sign(&mut self) {
        self.balance_root = Default::default();
        self.state_root = Default::default();
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
            input_index: rng.gen(),
            balance_root: rng.gen(),
            state_root: rng.gen(),
        }
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use super::*;

    use fuel_types::Bytes32;

    #[wasm_bindgen(js_class = OutputContract)]
    impl Contract {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new(
            input_index: u8,
            balance_root: Bytes32,
            state_root: Bytes32,
        ) -> Self {
            Self {
                input_index,
                balance_root,
                state_root,
            }
        }
    }
}
