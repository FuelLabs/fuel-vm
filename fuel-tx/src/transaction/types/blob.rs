use crate::{
    transaction::{
        fee::min_gas,
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
    FeeParameters,
    GasCosts,
    Input,
    Output,
    TransactionRepr,
    ValidityError,
};
use derivative::Derivative;
use fuel_types::{
    bytes::WORD_SIZE,
    canonical::Serialize,
    ChainId,
    Word,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub type Blob = ChargeableTransaction<BlobBody, BlobMetadata>;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobMetadata;

/// The body of the [`Blob`] transaction.
#[derive(Clone, Default, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Blob)]
#[derivative(Eq, PartialEq, Hash, Debug)]
pub struct BlobBody {
    pub data: Vec<u8>,
}

impl PrepareSign for BlobBody {
    fn prepare_sign(&mut self) {}
}

impl Chargeable for Blob {
    fn min_gas(&self, gas_costs: &GasCosts, fee: &FeeParameters) -> fuel_asm::Word {
        min_gas(self, gas_costs, fee)
    }

    #[inline(always)]
    fn metered_bytes_size(&self) -> usize {
        Serialize::size(self)
    }

    #[inline(always)]
    fn gas_used_by_metadata(&self, gas_cost: &GasCosts) -> Word {
        let bytes = Serialize::size(self);
        // Gas required to calculate the `tx_id`.
        gas_cost.s256().resolve(bytes as u64)
    }
}

impl UniqueFormatValidityChecks for Blob {
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
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

                Output::Change { asset_id, .. } => {
                    if asset_id != consensus_params.base_asset_id() {
                        Err(ValidityError::TransactionChangeChangeUsesNotBaseAsset {
                            index,
                        })
                    } else {
                        Ok(())
                    }
                }

                Output::ContractCreated { .. } => {
                    Err(ValidityError::TransactionOutputContainsContractCreated { index })
                }
                Output::Coin { .. } => {
                    Err(ValidityError::TransactionOutputContainsCoin { index })
                }
            })?;

        Ok(())
    }
}

impl crate::Cacheable for Blob {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.metadata = None;
        self.metadata = Some(ChargeableMetadata {
            common: CommonMetadata::compute(self, chain_id)?,
            body: BlobMetadata {},
        });
        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::ChargeableBody;

    impl ChargeableBody<BlobBody> for Blob {
        fn body(&self) -> &BlobBody {
            &self.body
        }

        fn body_mut(&mut self) -> &mut BlobBody {
            &mut self.body
        }

        fn body_offset_end(&self) -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }
}