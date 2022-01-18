use crate::consts::*;
use crate::{Input, Output, Transaction, Witness};

use fuel_types::{Color, Word};

mod error;

pub use error::ValidationError;

impl Input {
    pub fn validate(
        &self,
        index: usize,
        outputs: &[Output],
        witnesses: &[Witness],
    ) -> Result<(), ValidationError> {
        match self {
            Self::Coin { predicate, .. } if predicate.len() > MAX_PREDICATE_LENGTH as usize => {
                Err(ValidationError::InputCoinPredicateLength { index })
            }

            Self::Coin { predicate_data, .. }
                if predicate_data.len() > MAX_PREDICATE_DATA_LENGTH as usize =>
            {
                Err(ValidationError::InputCoinPredicateDataLength { index })
            }

            Self::Coin { witness_index, .. } if *witness_index as usize >= witnesses.len() => {
                Err(ValidationError::InputCoinWitnessIndexBounds { index })
            }

            // ∀ inputContract ∃! outputContract : outputContract.inputIndex = inputContract.index
            Self::Contract { .. }
                if 1 != outputs
                    .iter()
                    .filter_map(|output| match output {
                        Output::Contract { input_index, .. } if *input_index as usize == index => {
                            Some(())
                        }
                        _ => None,
                    })
                    .count() =>
            {
                Err(ValidationError::InputContractAssociatedOutputContract { index })
            }

            // TODO If h is the block height the UTXO being spent was created, transaction is
            // invalid if `blockheight() < h + maturity`.
            _ => Ok(()),
        }
    }
}

impl Output {
    pub fn validate(&self, index: usize, inputs: &[Input]) -> Result<(), ValidationError> {
        match self {
            Self::Contract { input_index, .. } => match inputs.get(*input_index as usize) {
                Some(Input::Contract { .. }) => Ok(()),
                _ => Err(ValidationError::OutputContractInputIndex { index }),
            },

            _ => Ok(()),
        }
    }
}

impl Transaction {
    pub fn validate(&self, block_height: Word) -> Result<(), ValidationError> {
        if self.gas_price() > MAX_GAS_PER_TX {
            Err(ValidationError::TransactionGasLimit)?
        }

        if block_height < self.maturity() as Word {
            Err(ValidationError::TransactionMaturity)?;
        }

        if self.inputs().len() > MAX_INPUTS as usize {
            Err(ValidationError::TransactionInputsMax)?
        }

        if self.outputs().len() > MAX_OUTPUTS as usize {
            Err(ValidationError::TransactionOutputsMax)?
        }

        if self.witnesses().len() > MAX_WITNESSES as usize {
            Err(ValidationError::TransactionWitnessesMax)?
        }

        let input_colors: Vec<&Color> = self.input_colors().collect();
        for input_color in input_colors.as_slice() {
            if self
                .outputs()
                .iter()
                .filter_map(|output| match output {
                    Output::Change { color, .. }
                        if color != &Color::default() && input_color == &color =>
                    {
                        Some(())
                    }
                    _ => None,
                })
                .count()
                > 1
            {
                Err(ValidationError::TransactionOutputChangeColorDuplicated)?
            }
        }

        for (index, input) in self.inputs().iter().enumerate() {
            input.validate(index, self.outputs(), self.witnesses())?;
        }

        for (index, output) in self.outputs().iter().enumerate() {
            output.validate(index, self.inputs())?;
            if let Output::Change { color, .. } = output {
                if !input_colors.iter().any(|input_color| input_color == &color) {
                    Err(ValidationError::TransactionOutputChangeColorNotFound)?
                }
            }
        }

        match self {
            Self::Script {
                outputs,
                script,
                script_data,
                ..
            } => {
                if script.len() > MAX_SCRIPT_LENGTH as usize {
                    Err(ValidationError::TransactionScriptLength)?;
                }

                if script_data.len() > MAX_SCRIPT_DATA_LENGTH as usize {
                    Err(ValidationError::TransactionScriptDataLength)?;
                }

                outputs
                    .iter()
                    .enumerate()
                    .try_for_each(|(index, output)| match output {
                        Output::ContractCreated { .. } => {
                            Err(ValidationError::TransactionScriptOutputContractCreated { index })
                        }
                        _ => Ok(()),
                    })?;

                Ok(())
            }

            Self::Create {
                inputs,
                outputs,
                witnesses,
                bytecode_witness_index,
                static_contracts,
                ..
            } => {
                match witnesses.get(*bytecode_witness_index as usize) {
                    Some(witness) if witness.as_ref().len() as u64 > CONTRACT_MAX_SIZE => {
                        Err(ValidationError::TransactionCreateBytecodeLen)?
                    }
                    None => Err(ValidationError::TransactionCreateBytecodeWitnessIndex)?,
                    _ => (),
                }

                if static_contracts.len() > MAX_STATIC_CONTRACTS as usize {
                    Err(ValidationError::TransactionCreateStaticContractsMax)?;
                }

                if !static_contracts.as_slice().windows(2).all(|w| w[0] <= w[1]) {
                    Err(ValidationError::TransactionCreateStaticContractsOrder)?;
                }

                // TODO Any contract with ADDRESS in staticContracts is not in the state
                // TODO The computed contract ADDRESS (see below) is not equal to the
                // contractADDRESS of the one OutputType.ContractCreated output

                for (index, input) in inputs.iter().enumerate() {
                    if let Input::Contract { .. } = input {
                        Err(ValidationError::TransactionCreateInputContract { index })?
                    }
                }

                let mut change_color_zero = false;
                let mut contract_created = false;
                for (index, output) in outputs.iter().enumerate() {
                    match output {
                        Output::Contract { .. } => {
                            Err(ValidationError::TransactionCreateOutputContract { index })?
                        }
                        Output::Variable { .. } => {
                            Err(ValidationError::TransactionCreateOutputVariable { index })?
                        }

                        Output::Change { color, .. }
                            if color == &Color::default() && change_color_zero =>
                        {
                            Err(ValidationError::TransactionCreateOutputChangeColorZero { index })?
                        }
                        Output::Change { color, .. } if color == &Color::default() => {
                            change_color_zero = true
                        }
                        Output::Change { .. } => {
                            Err(ValidationError::TransactionCreateOutputChangeColorNonZero {
                                index,
                            })?
                        }

                        Output::ContractCreated { .. } if contract_created => Err(
                            ValidationError::TransactionCreateOutputContractCreatedMultiple {
                                index,
                            },
                        )?,
                        Output::ContractCreated { .. } => contract_created = true,

                        _ => (),
                    }
                }

                Ok(())
            }
        }
    }
}
