use crate::{
    ConsensusParameters,
    FeeParameters,
    GasCosts,
    Input,
    Output,
    TransactionRepr,
    ValidityError,
    transaction::{
        Chargeable,
        fee::min_gas,
        id::PrepareSign,
        metadata::CommonMetadata,
        types::chargeable_transaction::{
            ChargeableMetadata,
            ChargeableTransaction,
            UniqueFormatValidityChecks,
        },
    },
};
use educe::Educe;
use fuel_types::{
    BlobId,
    ChainId,
    Word,
    bytes::WORD_SIZE,
    canonical::Serialize,
};

/// Adds method to `BlobId` to compute the it from blob data.
pub trait BlobIdExt {
    /// Computes the `BlobId` from by hashing the given data.
    fn compute(data: &[u8]) -> BlobId;
}

impl BlobIdExt for BlobId {
    fn compute(data: &[u8]) -> Self {
        Self::new(*fuel_crypto::Hasher::hash(data))
    }
}

pub type Blob = ChargeableTransaction<BlobBody, BlobMetadata>;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobMetadata;

/// The body of the [`Blob`] transaction.
#[derive(Clone, Default, Educe, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Blob)]
#[educe(Eq, PartialEq, Hash, Debug)]
pub struct BlobBody {
    /// Hash of the bytecode. Used both as a unique identifier and to verify the
    /// bytecode.
    pub id: BlobId,
    /// The witness index of the payload.
    pub witness_index: u16,
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
        let blob_len = self
            .witnesses
            .get(self.body.witness_index as usize)
            .map(|c| c.as_ref().len())
            .unwrap_or(0);

        // Gas required to calculate the `tx_id` and `blob_id`.
        gas_cost
            .s256()
            .resolve(bytes as u64)
            .saturating_add(gas_cost.s256().resolve(blob_len as u64))
    }
}

impl UniqueFormatValidityChecks for Blob {
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        let index = self.body.witness_index as usize;
        let witness = self
            .witnesses
            .get(index)
            .ok_or(ValidityError::InputWitnessIndexBounds { index })?;

        // Verify that blob id is correct
        if BlobId::compute(witness.as_ref()) != self.body.id {
            return Err(ValidityError::TransactionBlobIdVerificationFailed);
        }

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

                Output::Coin { .. } => Ok(()),
                Output::DataCoin { .. } => Ok(()),
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
    use crate::field::{
        self,
        BlobId as BlobIdField,
        BytecodeWitnessIndex,
    };

    impl field::BlobId for Blob {
        #[inline(always)]
        fn blob_id(&self) -> &BlobId {
            &self.body.id
        }

        #[inline(always)]
        fn blob_id_mut(&mut self) -> &mut BlobId {
            &mut self.body.id
        }

        #[inline(always)]
        fn blob_id_offset_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl field::BytecodeWitnessIndex for Blob {
        #[inline(always)]
        fn bytecode_witness_index(&self) -> &u16 {
            &self.body.witness_index
        }

        #[inline(always)]
        fn bytecode_witness_index_mut(&mut self) -> &mut u16 {
            &mut self.body.witness_index
        }

        #[inline(always)]
        fn bytecode_witness_index_offset_static() -> usize {
            Self::blob_id_offset_static().saturating_add(BlobId::LEN)
        }
    }

    impl field::ChargeableBody<BlobBody> for Blob {
        fn body(&self) -> &BlobBody {
            &self.body
        }

        fn body_mut(&mut self) -> &mut BlobBody {
            &mut self.body
        }

        #[allow(clippy::arithmetic_side_effects)] // Statically known to be ok
        fn body_offset_end(&self) -> usize {
            Self::bytecode_witness_index_offset_static().saturating_add(
                WORD_SIZE // witness_index
                + WORD_SIZE // Policies size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE, // Witnesses size
            )
        }
    }
}
