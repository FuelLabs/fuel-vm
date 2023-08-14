use crate::{
    input::sizes::ContractSizes,
    TxPointer,
    UtxoId,
};
use fuel_types::{
    bytes,
    bytes::SizedBytes,
    Bytes32,
    ContractId,
    MemLayout,
};

#[cfg(feature = "std")]
use fuel_types::{
    MemLocType,
    Word,
};

#[cfg(feature = "std")]
use fuel_types::bytes::Deserializable;

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

impl bytes::SizedBytes for Contract {
    #[inline(always)]
    fn serialized_size(&self) -> usize {
        ContractSizes::LEN
    }
}

#[cfg(feature = "std")]
impl std::io::Read for Contract {
    fn read(&mut self, full_buf: &mut [u8]) -> std::io::Result<usize> {
        let Self {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        } = self;

        type S = ContractSizes;
        const LEN: usize = ContractSizes::LEN;
        let buf: &mut [_; LEN] = full_buf
            .get_mut(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        bytes::store_at(buf, S::layout(S::LAYOUT.tx_id), utxo_id.tx_id());
        bytes::store_number_at(
            buf,
            S::layout(S::LAYOUT.output_index),
            utxo_id.output_index() as Word,
        );
        bytes::store_at(buf, S::layout(S::LAYOUT.balance_root), balance_root);
        bytes::store_at(buf, S::layout(S::LAYOUT.state_root), state_root);

        let n = tx_pointer.read(&mut buf[S::LAYOUT.tx_pointer.range()])?;
        if n != S::LAYOUT.tx_pointer.size() {
            return Err(bytes::eof())
        }

        bytes::store_at(buf, S::layout(S::LAYOUT.contract_id), contract_id);

        Ok(LEN)
    }
}

#[cfg(feature = "std")]
impl std::io::Write for Contract {
    fn write(&mut self, full_buf: &[u8]) -> std::io::Result<usize> {
        use fuel_types::bytes::Deserializable;
        type S = ContractSizes;
        const LEN: usize = ContractSizes::LEN;
        let buf: &[_; LEN] = full_buf
            .get(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let utxo_id = UtxoId::from_bytes(
            &buf[S::LAYOUT.tx_id.range().start..S::LAYOUT.output_index.range().end],
        )?;

        let balance_root = bytes::restore_at(buf, S::layout(S::LAYOUT.balance_root));
        let state_root = bytes::restore_at(buf, S::layout(S::LAYOUT.state_root));

        let tx_pointer = TxPointer::from_bytes(&buf[S::LAYOUT.tx_pointer.range()])?;

        let contract_id = bytes::restore_at(buf, S::layout(S::LAYOUT.contract_id));

        let balance_root = balance_root.into();
        let state_root = state_root.into();
        let contract_id = contract_id.into();

        *self = Self {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        };

        Ok(LEN)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
