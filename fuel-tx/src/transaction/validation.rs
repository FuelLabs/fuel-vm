use super::{Input, Output, Transaction, Witness};
use crate::consts::*;

use fuel_types::{AssetId, Word};

#[cfg(feature = "std")]
use fuel_types::Bytes32;

#[cfg(feature = "std")]
use fuel_crypto::{Message, Signature};

mod error;

pub use error::ValidationError;

impl Input {
    #[cfg(feature = "std")]
    pub fn validate(
        &self,
        index: usize,
        txhash: &Bytes32,
        outputs: &[Output],
        witnesses: &[Witness],
    ) -> Result<(), ValidationError> {
        self.validate_without_signature(index, outputs, witnesses)?;
        self.validate_signature(index, txhash, witnesses)?;

        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn validate_signature(
        &self,
        index: usize,
        txhash: &Bytes32,
        witnesses: &[Witness],
    ) -> Result<(), ValidationError> {
        if let Input::Coin {
            witness_index,
            owner,
            ..
        } = self
        {
            let witness = witnesses
                .get(*witness_index as usize)
                .ok_or(ValidationError::InputCoinWitnessIndexBounds { index })?
                .as_ref();

            if witness.len() != Signature::LEN {
                return Err(ValidationError::InputCoinInvalidSignature { index });
            }

            // Safety: checked length
            let signature = unsafe { Signature::as_ref_unchecked(witness) };

            // Safety: checked length
            let message = unsafe { Message::as_ref_unchecked(txhash.as_ref()) };

            let pk = signature
                .recover(message)
                .map_err(|_| ValidationError::InputCoinInvalidSignature { index })
                .map(|pk| Input::coin_owner(&pk))?;

            if owner != &pk {
                return Err(ValidationError::InputCoinInvalidSignature { index });
            }
        }

        Ok(())
    }

    pub fn validate_without_signature(
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

            Self::Coin { .. } => Ok(()),

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
    #[cfg(feature = "std")]
    pub fn validate(&self, block_height: Word) -> Result<(), ValidationError> {
        self.validate_without_signature(block_height)?;
        self.validate_input_signature()?;

        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn validate_input_signature(&self) -> Result<(), ValidationError> {
        let id = self.id();

        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                input.validate_signature(index, &id, self.witnesses())
            })?;

        Ok(())
    }

    pub fn validate_without_signature(&self, block_height: Word) -> Result<(), ValidationError> {
        if self.gas_limit() > MAX_GAS_PER_TX {
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

        self.input_asset_ids_unique()
            .try_for_each(|input_asset_id| {
                // check for duplicate change outputs
                if self
                    .outputs()
                    .iter()
                    .filter_map(|output| match output {
                        Output::Change { asset_id, .. } if input_asset_id == asset_id => Some(()),
                        Output::Change { asset_id, .. }
                            if asset_id != &AssetId::default() && input_asset_id == asset_id =>
                        {
                            Some(())
                        }
                        _ => None,
                    })
                    .count()
                    > 1
                {
                    return Err(ValidationError::TransactionOutputChangeAssetIdDuplicated);
                }

                Ok(())
            })?;

        // check for duplicate coin inputs
        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                match input {
                    Input::Coin { utxo_id, .. } => {
                        if self
                            .inputs()
                            .iter()
                            .filter(|input| input.is_coin())
                            .filter(|other_input| other_input.utxo_id() == utxo_id)
                            .count()
                            > 1
                        {
                            return Err(ValidationError::DuplicateInputUtxoId {
                                utxo_id: *input.utxo_id(),
                            });
                        }
                    }
                    Input::Contract { contract_id, .. } => {
                        if self
                            .inputs()
                            .iter()
                            .filter(|other_input| {
                                if let Input::Contract {
                                    contract_id: other_contract_id,
                                    ..
                                } = other_input
                                {
                                    other_contract_id == contract_id
                                } else {
                                    false
                                }
                            })
                            .count()
                            > 1
                        {
                            return Err(ValidationError::DuplicateInputContractId {
                                contract_id: *contract_id,
                            });
                        }
                    }
                }

                input.validate_without_signature(index, self.outputs(), self.witnesses())
            })?;

        self.outputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| {
                output.validate(index, self.inputs())?;

                if let Output::Change { asset_id, .. } = output {
                    if !self
                        .input_asset_ids()
                        .any(|input_asset_id| input_asset_id == asset_id)
                    {
                        return Err(ValidationError::TransactionOutputChangeAssetIdNotFound(
                            *asset_id,
                        ));
                    }
                }

                if let Output::Coin { asset_id, .. } = output {
                    if !self
                        .input_asset_ids()
                        .any(|input_asset_id| input_asset_id == asset_id)
                    {
                        return Err(ValidationError::TransactionOutputCoinAssetIdNotFound(
                            *asset_id,
                        ));
                    }
                }

                Ok(())
            })?;

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
                bytecode_length,
                bytecode_witness_index,
                static_contracts,
                storage_slots,
                ..
            } => {
                let bytecode_witness_len = witnesses
                    .get(*bytecode_witness_index as usize)
                    .map(|w| w.as_ref().len() as Word)
                    .ok_or(ValidationError::TransactionCreateBytecodeWitnessIndex)?;

                if bytecode_witness_len > CONTRACT_MAX_SIZE
                    || bytecode_witness_len / 4 != *bytecode_length
                {
                    return Err(ValidationError::TransactionCreateBytecodeLen);
                }

                if static_contracts.len() > MAX_STATIC_CONTRACTS as usize {
                    Err(ValidationError::TransactionCreateStaticContractsMax)?;
                }

                if !static_contracts.as_slice().windows(2).all(|w| w[0] <= w[1]) {
                    Err(ValidationError::TransactionCreateStaticContractsOrder)?;
                }

                // Restrict to subset of u16::MAX, allowing this to be increased in the future
                // in a non-breaking way.
                if storage_slots.len() > MAX_STORAGE_SLOTS as usize {
                    return Err(ValidationError::TransactionCreateStorageSlotMax);
                }

                if !storage_slots.as_slice().windows(2).all(|s| s[0] <= s[1]) {
                    return Err(ValidationError::TransactionCreateStorageSlotOrder);
                }

                // TODO Any contract with ADDRESS in staticContracts is not in the state
                // TODO The computed contract ADDRESS (see below) is not equal to the
                // contractADDRESS of the one OutputType.ContractCreated output

                inputs.iter().enumerate().try_for_each(|(index, input)| {
                    if let Input::Contract { .. } = input {
                        return Err(ValidationError::TransactionCreateInputContract { index });
                    }

                    Ok(())
                })?;

                let mut contract_created = false;
                outputs
                    .iter()
                    .enumerate()
                    .try_for_each(|(index, output)| match output {
                        Output::Contract { .. } => {
                            Err(ValidationError::TransactionCreateOutputContract { index })
                        }
                        Output::Variable { .. } => {
                            Err(ValidationError::TransactionCreateOutputVariable { index })
                        }

                        Output::Change { asset_id, .. } if asset_id != &AssetId::default() => {
                            Err(ValidationError::TransactionCreateOutputChangeNotBaseAsset {
                                index,
                            })
                        }

                        Output::ContractCreated { .. } if contract_created => Err(
                            ValidationError::TransactionCreateOutputContractCreatedMultiple {
                                index,
                            },
                        ),

                        Output::ContractCreated { .. } => {
                            contract_created = true;

                            Ok(())
                        }

                        _ => Ok(()),
                    })?;

                Ok(())
            }
        }
    }
}
