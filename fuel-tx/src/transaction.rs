use crate::bytes::{self, SerializableVec, SizedBytes};
use crate::crypto;

use fuel_asm::{Opcode, Word};
use itertools::Itertools;

use std::convert::TryFrom;
use std::io::Write;
use std::{io, mem};

mod types;
mod validation;

pub use types::{Address, Color, ContractAddress, Hash, Input, Output, Salt, Witness};
pub use validation::ValidationError;

const WORD_SIZE: usize = mem::size_of::<Word>();

const TRANSACTION_SCRIPT_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Maturity
    + WORD_SIZE // Script size
    + WORD_SIZE // Script data size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE; // Witnesses size

const TRANSACTION_CREATE_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Maturity
    + WORD_SIZE // Bytecode size
    + WORD_SIZE // Bytecode witness index
    + WORD_SIZE // Static contracts size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE // Witnesses size
    + Salt::size_of(); // Salt

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
}

impl TryFrom<Word> for TransactionRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Script),
            0x01 => Ok(Self::Create),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Transaction {
    Script {
        gas_price: Word,
        gas_limit: Word,
        maturity: Word,
        script: Vec<u8>,
        script_data: Vec<u8>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    },

    Create {
        gas_price: Word,
        gas_limit: Word,
        maturity: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        static_contracts: Vec<ContractAddress>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    },
}

impl Default for Transaction {
    fn default() -> Self {
        // Create a valid transaction with a single return instruction
        //
        // The Return op is mandatory for the execution of any context
        let script = Opcode::RET(0x10).to_bytes().to_vec();

        Transaction::script(0, 1000000, 0, script, vec![], vec![], vec![], vec![])
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
                static_contracts, ..
            } => {
                TRANSACTION_CREATE_FIXED_SIZE + static_contracts.len() * ContractAddress::size_of()
            }
        };

        n + inputs + outputs + witnesses
    }
}

impl Transaction {
    pub const fn script(
        gas_price: Word,
        gas_limit: Word,
        maturity: Word,
        script: Vec<u8>,
        script_data: Vec<u8>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Self {
        Self::Script {
            gas_price,
            gas_limit,
            maturity,
            script,
            script_data,
            inputs,
            outputs,
            witnesses,
        }
    }

    pub const fn create(
        gas_price: Word,
        gas_limit: Word,
        maturity: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        static_contracts: Vec<ContractAddress>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Self {
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
        }
    }

    pub fn input_colors(&self) -> impl Iterator<Item = &Color> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::Coin { color, .. } => Some(color),
                _ => None,
            })
            .unique()
    }

    pub fn id(&self) -> Hash {
        let mut tx = self.clone();
        tx.prepare_sign();

        crypto::hash(tx.to_bytes().as_slice())
    }

    pub fn prepare_sign(&mut self) {
        self.inputs_mut().iter_mut().for_each(|input| {
            if let Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                ..
            } = input
            {
                utxo_id.iter_mut().for_each(|b| *b = 0);
                balance_root.iter_mut().for_each(|b| *b = 0);
                state_root.iter_mut().for_each(|b| *b = 0);
            }
        });

        self.outputs_mut()
            .iter_mut()
            .for_each(|output| match output {
                Output::Contract {
                    balance_root,
                    state_root,
                    ..
                } => {
                    balance_root.iter_mut().for_each(|b| *b = 0);
                    state_root.iter_mut().for_each(|b| *b = 0);
                }

                Output::Change { amount, .. } => *amount = 0,

                Output::Variable {
                    to, amount, color, ..
                } => {
                    to.iter_mut().for_each(|b| *b = 0);
                    *amount = 0;
                    color.iter_mut().for_each(|b| *b = 0);
                }

                _ => (),
            });
    }

    pub fn input_contracts(&self) -> impl Iterator<Item = &ContractAddress> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::Contract { contract_id, .. } => Some(contract_id),
                _ => None,
            })
            .unique()
    }

    pub const fn gas_price(&self) -> Word {
        match self {
            Self::Script { gas_price, .. } => *gas_price,
            Self::Create { gas_price, .. } => *gas_price,
        }
    }

    pub fn set_gas_price(&mut self, price: Word) {
        match self {
            Self::Script { gas_price, .. } => *gas_price = price,
            Self::Create { gas_price, .. } => *gas_price = price,
        }
    }

    pub const fn gas_limit(&self) -> Word {
        match self {
            Self::Script { gas_limit, .. } => *gas_limit,
            Self::Create { gas_limit, .. } => *gas_limit,
        }
    }

    pub fn set_gas_limit(&mut self, limit: Word) {
        match self {
            Self::Script { gas_limit, .. } => *gas_limit = limit,
            Self::Create { gas_limit, .. } => *gas_limit = limit,
        }
    }

    pub const fn maturity(&self) -> Word {
        match self {
            Self::Script { maturity, .. } => *maturity,
            Self::Create { maturity, .. } => *maturity,
        }
    }

    pub fn set_maturity(&mut self, mat: Word) {
        match self {
            Self::Script { maturity, .. } => *maturity = mat,
            Self::Create { maturity, .. } => *maturity = mat,
        }
    }

    pub const fn is_script(&self) -> bool {
        matches!(self, Self::Script { .. })
    }

    /// For a serialized transaction of type `Script`, return the bytes offset
    /// of the script
    pub const fn script_offset() -> usize {
        TRANSACTION_SCRIPT_FIXED_SIZE
    }

    /// For a serialized transaction of type `Script`, return the bytes offset
    /// of the script data
    pub fn script_data_offset(&self) -> Option<usize> {
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
        match self {
            Transaction::Create {
                inputs,
                static_contracts,
                ..
            } => inputs.get(index).map(|input| match input {
                Input::Coin {
                    predicate,
                    predicate_data,
                    ..
                } => Some(
                    TRANSACTION_CREATE_FIXED_SIZE
                        + ContractAddress::size_of() * static_contracts.len()
                        + inputs
                            .iter()
                            .take(index)
                            .map(|i| i.serialized_size())
                            .sum::<usize>()
                        + input.serialized_size()
                        - bytes::padded_len(predicate.as_slice())
                        - bytes::padded_len(predicate_data.as_slice()),
                ),

                _ => None,
            }),

            Transaction::Script {
                inputs,
                script,
                script_data,
                ..
            } => inputs.get(index).map(|input| match input {
                Input::Coin {
                    predicate,
                    predicate_data,
                    ..
                } => Some(
                    TRANSACTION_SCRIPT_FIXED_SIZE
                        + bytes::padded_len(script.as_slice())
                        + bytes::padded_len(script_data.as_slice())
                        + inputs
                            .iter()
                            .take(index)
                            .map(|i| i.serialized_size())
                            .sum::<usize>()
                        + input.serialized_size()
                        - bytes::padded_len(predicate.as_slice())
                        - bytes::padded_len(predicate_data.as_slice()),
                ),

                _ => None,
            }),
        }
        .flatten()
    }

    pub fn inputs(&self) -> &[Input] {
        match self {
            Self::Script { inputs, .. } => inputs.as_slice(),
            Self::Create { inputs, .. } => inputs.as_slice(),
        }
    }

    pub fn inputs_mut(&mut self) -> &mut [Input] {
        match self {
            Self::Script { inputs, .. } => inputs.as_mut_slice(),
            Self::Create { inputs, .. } => inputs.as_mut_slice(),
        }
    }

    pub fn outputs(&self) -> &[Output] {
        match self {
            Self::Script { outputs, .. } => outputs.as_slice(),
            Self::Create { outputs, .. } => outputs.as_slice(),
        }
    }

    pub fn outputs_mut(&mut self) -> &mut [Output] {
        match self {
            Self::Script { outputs, .. } => outputs.as_mut_slice(),
            Self::Create { outputs, .. } => outputs.as_mut_slice(),
        }
    }

    pub fn witnesses(&self) -> &[Witness] {
        match self {
            Self::Script { witnesses, .. } => witnesses.as_slice(),
            Self::Create { witnesses, .. } => witnesses.as_slice(),
        }
    }

    pub fn witnesses_mut(&mut self) -> &mut [Witness] {
        match self {
            Self::Script { witnesses, .. } => witnesses.as_mut_slice(),
            Self::Create { witnesses, .. } => witnesses.as_mut_slice(),
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> io::Result<(usize, Self)> {
        let mut tx = Self::script(0, 0, 0, vec![], vec![], vec![], vec![], vec![]);

        let n = tx.write(bytes)?;

        Ok((n, tx))
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

        let (identifier, buf): (Word, _) = bytes::restore_number_unchecked(buf);
        let identifier = TransactionRepr::try_from(identifier)?;

        match identifier {
            TransactionRepr::Script => {
                let mut n = TRANSACTION_SCRIPT_FIXED_SIZE;
                if buf.len() < n - WORD_SIZE {
                    return Err(bytes::eof());
                }

                let (gas_price, buf) = bytes::restore_number_unchecked(buf);
                let (gas_limit, buf) = bytes::restore_number_unchecked(buf);
                let (maturity, buf) = bytes::restore_number_unchecked(buf);
                let (script_len, buf) = bytes::restore_usize_unchecked(buf);
                let (script_data_len, buf) = bytes::restore_usize_unchecked(buf);
                let (inputs_len, buf) = bytes::restore_usize_unchecked(buf);
                let (outputs_len, buf) = bytes::restore_usize_unchecked(buf);
                let (witnesses_len, buf) = bytes::restore_usize_unchecked(buf);

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
                };

                Ok(n)
            }

            TransactionRepr::Create => {
                let mut n = TRANSACTION_CREATE_FIXED_SIZE;
                if buf.len() < n - WORD_SIZE {
                    return Err(bytes::eof());
                }

                let (gas_price, buf) = bytes::restore_number_unchecked(buf);
                let (gas_limit, buf) = bytes::restore_number_unchecked(buf);
                let (maturity, buf) = bytes::restore_number_unchecked(buf);
                let (_bytecode_length, buf) = bytes::restore_u16_unchecked(buf);
                let (bytecode_witness_index, buf) = bytes::restore_u8_unchecked(buf);
                let (static_contracts_len, buf) = bytes::restore_usize_unchecked(buf);
                let (inputs_len, buf) = bytes::restore_usize_unchecked(buf);
                let (outputs_len, buf) = bytes::restore_usize_unchecked(buf);
                let (witnesses_len, buf) = bytes::restore_usize_unchecked(buf);
                let (salt, mut buf) = bytes::restore_array_unchecked(buf);

                let salt = salt.into();

                if buf.len() < static_contracts_len * ContractAddress::size_of() {
                    return Err(bytes::eof());
                }

                let mut static_contracts = vec![ContractAddress::default(); static_contracts_len];
                n += ContractAddress::size_of() * static_contracts_len;
                for static_contract in static_contracts.iter_mut() {
                    static_contract.copy_from_slice(&buf[..ContractAddress::size_of()]);
                    buf = &buf[ContractAddress::size_of()..];
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
