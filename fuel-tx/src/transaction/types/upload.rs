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
use core::ops::Deref;
use derivative::Derivative;
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
#[derive(Clone, Default, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Upload)]
#[derivative(Eq, PartialEq, Hash, Debug)]
pub struct UploadBody {
    /// The root of the Merkle tree is created over the bytecode.
    pub root: Bytes32,
    /// The witness index of the part of the bytecode.
    pub witness_index: u16,
    /// The index of the part.
    pub part_index: u16,
    /// The total number of parts on which bytecode was divided.
    pub parts_number: u16,
    /// The proof set helps to verify the connection of the part to the `root`.
    pub proof_set: Vec<Bytes32>,
}

#[derive(Clone, Default, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UploadPart {
    /// The root of the Merkle tree is created over the bytecode.
    pub root: Bytes32,
    /// The part of the bytecode.
    pub part_bytecode: Vec<u8>,
    /// The index of the part.
    pub part_index: u16,
    /// The total number of parts on which bytecode was divided.
    pub parts_number: u16,
    /// The proof set helps to verify the connection of the part to the `root`.
    pub proof_set: Vec<Bytes32>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SplitError {
    /// The size of the part is too small to fit all parts into `u16::MAX`.
    PartSizeTooSmall,
}

impl UploadPart {
    /// Splits the bytecode into verifiable parts and returns a vector of [`UploadPart`]s.
    pub fn split_bytecode(
        bytecode: &[u8],
        part_size: usize,
    ) -> Result<Vec<UploadPart>, SplitError> {
        let parts = bytecode
            .chunks(part_size)
            .map(|part| part.to_vec())
            .collect::<Vec<_>>();

        if parts.len() > u16::MAX as usize {
            return Err(SplitError::PartSizeTooSmall);
        }
        let parts_number =
            u16::try_from(parts.len()).expect("We've just checked it; qed");

        let mut merkle_tree = fuel_merkle::binary::in_memory::MerkleTree::new();
        parts.iter().for_each(|part| merkle_tree.push(part));

        let merkle_root = merkle_tree.root();

        let parts = parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                let (root, proof_set) = merkle_tree
                    .prove(index as u64)
                    .expect("We've just created a merkle tree, so it is valid; qed");
                debug_assert_eq!(root, merkle_root);

                UploadPart {
                    root: merkle_root.into(),
                    part_bytecode: part,
                    part_index: u16::try_from(index)
                        .expect("The total number of parts is less than u16::MAX; qed"),
                    parts_number,
                    proof_set: proof_set.into_iter().map(Into::into).collect(),
                }
            })
            .collect();

        Ok(parts)
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
        let verify_proof_gas =
            gas_cost.state_root().resolve(self.body.parts_number as u64);

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
        if self.body.parts_number > consensus_params.tx_params().max_bytecode_parts() {
            return Err(ValidityError::TransactionUploadTooManyBytecodeParts);
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

        // Verify that part of the bytecode is connected to the `root` of the bytecode.
        let result = fuel_merkle::binary::verify(
            self.body.root.deref(),
            witness,
            &proof_set,
            self.body.part_index as u64,
            self.body.parts_number as u64,
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

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.metadata = None;
        self.metadata = Some(ChargeableMetadata {
            common: CommonMetadata::compute(self, chain_id),
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
        PartIndex,
        PartsNumber,
        ProofSet,
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
            Self::bytecode_root_offset_static() + Bytes32::LEN
        }
    }

    impl PartIndex for Upload {
        #[inline(always)]
        fn part_index(&self) -> &u16 {
            &self.body.part_index
        }

        #[inline(always)]
        fn part_index_mut(&mut self) -> &mut u16 {
            &mut self.body.part_index
        }

        #[inline(always)]
        fn part_index_offset_static() -> usize {
            Self::bytecode_witness_index_offset_static() + WORD_SIZE
        }
    }

    impl PartsNumber for Upload {
        #[inline(always)]
        fn parts_number(&self) -> &u16 {
            &self.body.parts_number
        }

        #[inline(always)]
        fn parts_number_mut(&mut self) -> &mut u16 {
            &mut self.body.parts_number
        }

        #[inline(always)]
        fn parts_number_offset_static() -> usize {
            Self::part_index_offset_static() + WORD_SIZE
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
            Self::parts_number_offset_static() + WORD_SIZE
                + WORD_SIZE // Proof set size
                + WORD_SIZE // Policies size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE // Witnesses size
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
            Self::proof_set_offset_static() + self.body.proof_set.len() * Bytes32::LEN
        }
    }
}
