use super::{TransactionRepr, TRANSACTION_CREATE_FIXED_SIZE, TRANSACTION_SCRIPT_FIXED_SIZE};
use crate::transaction::types::StorageSlot;
use crate::{Input, Output, Transaction, Witness};

use fuel_types::bytes::{self, SizedBytes, WORD_SIZE};
use fuel_types::{ContractId, Word};

use std::io::{self, Write};

impl Transaction {
    pub fn try_from_bytes(bytes: &[u8]) -> io::Result<(usize, Self)> {
        let mut tx = Self::default();

        let n = tx.write(bytes)?;

        Ok((n, tx))
    }
}

impl bytes::SizedBytes for Transaction {
    fn serialized_size(&self) -> usize {
        let inputs = self
            .inputs()
            .iter()
            .map(|i| i.serialized_size())
            .sum::<usize>();
        let outputs = self
            .outputs()
            .iter()
            .map(|o| o.serialized_size())
            .sum::<usize>();
        let witnesses = self
            .witnesses()
            .iter()
            .map(|w| w.serialized_size())
            .sum::<usize>();

        let n = match self {
            Self::Script {
                script,
                script_data,
                ..
            } => {
                TRANSACTION_SCRIPT_FIXED_SIZE
                    + bytes::padded_len(script.as_slice())
                    + bytes::padded_len(script_data.as_slice())
            }

            Self::Create {
                static_contracts,
                storage_slots,
                ..
            } => {
                TRANSACTION_CREATE_FIXED_SIZE
                    + static_contracts.len() * ContractId::LEN
                    + storage_slots.len() * StorageSlot::SLOT_SIZE
            }
        };

        n + inputs + outputs + witnesses
    }
}

impl io::Read for Transaction {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let mut buf = match self {
            Self::Script {
                gas_price,
                gas_limit,
                byte_price,
                maturity,
                receipts_root,
                script,
                script_data,
                inputs,
                outputs,
                witnesses,
                ..
            } => {
                let buf = bytes::store_number_unchecked(buf, TransactionRepr::Script as Word);
                let buf = bytes::store_number_unchecked(buf, *gas_price);
                let buf = bytes::store_number_unchecked(buf, *gas_limit);
                let buf = bytes::store_number_unchecked(buf, *byte_price);
                let buf = bytes::store_number_unchecked(buf, *maturity);
                let buf = bytes::store_number_unchecked(buf, script.len() as Word);
                let buf = bytes::store_number_unchecked(buf, script_data.len() as Word);
                let buf = bytes::store_number_unchecked(buf, inputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, outputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, witnesses.len() as Word);
                let buf = bytes::store_array_unchecked(buf, receipts_root);

                let (_, buf) = bytes::store_raw_bytes(buf, script.as_slice())?;
                let (_, buf) = bytes::store_raw_bytes(buf, script_data.as_slice())?;

                buf
            }

            Self::Create {
                gas_price,
                gas_limit,
                byte_price,
                maturity,
                bytecode_length,
                bytecode_witness_index,
                salt,
                static_contracts,
                storage_slots,
                inputs,
                outputs,
                witnesses,
                ..
            } => {
                let buf = bytes::store_number_unchecked(buf, TransactionRepr::Create as Word);
                let buf = bytes::store_number_unchecked(buf, *gas_price);
                let buf = bytes::store_number_unchecked(buf, *gas_limit);
                let buf = bytes::store_number_unchecked(buf, *byte_price);
                let buf = bytes::store_number_unchecked(buf, *maturity);
                let buf = bytes::store_number_unchecked(buf, *bytecode_length);
                let buf = bytes::store_number_unchecked(buf, *bytecode_witness_index);
                let buf = bytes::store_number_unchecked(buf, static_contracts.len() as Word);
                let buf = bytes::store_number_unchecked(buf, storage_slots.len() as Word);
                let buf = bytes::store_number_unchecked(buf, inputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, outputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, witnesses.len() as Word);
                let mut buf = bytes::store_array_unchecked(buf, salt);

                for static_contract in static_contracts.iter() {
                    buf = bytes::store_array_unchecked(buf, static_contract);
                }

                for storage_slot in storage_slots.iter_mut() {
                    let storage_len = storage_slot.read(buf)?;
                    buf = &mut buf[storage_len..];
                }

                buf
            }
        };

        for input in self.inputs_mut() {
            let input_len = input.read(buf)?;
            buf = &mut buf[input_len..];
        }

        for output in self.outputs_mut() {
            let output_len = output.read(buf)?;
            buf = &mut buf[output_len..];
        }

        for witness in self.witnesses_mut() {
            let witness_len = witness.read(buf)?;
            buf = &mut buf[witness_len..];
        }

        Ok(n)
    }
}

impl io::Write for Transaction {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        // Safety: buffer size is checked
        let (identifier, buf): (Word, _) = unsafe { bytes::restore_number_unchecked(buf) };
        let identifier = TransactionRepr::try_from(identifier)?;

        match identifier {
            TransactionRepr::Script => {
                let mut n = TRANSACTION_SCRIPT_FIXED_SIZE;
                if buf.len() < n - WORD_SIZE {
                    return Err(bytes::eof());
                }

                // Safety: buffer size is checked
                let (gas_price, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (gas_limit, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (byte_price, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (script_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (script_data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (inputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (outputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (witnesses_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (receipts_root, buf) = unsafe { bytes::restore_array_unchecked(buf) };

                let receipts_root = receipts_root.into();

                let (size, script, buf) = bytes::restore_raw_bytes(buf, script_len)?;
                n += size;

                let (size, script_data, mut buf) = bytes::restore_raw_bytes(buf, script_data_len)?;
                n += size;

                let mut inputs = vec![Input::default(); inputs_len];
                for input in inputs.iter_mut() {
                    let input_len = input.write(buf)?;
                    buf = &buf[input_len..];
                    n += input_len;
                }

                let mut outputs = vec![Output::default(); outputs_len];
                for output in outputs.iter_mut() {
                    let output_len = output.write(buf)?;
                    buf = &buf[output_len..];
                    n += output_len;
                }

                let mut witnesses = vec![Witness::default(); witnesses_len];
                for witness in witnesses.iter_mut() {
                    let witness_len = witness.write(buf)?;
                    buf = &buf[witness_len..];
                    n += witness_len;
                }

                *self = Transaction::Script {
                    gas_price,
                    gas_limit,
                    byte_price,
                    maturity,
                    receipts_root,
                    script,
                    script_data,
                    inputs,
                    outputs,
                    witnesses,
                    metadata: None,
                };

                Ok(n)
            }

            TransactionRepr::Create => {
                let mut n = TRANSACTION_CREATE_FIXED_SIZE;
                if buf.len() < n - WORD_SIZE {
                    return Err(bytes::eof());
                }

                // Safety: buffer size is checked
                let (gas_price, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (gas_limit, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (byte_price, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (bytecode_length, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (bytecode_witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
                let (static_contracts_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (storage_slots_len, buf) = unsafe { bytes::restore_u16_unchecked(buf) };
                let (inputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (outputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (witnesses_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (salt, mut buf) = unsafe { bytes::restore_array_unchecked(buf) };

                let salt = salt.into();

                if buf.len() < static_contracts_len * ContractId::LEN {
                    return Err(bytes::eof());
                }

                let mut static_contracts = vec![ContractId::default(); static_contracts_len];
                n += ContractId::LEN * static_contracts_len;
                for static_contract in static_contracts.iter_mut() {
                    static_contract.copy_from_slice(&buf[..ContractId::LEN]);
                    buf = &buf[ContractId::LEN..];
                }

                let mut storage_slots = vec![StorageSlot::default(); storage_slots_len as usize];
                n += StorageSlot::SLOT_SIZE * storage_slots_len as usize;
                for storage_slot in storage_slots.iter_mut() {
                    let _ = storage_slot.write(buf)?;
                    buf = &buf[StorageSlot::SLOT_SIZE..];
                }

                let mut inputs = vec![Input::default(); inputs_len];
                for input in inputs.iter_mut() {
                    let input_len = input.write(buf)?;
                    buf = &buf[input_len..];
                    n += input_len;
                }

                let mut outputs = vec![Output::default(); outputs_len];
                for output in outputs.iter_mut() {
                    let output_len = output.write(buf)?;
                    buf = &buf[output_len..];
                    n += output_len;
                }

                let mut witnesses = vec![Witness::default(); witnesses_len];
                for witness in witnesses.iter_mut() {
                    let witness_len = witness.write(buf)?;
                    buf = &buf[witness_len..];
                    n += witness_len;
                }

                *self = Self::Create {
                    gas_price,
                    gas_limit,
                    byte_price,
                    maturity,
                    bytecode_length,
                    bytecode_witness_index,
                    salt,
                    static_contracts,
                    storage_slots,
                    inputs,
                    outputs,
                    witnesses,
                    metadata: None,
                };

                Ok(n)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inputs_mut()
            .iter_mut()
            .try_for_each(|input| input.flush())?;
        self.outputs_mut()
            .iter_mut()
            .try_for_each(|output| output.flush())?;
        self.witnesses_mut()
            .iter_mut()
            .try_for_each(|witness| witness.flush())?;

        if let Transaction::Create { storage_slots, .. } = self {
            storage_slots.iter_mut().try_for_each(|slot| slot.flush())?;
        }

        Ok(())
    }
}
