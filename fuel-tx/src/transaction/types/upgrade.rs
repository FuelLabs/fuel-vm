use crate::{
    transaction::{
        id::PrepareSign,
        metadata::CommonMetadata,
        types::chargeable_transaction::{
            ChargeableMetadata,
            ChargeableTransaction,
            UniqueFormatValidityChecks,
        },
        Chargeable,
    },
    ConsensusParameters,
    GasCosts,
    Input,
    Output,
    TransactionRepr,
    ValidityError,
};
use educe::Educe;
use fuel_types::{
    bytes::WORD_SIZE,
    canonical::Serialize,
    Bytes32,
    ChainId,
    Word,
};

use fuel_crypto::Hasher;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

pub type Upgrade = ChargeableTransaction<UpgradeBody, UpgradeMetadata>;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UpgradeMetadata {
    /// The metadata for the upgrade transaction that changes the consensus parameters.
    ConsensusParameters {
        /// Deserialized consensus parameters from the witness.
        consensus_parameters: Box<ConsensusParameters>,
        /// The actual checksum of the serialized consensus parameters.
        calculated_checksum: Bytes32,
    },
    /// Currently there is no metadata for state transition upgrades, so leave it empty.
    #[default]
    StateTransition,
}

impl UpgradeMetadata {
    pub fn compute(tx: &Upgrade) -> Result<Self, ValidityError> {
        match &tx.body.purpose {
            UpgradePurpose::ConsensusParameters {
                witness_index,
                checksum,
            } => {
                let index = *witness_index as usize;
                let witness = tx
                    .witnesses
                    .get(index)
                    .ok_or(ValidityError::InputWitnessIndexBounds { index })?;

                let serialized_consensus_parameters = witness.as_vec();
                let actual_checksum = Hasher::hash(serialized_consensus_parameters);

                if &actual_checksum != checksum {
                    Err(ValidityError::TransactionUpgradeConsensusParametersChecksumMismatch)?;
                }

                // The code that creates/verifies the `Upgrade` transaction should always
                // be able to decode the current consensus parameters
                // type. The state transition function should always know
                // how to decode consensus parameters. Otherwise, the next
                // block will be impossible to produce. If deserialization fails, it is a
                // sign that the code/state transition function should be updated.
                let consensus_parameters = postcard::from_bytes::<ConsensusParameters>(
                    serialized_consensus_parameters,
                )
                .map_err(|_| {
                    ValidityError::TransactionUpgradeConsensusParametersDeserialization
                })?;

                Ok(Self::ConsensusParameters {
                    consensus_parameters: Box::new(consensus_parameters),
                    calculated_checksum: actual_checksum,
                })
            }
            UpgradePurpose::StateTransition { .. } => {
                // Nothing metadata for state transition upgrades.
                Ok(Self::StateTransition)
            }
        }
    }
}

/// The types describe the purpose of the upgrade performed by the [`Upgrade`]
/// transaction.
#[derive(
    Copy, Clone, Educe, strum_macros::EnumCount, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[educe(Eq, PartialEq, Hash, Debug)]
pub enum UpgradePurpose {
    /// The upgrade is performed to change the consensus parameters.
    ConsensusParameters {
        /// The index of the witness in the [`Witnesses`] field that contains
        /// the serialized consensus parameters.
        witness_index: u16,
        /// The hash of the serialized consensus parameters.
        /// Since the serialized consensus parameters live inside witnesses(malleable
        /// data), any party can override them. The `checksum` is used to verify that the
        /// data was not modified.
        checksum: Bytes32,
    },
    /// The upgrade is performed to change the state transition function.
    StateTransition {
        /// The Merkle root of the new bytecode of the state transition function.
        /// The bytecode must be present on the blockchain(should be known by the
        /// network) at the moment of inclusion of this transaction.
        root: Bytes32,
    },
}

/// The body of the [`Upgrade`] transaction.
#[derive(Clone, Educe, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Upgrade)]
#[educe(Eq, PartialEq, Hash, Debug)]
pub struct UpgradeBody {
    /// The purpose of the upgrade.
    pub(crate) purpose: UpgradePurpose,
}

impl Default for UpgradeBody {
    fn default() -> Self {
        Self {
            purpose: UpgradePurpose::StateTransition {
                root: Default::default(),
            },
        }
    }
}

impl PrepareSign for UpgradeBody {
    fn prepare_sign(&mut self) {}
}

impl Chargeable for Upgrade {
    #[inline(always)]
    fn metered_bytes_size(&self) -> usize {
        Serialize::size(self)
    }

    #[inline(always)]
    fn gas_used_by_metadata(&self, gas_cost: &GasCosts) -> Word {
        let bytes = Serialize::size(self);
        // Gas required to calculate the `tx_id`.
        let tx_id_gas = gas_cost.s256().resolve(bytes as u64);

        let purpose_gas = match &self.body.purpose {
            UpgradePurpose::ConsensusParameters { witness_index, .. } => {
                let len = self
                    .witnesses
                    .get(*witness_index as usize)
                    .map_or(0, |w| w.as_vec().len());
                gas_cost.s256().resolve(len as u64)
            }
            UpgradePurpose::StateTransition { .. } => {
                // In the case of the state transition upgrade, we only require the
                // existence of the bytecode on the blockchain. So we
                // verify nothing and charge nothing.
                0
            }
        };

        tx_id_gas.saturating_add(purpose_gas)
    }
}

impl UniqueFormatValidityChecks for Upgrade {
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        // At least one of inputs must be owned by the privileged address.
        self.inputs
            .iter()
            .find(|input| {
                if let Some(owner) = input.input_owner() {
                    owner == consensus_params.privileged_address()
                } else {
                    false
                }
            })
            .ok_or(ValidityError::TransactionUpgradeNoPrivilegedAddress)?;

        // We verify validity of the `UpgradePurpose` in the
        // `UpgradeMetadata::compute`.
        let calculated_metadata = UpgradeMetadata::compute(self)?;

        if let Some(metadata) = self.metadata.as_ref() {
            if metadata.body != calculated_metadata {
                return Err(ValidityError::TransactionMetadataMismatch);
            }
        }

        // The upgrade transaction cant touch the contract.
        self.inputs
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                if let Some(asset_id) = input.asset_id(consensus_params.base_asset_id()) {
                    if asset_id != consensus_params.base_asset_id() {
                        return Err(
                            ValidityError::TransactionInputContainsNonBaseAssetId {
                                index,
                            },
                        );
                    }
                }

                match input {
                    Input::Contract(_) => {
                        Err(ValidityError::TransactionInputContainsContract { index })
                    }
                    Input::MessageDataSigned(_) | Input::MessageDataPredicate(_) => {
                        Err(ValidityError::TransactionInputContainsMessageData { index })
                    }
                    _ => Ok(()),
                }
            })?;

        // The upgrade transaction can't create a contract.
        self.outputs
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| match output {
                Output::Contract(_) => {
                    Err(ValidityError::TransactionOutputContainsContract { index })
                }

                Output::Variable { .. } => {
                    Err(ValidityError::TransactionOutputContainsVariable { index })
                }

                Output::Change { asset_id, .. }
                    if asset_id != consensus_params.base_asset_id() =>
                {
                    Err(ValidityError::TransactionChangeChangeUsesNotBaseAsset { index })
                }

                Output::ContractCreated { .. } => {
                    Err(ValidityError::TransactionOutputContainsContractCreated { index })
                }
                _ => Ok(()),
            })?;

        Ok(())
    }
}

impl crate::Cacheable for Upgrade {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.metadata = None;
        self.metadata = Some(ChargeableMetadata {
            common: CommonMetadata::compute(self, chain_id)?,
            body: UpgradeMetadata::compute(self)?,
        });
        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::{
        ChargeableBody,
        UpgradePurpose as UpgradePurposeTrait,
    };

    impl UpgradePurposeTrait for Upgrade {
        #[inline(always)]
        fn upgrade_purpose(&self) -> &UpgradePurpose {
            &self.body.purpose
        }

        #[inline(always)]
        fn upgrade_purpose_mut(&mut self) -> &mut UpgradePurpose {
            &mut self.body.purpose
        }

        #[inline(always)]
        fn upgrade_purpose_offset_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl ChargeableBody<UpgradeBody> for Upgrade {
        fn body(&self) -> &UpgradeBody {
            &self.body
        }

        fn body_mut(&mut self) -> &mut UpgradeBody {
            &mut self.body
        }

        fn body_offset_end(&self) -> usize {
            Self::upgrade_purpose_offset_static()
                .saturating_add(self.body.purpose.size())
                .saturating_add(
                    WORD_SIZE  // Policies size
                    + WORD_SIZE // Inputs size
                    + WORD_SIZE // Outputs size
                    + WORD_SIZE, // Witnesses size
                )
        }
    }
}
