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
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct Contract {
    pub utxo_id: UtxoId,
    pub balance_root: Bytes32,
    pub state_root: Bytes32,
    pub tx_pointer: TxPointer,
    pub contract_id: ContractId,
}

impl Contract {
    /// The "Note" section from the specification:
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/input.md#inputcontract>.
    pub fn prepare_sign(&mut self) {
        core::mem::take(&mut self.utxo_id);
        core::mem::take(&mut self.balance_root);
        core::mem::take(&mut self.state_root);
        core::mem::take(&mut self.tx_pointer);
    }
}
