use super::{TRANSACTION_CREATE_FIXED_SIZE, TRANSACTION_SCRIPT_FIXED_SIZE};
use crate::{Input, Metadata, Transaction};

use crate::transaction::types::SLOT_SIZE;
use fuel_types::bytes::{self, SizedBytes};
use fuel_types::ContractId;

impl Transaction {
    /// For a serialized transaction of type `Script`, return the bytes offset
    /// of the script
    pub const fn script_offset() -> usize {
        TRANSACTION_SCRIPT_FIXED_SIZE
    }

    /// For a serialized transaction of type `Script`, return the bytes offset
    /// of the script data
    pub fn script_data_offset(&self) -> Option<usize> {
        self.metadata()
            .map(Metadata::script_data_offset)
            .unwrap_or_else(|| self._script_data_offset())
    }

    pub(crate) fn _script_data_offset(&self) -> Option<usize> {
        match &self {
            Self::Script { script, .. } => {
                Some(TRANSACTION_SCRIPT_FIXED_SIZE + bytes::padded_len(script.as_slice()))
            }
            _ => None,
        }
    }

    /// For a transaction of type `Create`, return the offset of the data
    /// relative to the serialized transaction for a given index of inputs,
    /// if this input is of type `Coin`.
    pub fn input_coin_predicate_offset(&self, index: usize) -> Option<usize> {
        self.metadata()
            .map(|m| m.input_coin_predicate_offset(index))
            .unwrap_or_else(|| self._input_coin_predicate_offset(index))
    }

    pub(crate) fn _input_coin_predicate_offset(&self, index: usize) -> Option<usize> {
        self.input_offset(index)
            .map(|ofs| ofs + Input::coin_predicate_offset())
            .filter(|_| self.inputs()[index].is_coin_predicate())
    }

    /// Return the serialized bytes offset of the input with the provided index
    ///
    /// Return `None` if `index` is invalid
    pub fn input_offset(&self, index: usize) -> Option<usize> {
        self.metadata()
            .map(|m| m.inputs_offset(index))
            .unwrap_or_else(|| self._input_offset(index))
    }

    pub(crate) fn inputs_offset(&self) -> usize {
        match self {
            Transaction::Script {
                script,
                script_data,
                ..
            } => {
                TRANSACTION_SCRIPT_FIXED_SIZE
                    + bytes::padded_len(script.as_slice())
                    + bytes::padded_len(script_data.as_slice())
            }

            Transaction::Create {
                static_contracts,
                storage_slots,
                ..
            } => {
                TRANSACTION_CREATE_FIXED_SIZE
                    + ContractId::LEN * static_contracts.len()
                    + SLOT_SIZE * storage_slots.len()
            }
        }
    }

    pub(crate) fn _input_offset(&self, index: usize) -> Option<usize> {
        let offset = self.inputs_offset();
        self.inputs().iter().nth(index).map(|_| {
            self.inputs()
                .iter()
                .take(index)
                .map(|i| i.serialized_size())
                .sum::<usize>()
                + offset
        })
    }

    /// Return the serialized bytes offset of the output with the provided index
    ///
    /// Return `None` if `index` is invalid
    pub fn output_offset(&self, index: usize) -> Option<usize> {
        self.metadata()
            .map(|m| m.outputs_offset(index))
            .unwrap_or_else(|| self._output_offset(index))
    }

    pub(crate) fn outputs_offset(&self) -> usize {
        match self {
            Transaction::Script {
                script,
                script_data,
                inputs,
                ..
            } => {
                TRANSACTION_SCRIPT_FIXED_SIZE
                    + bytes::padded_len(script.as_slice())
                    + bytes::padded_len(script_data.as_slice())
                    + inputs.iter().map(|i| i.serialized_size()).sum::<usize>()
            }

            Transaction::Create {
                static_contracts,
                storage_slots,
                inputs,
                ..
            } => {
                TRANSACTION_CREATE_FIXED_SIZE
                    + ContractId::LEN * static_contracts.len()
                    + SLOT_SIZE * storage_slots.len()
                    + inputs.iter().map(|i| i.serialized_size()).sum::<usize>()
            }
        }
    }

    pub(crate) fn _output_offset(&self, index: usize) -> Option<usize> {
        let offset = self.outputs_offset();
        self.outputs().iter().nth(index).map(|_| {
            self.outputs()
                .iter()
                .take(index)
                .map(|i| i.serialized_size())
                .sum::<usize>()
                + offset
        })
    }

    /// Return the serialized bytes offset of the witness with the provided
    /// index
    ///
    /// Return `None` if `index` is invalid
    pub fn witness_offset(&self, index: usize) -> Option<usize> {
        self.metadata()
            .map(|m| m.witnesses_offset(index))
            .unwrap_or_else(|| self._witness_offset(index))
    }

    pub(crate) fn witnesses_offset(&self) -> usize {
        match self {
            Transaction::Script {
                script,
                script_data,
                inputs,
                outputs,
                ..
            } => {
                TRANSACTION_SCRIPT_FIXED_SIZE
                    + bytes::padded_len(script.as_slice())
                    + bytes::padded_len(script_data.as_slice())
                    + inputs.iter().map(|i| i.serialized_size()).sum::<usize>()
                    + outputs.iter().map(|o| o.serialized_size()).sum::<usize>()
            }

            Transaction::Create {
                static_contracts,
                storage_slots,
                inputs,
                outputs,
                ..
            } => {
                TRANSACTION_CREATE_FIXED_SIZE
                    + ContractId::LEN * static_contracts.len()
                    + SLOT_SIZE * storage_slots.len()
                    + inputs.iter().map(|i| i.serialized_size()).sum::<usize>()
                    + outputs.iter().map(|o| o.serialized_size()).sum::<usize>()
            }
        }
    }

    pub(crate) fn _witness_offset(&self, index: usize) -> Option<usize> {
        let offset = self.witnesses_offset();
        self.witnesses().iter().nth(index).map(|_| {
            self.witnesses()
                .iter()
                .take(index)
                .map(|i| i.serialized_size())
                .sum::<usize>()
                + offset
        })
    }
}
