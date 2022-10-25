use super::{Input, Output, Transaction, Witness};
use core::hash::Hash;

use fuel_types::{AssetId, Word};

#[cfg(feature = "std")]
use fuel_types::Bytes32;

#[cfg(feature = "std")]
use fuel_crypto::{Message, Signature};
use itertools::Itertools;

mod error;

use crate::transaction::consensus_parameters::ConsensusParameters;
use crate::transaction::{field, Executable};
pub use error::CheckError;

impl Input {
    #[cfg(feature = "std")]
    pub fn check(
        &self,
        index: usize,
        txhash: &Bytes32,
        outputs: &[Output],
        witnesses: &[Witness],
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        self.check_without_signature(index, outputs, witnesses, parameters)?;
        self.check_signature(index, txhash, witnesses)?;

        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn check_signature(
        &self,
        index: usize,
        txhash: &Bytes32,
        witnesses: &[Witness],
    ) -> Result<(), CheckError> {
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
                    .ok_or(CheckError::InputWitnessIndexBounds { index })?
                    .as_ref();

                if witness.len() != Signature::LEN {
                    return Err(CheckError::InputInvalidSignature { index });
                }

                // Safety: checked length
                let signature = unsafe { Signature::as_ref_unchecked(witness) };

                // Safety: checked length
                let message = unsafe { Message::as_ref_unchecked(txhash.as_ref()) };

                let pk = signature
                    .recover(message)
                    .map_err(|_| CheckError::InputInvalidSignature { index })
                    .map(|pk| Input::owner(&pk))?;

                if owner != &pk {
                    return Err(CheckError::InputInvalidSignature { index });
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
                Err(CheckError::InputPredicateOwner { index })
            }

            _ => Ok(()),
        }
    }

    pub fn check_without_signature(
        &self,
        index: usize,
        outputs: &[Output],
        witnesses: &[Witness],
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        match self {
            Self::CoinPredicate { predicate, .. } | Self::MessagePredicate { predicate, .. }
                if predicate.is_empty() =>
            {
                Err(CheckError::InputPredicateEmpty { index })
            }

            Self::CoinPredicate { predicate, .. } | Self::MessagePredicate { predicate, .. }
                if predicate.len() > parameters.max_predicate_length as usize =>
            {
                Err(CheckError::InputPredicateLength { index })
            }

            Self::CoinPredicate { predicate_data, .. }
            | Self::MessagePredicate { predicate_data, .. }
                if predicate_data.len() > parameters.max_predicate_data_length as usize =>
            {
                Err(CheckError::InputPredicateDataLength { index })
            }

            Self::CoinSigned { witness_index, .. } | Self::MessageSigned { witness_index, .. }
                if *witness_index as usize >= witnesses.len() =>
            {
                Err(CheckError::InputWitnessIndexBounds { index })
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
                Err(CheckError::InputContractAssociatedOutputContract { index })
            }

            Self::MessageSigned { data, .. } | Self::MessagePredicate { data, .. }
                if data.len() > parameters.max_message_data_length as usize =>
            {
                Err(CheckError::InputMessageDataLength { index })
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
    pub fn check(&self, index: usize, inputs: &[Input]) -> Result<(), CheckError> {
        match self {
            Self::Contract { input_index, .. } => match inputs.get(*input_index as usize) {
                Some(Input::Contract { .. }) => Ok(()),
                _ => Err(CheckError::OutputContractInputIndex { index }),
            },

            _ => Ok(()),
        }
    }
}

/// Means that the transaction can be validated.
pub trait Checkable {
    #[cfg(feature = "std")]
    /// Fully validates the transaction. It checks the validity of fields according to rules in
    /// the specification and validity of signatures.
    fn check(
        &self,
        block_height: Word,
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        self.check_without_signatures(block_height, parameters)?;
        self.check_signatures()?;

        Ok(())
    }

    #[cfg(feature = "std")]
    /// Validates that all required signatures are set in the transaction and that they are valid.
    fn check_signatures(&self) -> Result<(), CheckError>;

    /// Validates the transactions according to rules from the specification:
    /// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
    fn check_without_signatures(
        &self,
        block_height: Word,
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError>;
}

impl Checkable for Transaction {
    #[cfg(feature = "std")]
    fn check_signatures(&self) -> Result<(), CheckError> {
        match self {
            Transaction::Script(script) => script.check_signatures(),
            Transaction::Create(create) => create.check_signatures(),
        }
    }

    fn check_without_signatures(
        &self,
        block_height: Word,
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        match self {
            Transaction::Script(script) => {
                script.check_without_signatures(block_height, parameters)
            }
            Transaction::Create(create) => {
                create.check_without_signatures(block_height, parameters)
            }
        }
    }
}

pub(crate) fn check_common_part<T>(
    tx: &T,
    block_height: Word,
    parameters: &ConsensusParameters,
) -> Result<(), CheckError>
where
    T: field::GasPrice
        + field::GasLimit
        + field::Maturity
        + field::Inputs
        + field::Outputs
        + field::Witnesses,
{
    if tx.gas_limit() > &parameters.max_gas_per_tx {
        Err(CheckError::TransactionGasLimit)?
    }

    if tx.maturity() > &block_height {
        Err(CheckError::TransactionMaturity)?;
    }

    if tx.inputs().len() > parameters.max_inputs as usize {
        Err(CheckError::TransactionInputsMax)?
    }

    if tx.outputs().len() > parameters.max_outputs as usize {
        Err(CheckError::TransactionOutputsMax)?
    }

    if tx.witnesses().len() > parameters.max_witnesses as usize {
        Err(CheckError::TransactionWitnessesMax)?
    }

    tx.input_asset_ids_unique().try_for_each(|input_asset_id| {
        // check for duplicate change outputs
        if tx
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
            return Err(CheckError::TransactionOutputChangeAssetIdDuplicated);
        }

        Ok(())
    })?;

    // Check for duplicated input utxo id
    let duplicated_utxo_id = tx
        .inputs()
        .iter()
        .filter_map(|i| i.is_coin().then(|| i.utxo_id()).flatten());

    if let Some(utxo_id) = next_duplicate(duplicated_utxo_id).copied() {
        return Err(CheckError::DuplicateInputUtxoId { utxo_id });
    }

    // Check for duplicated input contract id
    let duplicated_contract_id = tx.inputs().iter().filter_map(Input::contract_id);

    if let Some(contract_id) = next_duplicate(duplicated_contract_id).copied() {
        return Err(CheckError::DuplicateInputContractId { contract_id });
    }

    // Check for duplicated input message id
    let duplicated_message_id = tx.inputs().iter().filter_map(Input::message_id);
    if let Some(message_id) = next_duplicate(duplicated_message_id).copied() {
        return Err(CheckError::DuplicateMessageInputId { message_id });
    }

    // Validate the inputs without checking signature
    tx.inputs()
        .iter()
        .enumerate()
        .try_for_each(|(index, input)| {
            input.check_without_signature(index, tx.outputs(), tx.witnesses(), parameters)
        })?;

    tx.outputs()
        .iter()
        .enumerate()
        .try_for_each(|(index, output)| {
            output.check(index, tx.inputs())?;

            if let Output::Change { asset_id, .. } = output {
                if !tx
                    .input_asset_ids()
                    .any(|input_asset_id| input_asset_id == asset_id)
                {
                    return Err(CheckError::TransactionOutputChangeAssetIdNotFound(
                        *asset_id,
                    ));
                }
            }

            if let Output::Coin { asset_id, .. } = output {
                if !tx
                    .input_asset_ids()
                    .any(|input_asset_id| input_asset_id == asset_id)
                {
                    return Err(CheckError::TransactionOutputCoinAssetIdNotFound(*asset_id));
                }
            }

            Ok(())
        })?;

    Ok(())
}

// TODO https://github.com/FuelLabs/fuel-tx/issues/148
pub(crate) fn next_duplicate<U>(iter: impl Iterator<Item = U>) -> Option<U>
where
    U: PartialEq + Ord + Copy + Hash,
{
    #[cfg(not(feature = "std"))]
    return iter
        .sorted()
        .as_slice()
        .windows(2)
        .filter_map(|u| (u[0] == u[1]).then(|| u[0]))
        .next();

    #[cfg(feature = "std")]
    return iter.duplicates().next();
}
