use super::{Input, Output, Transaction, Witness};
use crate::transaction::internals;

use fuel_types::{AssetId, Word};

#[cfg(feature = "std")]
use fuel_types::Bytes32;

#[cfg(feature = "std")]
use fuel_crypto::{Message, Signature};

mod error;

use crate::transaction::consensus_parameters::ConsensusParameters;
pub use error::ValidationError;

impl Input {
    #[cfg(feature = "std")]
    pub fn validate(
        &self,
        index: usize,
        txhash: &Bytes32,
        outputs: &[Output],
        witnesses: &[Witness],
        parameters: &ConsensusParameters,
    ) -> Result<(), ValidationError> {
        self.validate_without_signature(index, outputs, witnesses, parameters)?;
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
        match self {
            Self::CoinSigned {
                witness_index,
                owner,
                ..
            }
            | Self::MessageSigned {
                witness_index,
                recipient: owner,
                ..
            } => {
                let witness = witnesses
                    .get(*witness_index as usize)
                    .ok_or(ValidationError::InputWitnessIndexBounds { index })?
                    .as_ref();

                if witness.len() != Signature::LEN {
                    return Err(ValidationError::InputInvalidSignature { index });
                }

                // Safety: checked length
                let signature = unsafe { Signature::as_ref_unchecked(witness) };

                // Safety: checked length
                let message = unsafe { Message::as_ref_unchecked(txhash.as_ref()) };

                let pk = signature
                    .recover(message)
                    .map_err(|_| ValidationError::InputInvalidSignature { index })
                    .map(|pk| Input::owner(&pk))?;

                if owner != &pk {
                    return Err(ValidationError::InputInvalidSignature { index });
                }

                Ok(())
            }

            Self::CoinPredicate {
                owner, predicate, ..
            }
            | Self::MessagePredicate {
                recipient: owner,
                predicate,
                ..
            } if !Input::is_predicate_owner_valid(owner, predicate) => {
                Err(ValidationError::InputPredicateOwner { index })
            }

            _ => Ok(()),
        }
    }

    pub fn validate_without_signature(
        &self,
        index: usize,
        outputs: &[Output],
        witnesses: &[Witness],
        parameters: &ConsensusParameters,
    ) -> Result<(), ValidationError> {
        match self {
            Self::CoinPredicate { predicate, .. } | Self::MessagePredicate { predicate, .. }
                if predicate.is_empty() =>
            {
                Err(ValidationError::InputPredicateEmpty { index })
            }

            Self::CoinPredicate { predicate, .. } | Self::MessagePredicate { predicate, .. }
                if predicate.len() > parameters.max_predicate_length as usize =>
            {
                Err(ValidationError::InputPredicateLength { index })
            }

            Self::CoinPredicate { predicate_data, .. }
            | Self::MessagePredicate { predicate_data, .. }
                if predicate_data.len() > parameters.max_predicate_data_length as usize =>
            {
                Err(ValidationError::InputPredicateDataLength { index })
            }

            Self::CoinSigned { witness_index, .. } | Self::MessageSigned { witness_index, .. }
                if *witness_index as usize >= witnesses.len() =>
            {
                Err(ValidationError::InputWitnessIndexBounds { index })
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

            Self::MessageSigned { data, .. } | Self::MessagePredicate { data, .. }
                if data.len() > parameters.max_message_data_length as usize =>
            {
                Err(ValidationError::InputMessageDataLength { index })
            }

            // TODO If h is the block height the UTXO being spent was created, transaction is
            // invalid if `blockheight() < h + maturity`.
            _ => Ok(()),
        }
    }
}

impl Output {
    /// Validate the output of the transaction.
    ///
    /// This function is stateful - meaning it might validate a transaction during VM
    /// initialization, but this transaction will no longer be valid in post-execution because the
    /// VM might mutate the message outputs, producing invalid transactions.
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
    /// Validate the transaction.
    ///
    /// This function will reflect the stateful property of [`Output::validate`]
    #[cfg(feature = "std")]
    pub fn validate(
        &self,
        block_height: Word,
        parameters: &ConsensusParameters,
    ) -> Result<(), ValidationError> {
        self.validate_without_signature(block_height, parameters)?;
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

    /// Validate the transaction.
    ///
    /// This function will reflect the stateful property of [`Output::validate`]
    pub fn validate_without_signature(
        &self,
        block_height: Word,
        parameters: &ConsensusParameters,
    ) -> Result<(), ValidationError> {
        if self.gas_limit() > parameters.max_gas_per_tx {
            Err(ValidationError::TransactionGasLimit)?
        }

        if block_height < self.maturity() as Word {
            Err(ValidationError::TransactionMaturity)?;
        }

        if self.inputs().len() > parameters.max_inputs as usize {
            Err(ValidationError::TransactionInputsMax)?
        }

        if self.outputs().len() > parameters.max_outputs as usize {
            Err(ValidationError::TransactionOutputsMax)?
        }

        if self.witnesses().len() > parameters.max_witnesses as usize {
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

        // Check for duplicated input utxo id
        let duplicated_utxo_id = self
            .inputs()
            .iter()
            .filter_map(|i| i.is_coin().then(|| i.utxo_id()).flatten());

        if let Some(utxo_id) = internals::next_duplicate(duplicated_utxo_id).copied() {
            return Err(ValidationError::DuplicateInputUtxoId { utxo_id });
        }

        // Check for duplicated input contract id
        let duplicated_contract_id = self.inputs().iter().filter_map(Input::contract_id);

        if let Some(contract_id) = internals::next_duplicate(duplicated_contract_id).copied() {
            return Err(ValidationError::DuplicateInputContractId { contract_id });
        }

        // Check for duplicated input message id
        let duplicated_message_id = self.inputs().iter().filter_map(Input::message_id);
        if let Some(message_id) = internals::next_duplicate(duplicated_message_id).copied() {
            return Err(ValidationError::DuplicateMessageInputId { message_id });
        }

        // Validate the inputs without checking signature
        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                input.validate_without_signature(
                    index,
                    self.outputs(),
                    self.witnesses(),
                    parameters,
                )
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
                if script.len() > parameters.max_script_length as usize {
                    Err(ValidationError::TransactionScriptLength)?;
                }

                if script_data.len() > parameters.max_script_data_length as usize {
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
                storage_slots,
                ..
            } => {
                let bytecode_witness_len = witnesses
                    .get(*bytecode_witness_index as usize)
                    .map(|w| w.as_ref().len() as Word)
                    .ok_or(ValidationError::TransactionCreateBytecodeWitnessIndex)?;

                if bytecode_witness_len > parameters.contract_max_size
                    || bytecode_witness_len / 4 != *bytecode_length
                {
                    return Err(ValidationError::TransactionCreateBytecodeLen);
                }

                // Restrict to subset of u16::MAX, allowing this to be increased in the future
                // in a non-breaking way.
                if storage_slots.len() > parameters.max_storage_slots as usize {
                    return Err(ValidationError::TransactionCreateStorageSlotMax);
                }

                if !storage_slots.as_slice().windows(2).all(|s| s[0] <= s[1]) {
                    return Err(ValidationError::TransactionCreateStorageSlotOrder);
                }

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
