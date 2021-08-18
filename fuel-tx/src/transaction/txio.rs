use super::{
    ContractId, Input, Output, Transaction, TransactionRepr, Witness,
    TRANSACTION_CREATE_FIXED_SIZE, TRANSACTION_SCRIPT_FIXED_SIZE,
};
use crate::bytes::{self, SizedBytes};

use fuel_asm::Word;

use std::convert::TryFrom;
use std::{io, mem};

const WORD_SIZE: usize = mem::size_of::<Word>();

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
                static_contracts, ..
            } => TRANSACTION_CREATE_FIXED_SIZE + static_contracts.len() * ContractId::size_of(),
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
                maturity,
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
                let buf = bytes::store_number_unchecked(buf, *maturity);
                let buf = bytes::store_number_unchecked(buf, script.len() as Word);
                let buf = bytes::store_number_unchecked(buf, script_data.len() as Word);
                let buf = bytes::store_number_unchecked(buf, inputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, outputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, witnesses.len() as Word);

                let (_, buf) = bytes::store_raw_bytes(buf, script.as_slice())?;
                let (_, buf) = bytes::store_raw_bytes(buf, script_data.as_slice())?;

                buf
            }

            Self::Create {
                gas_price,
                gas_limit,
                maturity,
                bytecode_witness_index,
                salt,
                static_contracts,
                inputs,
                outputs,
                witnesses,
                ..
            } => {
                let bytecode_length = witnesses
                    .get(*bytecode_witness_index as usize)
                    .map(|witness| witness.as_ref().len() as Word / 4)
                    .unwrap_or(0);

                let buf = bytes::store_number_unchecked(buf, TransactionRepr::Create as Word);
                let buf = bytes::store_number_unchecked(buf, *gas_price);
                let buf = bytes::store_number_unchecked(buf, *gas_limit);
                let buf = bytes::store_number_unchecked(buf, *maturity);
                let buf = bytes::store_number_unchecked(buf, bytecode_length);
                let buf = bytes::store_number_unchecked(buf, *bytecode_witness_index);
                let buf = bytes::store_number_unchecked(buf, static_contracts.len() as Word);
                let buf = bytes::store_number_unchecked(buf, inputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, outputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, witnesses.len() as Word);
                let mut buf = bytes::store_array_unchecked(buf, salt);

                for static_contract in static_contracts.iter() {
                    buf = bytes::store_array_unchecked(buf, static_contract);
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
                let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (script_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (script_data_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (inputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (outputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (witnesses_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };

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
                    maturity,
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
                let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };
                let (_bytecode_length, buf) = unsafe { bytes::restore_u16_unchecked(buf) };
                let (bytecode_witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
                let (static_contracts_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (inputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (outputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (witnesses_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
                let (salt, mut buf) = unsafe { bytes::restore_array_unchecked(buf) };

                let salt = salt.into();

                if buf.len() < static_contracts_len * ContractId::size_of() {
                    return Err(bytes::eof());
                }

                let mut static_contracts = vec![ContractId::default(); static_contracts_len];
                n += ContractId::size_of() * static_contracts_len;
                for static_contract in static_contracts.iter_mut() {
                    static_contract.copy_from_slice(&buf[..ContractId::size_of()]);
                    buf = &buf[ContractId::size_of()..];
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
                    maturity,
                    bytecode_witness_index,
                    salt,
                    static_contracts,
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

        Ok(())
    }
}
