use crate::bytes;

use fuel_asm::Word;
use itertools::Itertools;

use std::convert::TryFrom;
use std::io::Write;
use std::{io, mem};

mod types;
mod validation;

pub use types::{Address, Color, ContractAddress, Hash, Input, Output, Salt, Witness};
pub use validation::ValidationError;

const CONTRACT_ADDRESS_SIZE: usize = mem::size_of::<ContractAddress>();
const SALT_SIZE: usize = mem::size_of::<Salt>();
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
    + SALT_SIZE; // Salt

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
        Transaction::create(
            1,
            1000000,
            10,
            0,
            Salt::default(),
            vec![],
            vec![],
            vec![],
            vec![vec![0xffu8].into()],
        )
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

    pub const fn gas_limit(&self) -> Word {
        match self {
            Self::Script { gas_limit, .. } => *gas_limit,
            Self::Create { gas_limit, .. } => *gas_limit,
        }
    }

    pub const fn maturity(&self) -> Word {
        match self {
            Self::Script { maturity, .. } => *maturity,
            Self::Create { maturity, .. } => *maturity,
        }
    }

    pub const fn is_script(&self) -> bool {
        matches!(self, Self::Script { .. })
    }

    /// For a transaction of type `Create`, return the offset of the data
    /// relative to the serialized transaction for a given index of inputs,
    /// if this input is of type `Coin`.
    pub fn input_coin_data_offset(&self, index: usize) -> Option<usize> {
        match self {
            Transaction::Create {
                inputs,
                static_contracts,
                ..
            } => inputs.get(index).map(|input| match input {
                Input::Coin { predicate_data, .. } => Some(
                    TRANSACTION_CREATE_FIXED_SIZE
                        + CONTRACT_ADDRESS_SIZE * static_contracts.len()
                        + inputs.iter().take(index).map(|i| i.serialized_size()).sum::<usize>()
                        + input.serialized_size()
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
                Input::Coin { predicate_data, .. } => Some(
                    TRANSACTION_SCRIPT_FIXED_SIZE
                        + bytes::padded_len(script.as_slice())
                        + bytes::padded_len(script_data.as_slice())
                        + inputs.iter().take(index).map(|i| i.serialized_size()).sum::<usize>()
                        + input.serialized_size()
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
        match self {
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
                let mut n = TRANSACTION_SCRIPT_FIXED_SIZE;
                if buf.len() < n {
                    return Err(bytes::eof());
                }

                let buf = bytes::store_number_unchecked(buf, TransactionRepr::Script as Word);
                let buf = bytes::store_number_unchecked(buf, *gas_price);
                let buf = bytes::store_number_unchecked(buf, *gas_limit);
                let buf = bytes::store_number_unchecked(buf, *maturity);
                let buf = bytes::store_number_unchecked(buf, script.len() as Word);
                let buf = bytes::store_number_unchecked(buf, script_data.len() as Word);
                let buf = bytes::store_number_unchecked(buf, inputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, outputs.len() as Word);
                let buf = bytes::store_number_unchecked(buf, witnesses.len() as Word);

                let (size, buf) = bytes::store_raw_bytes(buf, script.as_slice())?;
                n += size;

                let (size, mut buf) = bytes::store_raw_bytes(buf, script_data.as_slice())?;
                n += size;

                for input in self.inputs_mut() {
                    let input_len = input.read(buf)?;
                    buf = &mut buf[input_len..];
                    n += input_len;
                }

                for output in self.outputs_mut() {
                    let output_len = output.read(buf)?;
                    buf = &mut buf[output_len..];
                    n += output_len;
                }

                for witness in self.witnesses_mut() {
                    let witness_len = witness.read(buf)?;
                    buf = &mut buf[witness_len..];
                    n += witness_len;
                }

                Ok(n)
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
                let mut n = TRANSACTION_CREATE_FIXED_SIZE + static_contracts.len() * CONTRACT_ADDRESS_SIZE;
                if buf.len() < n {
                    return Err(bytes::eof());
                }

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

                for input in self.inputs_mut() {
                    let input_len = input.read(buf)?;
                    buf = &mut buf[input_len..];
                    n += input_len;
                }

                for output in self.outputs_mut() {
                    let output_len = output.read(buf)?;
                    buf = &mut buf[output_len..];
                    n += output_len;
                }

                for witness in self.witnesses_mut() {
                    let witness_len = witness.read(buf)?;
                    buf = &mut buf[witness_len..];
                    n += witness_len;
                }

                Ok(n)
            }
        }
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

                let mut inputs = vec![
                    Input::contract(
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default()
                    );
                    inputs_len
                ];
                for input in inputs.iter_mut() {
                    let input_len = input.write(buf)?;
                    buf = &buf[input_len..];
                    n += input_len;
                }

                let mut outputs = vec![Output::contract_created(Default::default()); outputs_len];
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

                if buf.len() < static_contracts_len * CONTRACT_ADDRESS_SIZE {
                    return Err(bytes::eof());
                }

                let mut static_contracts = vec![ContractAddress::default(); static_contracts_len];
                n += CONTRACT_ADDRESS_SIZE * static_contracts_len;
                for static_contract in static_contracts.iter_mut() {
                    static_contract.copy_from_slice(&buf[..CONTRACT_ADDRESS_SIZE]);
                    buf = &buf[CONTRACT_ADDRESS_SIZE..];
                }

                let mut inputs = vec![
                    Input::contract(
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default()
                    );
                    inputs_len
                ];
                for input in inputs.iter_mut() {
                    let input_len = input.write(buf)?;
                    buf = &buf[input_len..];
                    n += input_len;
                }

                let mut outputs = vec![Output::contract_created(Default::default()); outputs_len];
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
        self.inputs_mut().iter_mut().try_for_each(|input| input.flush())?;
        self.outputs_mut().iter_mut().try_for_each(|output| output.flush())?;
        self.witnesses_mut()
            .iter_mut()
            .try_for_each(|witness| witness.flush())?;

        Ok(())
    }
}
