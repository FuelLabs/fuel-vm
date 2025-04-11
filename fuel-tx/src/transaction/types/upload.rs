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
    TxId,
    ValidityError,
};
use core::ops::Deref;
use educe::Educe;
use fuel_types::{
    bytes::WORD_SIZE,
    canonical::Serialize,
    Bytes32,
    ChainId,
    Word,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub type Upload = ChargeableTransaction<UploadBody, UploadMetadata>;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UploadMetadata;

/// The body of the [`Upload`] transaction.
#[derive(Clone, Default, Educe, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Upload)]
#[educe(Eq, PartialEq, Hash, Debug)]
pub struct UploadBody {
    /// The root of the Merkle tree is created over the bytecode.
    pub root: Bytes32,
    /// The witness index of the subsection of the bytecode.
    pub witness_index: u16,
    /// The index of the subsection of the bytecode.
    pub subsection_index: u16,
    /// The total number of subsections on which bytecode was divided.
    pub subsections_number: u16,
    /// The proof set helps to verify the connection of the subsection to the `root`.
    pub proof_set: Vec<Bytes32>,
}

#[derive(
    Clone, Default, Eq, PartialEq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct UploadSubsection {
    /// The root of the Merkle tree is created over the bytecode.
    pub root: Bytes32,
    /// The subsection of the bytecode.
    pub subsection: Vec<u8>,
    /// The index of the subsection.
    pub subsection_index: u16,
    /// The total number of subsections on which bytecode was divided.
    pub subsections_number: u16,
    /// The proof set helps to verify the connection of the subsection to the `root`.
    pub proof_set: Vec<Bytes32>,
}

#[derive(
    Copy, Clone, Eq, PartialEq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SplitError {
    /// The size of the subsection is too small to fit all subsections into `u16::MAX`.
    SubsectionSizeTooSmall,
}

impl UploadSubsection {
    /// Splits the bytecode into verifiable subsections and returns a vector of
    /// [`UploadSubsection`]s.
    pub fn split_bytecode(
        bytecode: &[u8],
        subsection_size: usize,
    ) -> Result<Vec<UploadSubsection>, SplitError> {
        let subsections = bytecode
            .chunks(subsection_size)
            .map(|subsection| subsection.to_vec())
            .collect::<Vec<_>>();

        if subsections.len() > u16::MAX as usize {
            return Err(SplitError::SubsectionSizeTooSmall);
        }
        let subsections_number =
            u16::try_from(subsections.len()).expect("We've just checked it; qed");

        let mut merkle_tree = fuel_merkle::binary::in_memory::MerkleTree::new();
        subsections
            .iter()
            .for_each(|subsection| merkle_tree.push(subsection));

        let merkle_root = merkle_tree.root();

        let subsections = subsections
            .into_iter()
            .enumerate()
            .map(|(index, subsection)| {
                let (root, proof_set) = merkle_tree
                    .prove(index as u64)
                    .expect("We've just created a merkle tree, so it is valid; qed");
                debug_assert_eq!(root, merkle_root);

                UploadSubsection {
                    root: merkle_root.into(),
                    subsection,
                    subsection_index: u16::try_from(index).expect(
                        "The total number of subsections is less than u16::MAX; qed",
                    ),
                    subsections_number,
                    proof_set: proof_set.into_iter().map(Into::into).collect(),
                }
            })
            .collect();

        Ok(subsections)
    }
}

impl PrepareSign for UploadBody {
    fn prepare_sign(&mut self) {}
}

impl Chargeable for Upload {
    fn min_gas(&self, gas_costs: &GasCosts, fee: &FeeParameters) -> fuel_asm::Word {
        let bytecode_len = self
            .witnesses
            .get(self.body.witness_index as usize)
            .map(|c| c.as_ref().len())
            .unwrap_or(0);

        // Since the `Upload` transaction occupies much of the storage, we want to
        // discourage people from using it too much. For that, we charge additional gas
        // for the storage.
        let additional_charge_for_storage = gas_costs
            .new_storage_per_byte()
            .saturating_mul(bytecode_len as u64);

        min_gas(self, gas_costs, fee).saturating_add(additional_charge_for_storage)
    }

    #[inline(always)]
    fn metered_bytes_size(&self) -> usize {
        Serialize::size(self)
    }

    #[inline(always)]
    fn gas_used_by_metadata(&self, gas_cost: &GasCosts) -> Word {
        let bytes = Serialize::size(self);
        // Gas required to calculate the `tx_id`.
        let tx_id_gas = gas_cost.s256().resolve(bytes as u64);

        let bytecode_len = self
            .witnesses
            .get(self.body.witness_index as usize)
            .map(|c| c.as_ref().len())
            .unwrap_or(0);

        let leaf_hash_gas = gas_cost.s256().resolve(bytecode_len as u64);
        let verify_proof_gas = gas_cost
            .state_root()
            .resolve(self.body.subsections_number as u64);

        tx_id_gas
            .saturating_add(leaf_hash_gas)
            .saturating_add(verify_proof_gas)
    }
}

impl UniqueFormatValidityChecks for Upload {
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        if self.body.subsections_number
            > consensus_params.tx_params().max_bytecode_subsections()
        {
            return Err(ValidityError::TransactionUploadTooManyBytecodeSubsections);
        }

        let index = self.body.witness_index as usize;
        let witness = self
            .witnesses
            .get(index)
            .ok_or(ValidityError::InputWitnessIndexBounds { index })?;

        let proof_set = self
            .body
            .proof_set
            .iter()
            .map(|proof| (*proof).into())
            .collect::<Vec<_>>();

        // Verify that subsection of the bytecode is connected to the `root` of the
        // bytecode.
        let result = fuel_merkle::binary::verify(
            self.body.root.deref(),
            witness,
            &proof_set,
            self.body.subsection_index as u64,
            self.body.subsections_number as u64,
        );

        if !result {
            return Err(ValidityError::TransactionUploadRootVerificationFailed);
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

impl crate::Cacheable for Upload {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), (TxId, ValidityError)> {
        self.metadata = None;
        self.metadata = Some(ChargeableMetadata {
            common: CommonMetadata::compute(self, chain_id)?,
            body: UploadMetadata {},
        });
        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::{
        BytecodeRoot,
        BytecodeWitnessIndex,
        ChargeableBody,
        ProofSet,
        SubsectionIndex,
        SubsectionsNumber,
    };

    impl BytecodeRoot for Upload {
        #[inline(always)]
        fn bytecode_root(&self) -> &Bytes32 {
            &self.body.root
        }

        #[inline(always)]
        fn bytecode_root_mut(&mut self) -> &mut Bytes32 {
            &mut self.body.root
        }

        #[inline(always)]
        fn bytecode_root_offset_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl BytecodeWitnessIndex for Upload {
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
            Self::bytecode_root_offset_static().saturating_add(Bytes32::LEN)
        }
    }

    impl SubsectionIndex for Upload {
        #[inline(always)]
        fn subsection_index(&self) -> &u16 {
            &self.body.subsection_index
        }

        #[inline(always)]
        fn subsection_index_mut(&mut self) -> &mut u16 {
            &mut self.body.subsection_index
        }

        #[inline(always)]
        fn subsection_index_offset_static() -> usize {
            Self::bytecode_witness_index_offset_static().saturating_add(WORD_SIZE)
        }
    }

    impl SubsectionsNumber for Upload {
        #[inline(always)]
        fn subsections_number(&self) -> &u16 {
            &self.body.subsections_number
        }

        #[inline(always)]
        fn subsections_number_mut(&mut self) -> &mut u16 {
            &mut self.body.subsections_number
        }

        #[inline(always)]
        fn subsections_number_offset_static() -> usize {
            Self::subsection_index_offset_static().saturating_add(WORD_SIZE)
        }
    }

    impl ProofSet for Upload {
        #[inline(always)]
        fn proof_set(&self) -> &Vec<Bytes32> {
            &self.body.proof_set
        }

        #[inline(always)]
        fn proof_set_mut(&mut self) -> &mut Vec<Bytes32> {
            &mut self.body.proof_set
        }

        #[inline(always)]
        fn proof_set_offset_static() -> usize {
            Self::subsections_number_offset_static().saturating_add(
                WORD_SIZE
                + WORD_SIZE // Proof set size
                + WORD_SIZE // Policies size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE, // Witnesses size
            )
        }

        #[inline(always)]
        fn proof_set_offset_at(&self, idx: usize) -> Option<usize> {
            if idx < self.body.proof_set.len() {
                Some(
                    Self::proof_set_offset_static()
                        .checked_add(idx.checked_mul(Bytes32::LEN)?)?,
                )
            } else {
                None
            }
        }
    }

    impl ChargeableBody<UploadBody> for Upload {
        fn body(&self) -> &UploadBody {
            &self.body
        }

        fn body_mut(&mut self) -> &mut UploadBody {
            &mut self.body
        }

        fn body_offset_end(&self) -> usize {
            Self::proof_set_offset_static()
                .saturating_add(self.body.proof_set.len().saturating_mul(Bytes32::LEN))
        }
    }
}
