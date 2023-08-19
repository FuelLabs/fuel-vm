use crate::{
    transaction::{
        field::{
            BytecodeLength,
            BytecodeWitnessIndex,
            GasLimit,
            GasPrice,
            Inputs,
            Maturity,
            Outputs,
            Salt as SaltField,
            StorageSlots,
            Witnesses,
        },
        validity::{
            check_common_part,
            FormatValidityChecks,
        },
    },
    Chargeable,
    CheckError,
    ConsensusParameters,
    Contract,
    Input,
    Output,
    StorageSlot,
    TransactionRepr,
    Witness,
};
use derivative::Derivative;
use fuel_types::{
    bytes,
    bytes::{
        SizedBytes,
        WORD_SIZE,
    },
    AssetId,
    BlockHeight,
    Bytes32,
    ContractId,
    Salt,
    Word,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use fuel_types::ChainId;
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(all(test, feature = "std"))]
mod ser_de_tests;

#[derive(Default, Debug, Clone, Derivative)]
#[derivative(Eq, PartialEq, Hash)]
pub struct CreateMetadata {
    pub contract_id: ContractId,
    pub contract_root: Bytes32,
    pub state_root: Bytes32,
    pub id: Bytes32,
    pub inputs_offset: usize,
    pub inputs_offset_at: Vec<usize>,
    pub inputs_predicate_offset_at: Vec<Option<(usize, usize)>>,
    pub outputs_offset: usize,
    pub outputs_offset_at: Vec<usize>,
    pub witnesses_offset: usize,
    pub witnesses_offset_at: Vec<usize>,
}

#[cfg(feature = "std")]
impl CreateMetadata {
    /// Computes the `Metadata` for the `tx` transaction.
    pub fn compute(tx: &Create, chain_id: &ChainId) -> Result<Self, CheckError> {
        use crate::transaction::metadata::CommonMetadata;

        let CommonMetadata {
            id,
            inputs_offset,
            inputs_offset_at,
            inputs_predicate_offset_at,
            outputs_offset,
            outputs_offset_at,
            witnesses_offset,
            witnesses_offset_at,
        } = CommonMetadata::compute(tx, chain_id);

        let salt = tx.salt();
        let storage_slots = tx.storage_slots();
        let contract = Contract::try_from(tx)?;
        let contract_root = contract.root();
        let state_root = Contract::initial_state_root(storage_slots.iter());
        let contract_id = contract.id(salt, &contract_root, &state_root);

        Ok(Self {
            contract_id,
            contract_root,
            state_root,
            id,
            inputs_offset,
            inputs_offset_at,
            inputs_predicate_offset_at,
            outputs_offset,
            outputs_offset_at,
            witnesses_offset,
            witnesses_offset_at,
        })
    }
}

#[derive(Default, Debug, Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    any(feature = "alloc", feature = "std"),
    derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)
)]
#[canonical(prefix = TransactionRepr::Create)]
#[derivative(Eq, PartialEq, Hash)]
pub struct Create {
    pub(crate) gas_price: Word,
    pub(crate) gas_limit: Word,
    pub(crate) maturity: BlockHeight,
    pub(crate) bytecode_length: Word,
    pub(crate) bytecode_witness_index: u8,
    pub(crate) storage_slots: Vec<StorageSlot>,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
    pub(crate) witnesses: Vec<Witness>,
    pub(crate) salt: Salt,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
    #[canonical(skip)]
    pub(crate) metadata: Option<CreateMetadata>,
}

impl Create {
    pub fn metadata(&self) -> &Option<CreateMetadata> {
        &self.metadata
    }
}

#[cfg(feature = "std")]
impl crate::UniqueIdentifier for Create {
    fn id(&self, chain_id: &ChainId) -> crate::TxId {
        if let Some(id) = self.cached_id() {
            return id
        }

        let mut clone = self.clone();

        // Empties fields that should be zero during the signing.
        clone.inputs_mut().iter_mut().for_each(Input::prepare_sign);
        clone
            .outputs_mut()
            .iter_mut()
            .for_each(Output::prepare_sign);
        clone.witnesses_mut().clear();

        crate::transaction::compute_transaction_id(chain_id, &mut clone)
    }

    fn cached_id(&self) -> Option<crate::TxId> {
        self.metadata.as_ref().map(|m| m.id)
    }
}

impl Chargeable for Create {
    fn price(&self) -> Word {
        *GasPrice::gas_price(self)
    }

    fn limit(&self) -> Word {
        *GasLimit::gas_limit(self)
    }

    #[inline(always)]
    fn metered_bytes_size(&self) -> usize {
        // Just use the default serialized size for now until
        // the compressed representation for accounting purposes
        // is defined. Witness data should still be excluded.
        self.witnesses_offset()
    }

    fn gas_used_by_predicates(&self) -> Word {
        let mut cumulative_predicate_gas: Word = 0;
        for input in self.inputs() {
            if let Some(predicate_gas_used) = input.predicate_gas_used() {
                cumulative_predicate_gas =
                    cumulative_predicate_gas.saturating_add(predicate_gas_used);
            }
        }
        cumulative_predicate_gas
    }
}

#[cfg(feature = "std")]
impl FormatValidityChecks for Create {
    fn check_signatures(&self, chain_id: &ChainId) -> Result<(), CheckError> {
        use crate::UniqueIdentifier;

        let id = self.id(chain_id);

        // There will be at most len(witnesses) - 1 signatures to cache, as one of the
        // witnesses will be bytecode
        let mut recovery_cache = Some(HashMap::with_capacity(core::cmp::max(
            self.witnesses().len() - 1,
            1,
        )));

        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                input.check_signature(
                    index,
                    &id,
                    &self.witnesses,
                    chain_id,
                    &mut recovery_cache,
                )
            })?;

        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        check_common_part(
            self,
            block_height,
            consensus_params.tx_params(),
            consensus_params.predicate_params(),
        )?;

        let bytecode_witness_len = self
            .witnesses
            .get(self.bytecode_witness_index as usize)
            .map(|w| w.as_ref().len() as Word)
            .ok_or(CheckError::TransactionCreateBytecodeWitnessIndex)?;

        let contract_params = consensus_params.contract_params();

        if bytecode_witness_len > contract_params.contract_max_size
            || bytecode_witness_len / 4 != self.bytecode_length
        {
            return Err(CheckError::TransactionCreateBytecodeLen)
        }

        // Restrict to subset of u16::MAX, allowing this to be increased in the future
        // in a non-breaking way.
        if self.storage_slots.len() > contract_params.max_storage_slots as usize {
            return Err(CheckError::TransactionCreateStorageSlotMax)
        }

        // Verify storage slots are sorted
        if !self
            .storage_slots
            .as_slice()
            .windows(2)
            .all(|s| s[0] < s[1])
        {
            return Err(CheckError::TransactionCreateStorageSlotOrder)
        }

        self.inputs
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| match input {
                Input::Contract(_) => {
                    Err(CheckError::TransactionCreateInputContract { index })
                }
                Input::MessageDataSigned(_) | Input::MessageDataPredicate(_) => {
                    Err(CheckError::TransactionCreateMessageData { index })
                }
                _ => Ok(()),
            })?;

        debug_assert!(
            self.metadata.is_some(),
            "`check_without_signatures` is called without cached metadata"
        );
        let (state_root_calculated, contract_id_calculated) =
            if let Some(metadata) = &self.metadata {
                (metadata.state_root, metadata.contract_id)
            } else {
                #[cfg(feature = "std")]
                {
                    let metadata =
                        CreateMetadata::compute(self, &consensus_params.chain_id())?;
                    (metadata.state_root, metadata.contract_id)
                }

                #[cfg(not(feature = "std"))]
                {
                    let salt = self.salt();
                    let storage_slots = self.storage_slots();
                    let contract = Contract::try_from(self)?;
                    let contract_root = contract.root();
                    let state_root = Contract::initial_state_root(storage_slots.iter());
                    let contract_id = contract.id(salt, &contract_root, &state_root);
                    (state_root, contract_id)
                }
            };

        let mut contract_created = false;
        self.outputs
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| match output {
                Output::Contract { .. } => {
                    Err(CheckError::TransactionCreateOutputContract { index })
                }

                Output::Variable { .. } => {
                    Err(CheckError::TransactionCreateOutputVariable { index })
                }

                Output::Change { asset_id, .. } if asset_id != &AssetId::BASE => {
                    Err(CheckError::TransactionCreateOutputChangeNotBaseAsset { index })
                }

                Output::ContractCreated {
                    contract_id,
                    state_root,
                } if contract_id != &contract_id_calculated
                    || state_root != &state_root_calculated =>
                {
                    Err(
                        CheckError::TransactionCreateOutputContractCreatedDoesntMatch {
                            index,
                        },
                    )
                }

                // TODO: Output::ContractCreated { contract_id, state_root } if
                // contract_id == &id && state_root == &storage_root
                //  maybe move from `fuel-vm` to here
                Output::ContractCreated { .. } if contract_created => {
                    Err(CheckError::TransactionCreateOutputContractCreatedMultiple {
                        index,
                    })
                }

                Output::ContractCreated { .. } => {
                    contract_created = true;

                    Ok(())
                }

                _ => Ok(()),
            })?;

        Ok(())
    }
}

#[cfg(feature = "std")]
impl crate::Cacheable for Create {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), CheckError> {
        self.metadata = None;
        self.metadata = Some(CreateMetadata::compute(self, chain_id)?);
        Ok(())
    }
}

impl SizedBytes for Create {
    fn serialized_size(&self) -> usize {
        self.witnesses_offset()
            + self
                .witnesses()
                .iter()
                .map(|w| w.serialized_size())
                .sum::<usize>()
    }
}

mod field {
    use super::*;
    use crate::field::StorageSlotRef;

    impl GasPrice for Create {
        #[inline(always)]
        fn gas_price(&self) -> &Word {
            &self.gas_price
        }

        #[inline(always)]
        fn gas_price_mut(&mut self) -> &mut Word {
            &mut self.gas_price
        }

        #[inline(always)]
        fn gas_price_offset_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl GasLimit for Create {
        #[inline(always)]
        fn gas_limit(&self) -> &Word {
            &self.gas_limit
        }

        #[inline(always)]
        fn gas_limit_mut(&mut self) -> &mut Word {
            &mut self.gas_limit
        }

        #[inline(always)]
        fn gas_limit_offset_static() -> usize {
            Self::gas_price_offset_static() + WORD_SIZE
        }
    }

    impl Maturity for Create {
        #[inline(always)]
        fn maturity(&self) -> &BlockHeight {
            &self.maturity
        }

        #[inline(always)]
        fn maturity_mut(&mut self) -> &mut BlockHeight {
            &mut self.maturity
        }

        #[inline(always)]
        fn maturity_offset_static() -> usize {
            Self::gas_limit_offset_static() + WORD_SIZE
        }
    }

    impl BytecodeLength for Create {
        #[inline(always)]
        fn bytecode_length(&self) -> &Word {
            &self.bytecode_length
        }

        #[inline(always)]
        fn bytecode_length_mut(&mut self) -> &mut Word {
            &mut self.bytecode_length
        }

        #[inline(always)]
        fn bytecode_length_offset_static() -> usize {
            Self::maturity_offset_static() + WORD_SIZE
        }
    }

    impl BytecodeWitnessIndex for Create {
        #[inline(always)]
        fn bytecode_witness_index(&self) -> &u8 {
            &self.bytecode_witness_index
        }

        #[inline(always)]
        fn bytecode_witness_index_mut(&mut self) -> &mut u8 {
            &mut self.bytecode_witness_index
        }

        #[inline(always)]
        fn bytecode_witness_index_offset_static() -> usize {
            Self::bytecode_length_offset_static() + WORD_SIZE
        }
    }

    impl SaltField for Create {
        #[inline(always)]
        fn salt(&self) -> &Salt {
            &self.salt
        }

        #[inline(always)]
        fn salt_mut(&mut self) -> &mut Salt {
            &mut self.salt
        }

        #[inline(always)]
        fn salt_offset_static() -> usize {
            Self::bytecode_witness_index_offset_static() + WORD_SIZE
                + WORD_SIZE // Storage slots size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE // Witnesses size
        }
    }

    impl StorageSlots for Create {
        #[inline(always)]
        fn storage_slots(&self) -> &Vec<StorageSlot> {
            &self.storage_slots
        }

        #[inline(always)]
        fn storage_slots_mut(&mut self) -> StorageSlotRef {
            StorageSlotRef {
                storage_slots: &mut self.storage_slots,
            }
        }

        #[inline(always)]
        fn storage_slots_offset_static() -> usize {
            Self::salt_offset_static() + Salt::LEN
        }

        fn storage_slots_offset_at(&self, idx: usize) -> Option<usize> {
            if idx < self.storage_slots.len() {
                Some(Self::storage_slots_offset_static() + idx * StorageSlot::SLOT_SIZE)
            } else {
                None
            }
        }
    }

    impl Inputs for Create {
        #[inline(always)]
        fn inputs(&self) -> &Vec<Input> {
            &self.inputs
        }

        #[inline(always)]
        fn inputs_mut(&mut self) -> &mut Vec<Input> {
            &mut self.inputs
        }

        #[inline(always)]
        fn inputs_offset(&self) -> usize {
            Self::storage_slots_offset_static()
                + self.storage_slots.len() * StorageSlot::SLOT_SIZE
        }

        #[inline(always)]
        fn inputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CreateMetadata {
                inputs_offset_at: inputs_offset,
                ..
            }) = &self.metadata
            {
                return inputs_offset.get(idx).cloned()
            }

            if idx < self.inputs.len() {
                Some(
                    self.inputs_offset()
                        + self
                            .inputs()
                            .iter()
                            .take(idx)
                            .map(|i| i.serialized_size())
                            .sum::<usize>(),
                )
            } else {
                None
            }
        }

        #[inline(always)]
        fn inputs_predicate_offset_at(&self, idx: usize) -> Option<(usize, usize)> {
            if let Some(CreateMetadata {
                inputs_predicate_offset_at: inputs_predicate_offset,
                ..
            }) = &self.metadata
            {
                return inputs_predicate_offset.get(idx).cloned().unwrap_or(None)
            }

            self.inputs().get(idx).and_then(|input| {
                input
                    .predicate_offset()
                    .and_then(|predicate| {
                        self.inputs_offset_at(idx).map(|inputs| inputs + predicate)
                    })
                    .zip(input.predicate_len().map(bytes::padded_len_usize))
            })
        }
    }

    impl Outputs for Create {
        #[inline(always)]
        fn outputs(&self) -> &Vec<Output> {
            &self.outputs
        }

        #[inline(always)]
        fn outputs_mut(&mut self) -> &mut Vec<Output> {
            &mut self.outputs
        }

        #[inline(always)]
        fn outputs_offset(&self) -> usize {
            if let Some(CreateMetadata { outputs_offset, .. }) = &self.metadata {
                return *outputs_offset
            }

            self.inputs_offset()
                + self
                    .inputs()
                    .iter()
                    .map(|i| i.serialized_size())
                    .sum::<usize>()
        }

        #[inline(always)]
        fn outputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CreateMetadata {
                outputs_offset_at: outputs_offset,
                ..
            }) = &self.metadata
            {
                return outputs_offset.get(idx).cloned()
            }

            if idx < self.outputs.len() {
                Some(
                    self.outputs_offset()
                        + self
                            .outputs()
                            .iter()
                            .take(idx)
                            .map(|i| i.serialized_size())
                            .sum::<usize>(),
                )
            } else {
                None
            }
        }
    }

    impl Witnesses for Create {
        #[inline(always)]
        fn witnesses(&self) -> &Vec<Witness> {
            &self.witnesses
        }

        #[inline(always)]
        fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
            &mut self.witnesses
        }

        #[inline(always)]
        fn witnesses_offset(&self) -> usize {
            if let Some(CreateMetadata {
                witnesses_offset, ..
            }) = &self.metadata
            {
                return *witnesses_offset
            }

            self.outputs_offset()
                + self
                    .outputs()
                    .iter()
                    .map(|i| i.serialized_size())
                    .sum::<usize>()
        }

        #[inline(always)]
        fn witnesses_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CreateMetadata {
                witnesses_offset_at: witnesses_offset,
                ..
            }) = &self.metadata
            {
                return witnesses_offset.get(idx).cloned()
            }

            if idx < self.witnesses.len() {
                Some(
                    self.witnesses_offset()
                        + self
                            .witnesses()
                            .iter()
                            .take(idx)
                            .map(|i| i.serialized_size())
                            .sum::<usize>(),
                )
            } else {
                None
            }
        }
    }
}

impl TryFrom<&Create> for Contract {
    type Error = CheckError;

    fn try_from(tx: &Create) -> Result<Self, Self::Error> {
        let Create {
            bytecode_witness_index,
            witnesses,
            ..
        } = tx;

        witnesses
            .get(*bytecode_witness_index as usize)
            .map(|c| c.as_ref().into())
            .ok_or(CheckError::TransactionCreateBytecodeWitnessIndex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Finalizable;
    use fuel_types::Bytes32;

    #[test]
    fn storage_slots_sorting() {
        // Test that storage slots must be sorted correctly
        let mut slot_data = [0u8; 64];

        let storage_slots = (0..10u64)
            .map(|i| {
                slot_data[..8].copy_from_slice(&i.to_be_bytes());
                StorageSlot::from(&slot_data.into())
            })
            .collect::<Vec<StorageSlot>>();

        let mut tx = crate::TransactionBuilder::create(
            vec![].into(),
            Salt::zeroed(),
            storage_slots,
        )
        .add_random_fee_input()
        .finalize();
        tx.storage_slots.reverse();

        let err = tx
            .check(0.into(), &ConsensusParameters::standard())
            .expect_err("Expected erroneous transaction");

        assert_eq!(CheckError::TransactionCreateStorageSlotOrder, err);
    }

    #[test]
    fn storage_slots_no_duplicates() {
        let storage_slots = vec![
            StorageSlot::new(Bytes32::zeroed(), Bytes32::zeroed()),
            StorageSlot::new(Bytes32::zeroed(), Bytes32::zeroed()),
        ];

        let err = crate::TransactionBuilder::create(
            vec![].into(),
            Salt::zeroed(),
            storage_slots,
        )
        .add_random_fee_input()
        .finalize()
        .check(0.into(), &ConsensusParameters::standard())
        .expect_err("Expected erroneous transaction");

        assert_eq!(CheckError::TransactionCreateStorageSlotOrder, err);
    }
}
