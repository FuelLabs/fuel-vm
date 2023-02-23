use crate::transaction::types::input::consts::INPUT_CONTRACT_SIZE;
use crate::{TxPointer, UtxoId};
use fuel_types::bytes::{Deserializable, SizedBytes, WORD_SIZE};
use fuel_types::{bytes, Bytes32, ContractId, Word};

/// It is a full representation of the contract input from the specification:
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputcontract.
///
/// The specification defines the layout of the [`Contract`] in the serialized form for
/// the `fuel-vm`.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Contract {
    pub utxo_id: UtxoId,
    pub balance_root: Bytes32,
    pub state_root: Bytes32,
    pub tx_pointer: TxPointer,
    pub contract_id: ContractId,
}

impl Contract {
    /// The "Note" section from the specification:
    /// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/input.md#inputcontract.
    pub fn prepare_sign(&mut self) {
        core::mem::take(&mut self.utxo_id);
        core::mem::take(&mut self.balance_root);
        core::mem::take(&mut self.state_root);
        core::mem::take(&mut self.tx_pointer);
    }
}

impl SizedBytes for Contract {
    #[inline(always)]
    fn serialized_size(&self) -> usize {
        INPUT_CONTRACT_SIZE - WORD_SIZE
    }
}

#[cfg(feature = "std")]
impl std::io::Read for Contract {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let serialized_size = self.serialized_size();
        if buf.len() < serialized_size {
            return Err(bytes::eof());
        }

        let Self {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        } = self;

        let buf = bytes::store_array_unchecked(buf, utxo_id.tx_id());
        let buf = bytes::store_number_unchecked(buf, utxo_id.output_index() as Word);
        let buf = bytes::store_array_unchecked(buf, balance_root);
        let buf = bytes::store_array_unchecked(buf, state_root);

        let n = tx_pointer.read(buf)?;
        let buf = &mut buf[n..];

        bytes::store_array_unchecked(buf, contract_id);
        Ok(serialized_size)
    }
}

#[cfg(feature = "std")]
impl std::io::Write for Contract {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = INPUT_CONTRACT_SIZE - WORD_SIZE;

        if buf.len() < n {
            return Err(bytes::eof());
        }

        let utxo_id = UtxoId::from_bytes(buf)?;
        let buf = &buf[utxo_id.serialized_size()..];
        self.utxo_id = utxo_id;

        // Safety: checked buffer len
        let (balance_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
        self.balance_root = balance_root.into();

        let (state_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };
        self.state_root = state_root.into();

        let tx_pointer = TxPointer::from_bytes(buf)?;
        let buf = &buf[tx_pointer.serialized_size()..];
        self.tx_pointer = tx_pointer;

        let (contract_id, _) = unsafe { bytes::restore_array_unchecked(buf) };
        self.contract_id = contract_id.into();

        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
