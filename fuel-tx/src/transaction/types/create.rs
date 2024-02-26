use crate::{
    policies::Policies,
    transaction::{
        field::{
            BytecodeLength,
            BytecodeWitnessIndex,
            Inputs,
            Outputs,
            Policies as PoliciesField,
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
    ConsensusParameters,
    Contract,
    GasCosts,
    Input,
    Output,
    StorageSlot,
    TransactionRepr,
    ValidityError,
    Witness,
};
use derivative::Derivative;
use fuel_types::{
    bytes,
    bytes::WORD_SIZE,
    canonical,
    BlockHeight,
    Bytes32,
    Bytes4,
    ChainId,
    ContractId,
    Salt,
    Word,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use hashbrown::HashMap;

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

impl CreateMetadata {
    /// Computes the `Metadata` for the `tx` transaction.
    pub fn compute(tx: &Create, chain_id: &ChainId) -> Result<Self, ValidityError> {
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
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Create)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[derivative(Eq, PartialEq, Hash)]
pub struct Create {
    pub(crate) bytecode_length: Word,
    pub(crate) bytecode_witness_index: u8,
    pub(crate) policies: Policies,
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

impl crate::UniqueIdentifier for Create {
    fn id(&self, chain_id: &ChainId) -> crate::TxId {
        if let Some(id) = self.cached_id() {
            return id;
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
    #[inline(always)]
    fn metered_bytes_size(&self) -> usize {
        canonical::Serialize::size(self)
    }

    fn gas_used_by_metadata(&self, gas_costs: &GasCosts) -> Word {
        let Create {
            bytecode_witness_index,
            witnesses,
            storage_slots,
            ..
        } = self;

        let contract_len = witnesses
            .get(*bytecode_witness_index as usize)
            .map(|c| c.as_ref().len())
            .unwrap_or(0);

        let contract_root_gas = gas_costs.contract_root.resolve(contract_len as Word);
        let state_root_length = storage_slots.len() as Word;
        let state_root_gas = gas_costs.state_root.resolve(state_root_length);

        // See https://github.com/FuelLabs/fuel-specs/blob/master/src/identifiers/contract-id.md
        let contract_id_input_length = core::mem::size_of::<Bytes4>()
            + core::mem::size_of::<Salt>()
            + core::mem::size_of::<Bytes32>()
            + core::mem::size_of::<Bytes32>();
        let contract_id_gas = gas_costs.s256.resolve(contract_id_input_length as Word);
        let bytes = canonical::Serialize::size(self);
        // Gas required to calculate the `tx_id`.
        let tx_id_gas = gas_costs.s256.resolve(bytes as u64);

        contract_root_gas
            .saturating_add(state_root_gas)
            .saturating_add(contract_id_gas)
            .saturating_add(tx_id_gas)
    }
}

impl FormatValidityChecks for Create {
    fn check_signatures(&self, chain_id: &ChainId) -> Result<(), ValidityError> {
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
                input.check_signature(index, &id, &self.witnesses, &mut recovery_cache)
            })?;

        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        let ConsensusParameters {
            contract_params,
            chain_id,
            base_asset_id,
            ..
        } = consensus_params;

        check_common_part(self, block_height, consensus_params)?;

        let bytecode_witness_len = self
            .witnesses
            .get(self.bytecode_witness_index as usize)
            .map(|w| w.as_ref().len() as Word)
            .ok_or(ValidityError::TransactionCreateBytecodeWitnessIndex)?;

        if bytecode_witness_len > contract_params.contract_max_size
            || bytecode_witness_len / 4 != self.bytecode_length
        {
            return Err(ValidityError::TransactionCreateBytecodeLen);
        }

        // Restrict to subset of u16::MAX, allowing this to be increased in the future
        // in a non-breaking way.
        if self.storage_slots.len() as u64 > contract_params.max_storage_slots {
            return Err(ValidityError::TransactionCreateStorageSlotMax);
        }

        // Verify storage slots are sorted
        if !self
            .storage_slots
            .as_slice()
            .windows(2)
            .all(|s| s[0] < s[1])
        {
            return Err(ValidityError::TransactionCreateStorageSlotOrder);
        }

        self.inputs
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| match input {
                Input::Contract(_) => {
                    Err(ValidityError::TransactionCreateInputContract { index })
                }
                Input::MessageDataSigned(_) | Input::MessageDataPredicate(_) => {
                    Err(ValidityError::TransactionCreateMessageData { index })
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
                let metadata = CreateMetadata::compute(self, chain_id)?;
                (metadata.state_root, metadata.contract_id)
            };

        let mut contract_created = false;
        self.outputs
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| match output {
                Output::Contract(_) => {
                    Err(ValidityError::TransactionCreateOutputContract { index })
                }

                Output::Variable { .. } => {
                    Err(ValidityError::TransactionCreateOutputVariable { index })
                }

                Output::Change { asset_id, .. } if asset_id != base_asset_id => {
                    Err(ValidityError::TransactionCreateOutputChangeNotBaseAsset { index })
                }

                Output::ContractCreated {
                    contract_id,
                    state_root,
                } if contract_id != &contract_id_calculated
                    || state_root != &state_root_calculated =>
                    {
                        Err(
                            ValidityError::TransactionCreateOutputContractCreatedDoesntMatch {
                                index,
                            },
                        )
                    }

                // TODO: Output::ContractCreated { contract_id, state_root } if
                // contract_id == &id && state_root == &storage_root
                //  maybe move from `fuel-vm` to here
                Output::ContractCreated { .. } if contract_created => {
                    Err(ValidityError::TransactionCreateOutputContractCreatedMultiple {
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

impl crate::Cacheable for Create {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.metadata = None;
        self.metadata = Some(CreateMetadata::compute(self, chain_id)?);
        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::StorageSlotRef;
    use fuel_types::canonical::Serialize;

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
            WORD_SIZE // `Transaction` enum discriminant
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

    impl PoliciesField for Create {
        fn policies(&self) -> &Policies {
            &self.policies
        }

        fn policies_mut(&mut self) -> &mut Policies {
            &mut self.policies
        }

        fn policies_offset(&self) -> usize {
            Self::salt_offset_static() + Salt::LEN
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
                + WORD_SIZE // Policies size
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
        fn storage_slots_offset(&self) -> usize {
            self.policies_offset() + self.policies.size_dynamic()
        }

        fn storage_slots_offset_at(&self, idx: usize) -> Option<usize> {
            if idx < self.storage_slots.len() {
                Some(self.storage_slots_offset() + idx * StorageSlot::SLOT_SIZE)
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
            self.storage_slots_offset()
                + self.storage_slots.len() * StorageSlot::SLOT_SIZE
        }

        #[inline(always)]
        fn inputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CreateMetadata {
                            inputs_offset_at: inputs_offset,
                            ..
                        }) = &self.metadata
            {
                return inputs_offset.get(idx).cloned();
            }

            if idx < self.inputs.len() {
                Some(
                    self.inputs_offset()
                        + self
                        .inputs()
                        .iter()
                        .take(idx)
                        .map(|i| i.size())
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
                return inputs_predicate_offset.get(idx).cloned().unwrap_or(None);
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
                return *outputs_offset;
            }

            self.inputs_offset() + self.inputs().iter().map(|i| i.size()).sum::<usize>()
        }

        #[inline(always)]
        fn outputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CreateMetadata {
                            outputs_offset_at: outputs_offset,
                            ..
                        }) = &self.metadata
            {
                return outputs_offset.get(idx).cloned();
            }

            if idx < self.outputs.len() {
                Some(
                    self.outputs_offset()
                        + self
                        .outputs()
                        .iter()
                        .take(idx)
                        .map(|i| i.size())
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
                return *witnesses_offset;
            }

            self.outputs_offset() + self.outputs().iter().map(|i| i.size()).sum::<usize>()
        }

        #[inline(always)]
        fn witnesses_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CreateMetadata {
                            witnesses_offset_at: witnesses_offset,
                            ..
                        }) = &self.metadata
            {
                return witnesses_offset.get(idx).cloned();
            }

            if idx < self.witnesses.len() {
                Some(
                    self.witnesses_offset()
                        + self
                        .witnesses()
                        .iter()
                        .take(idx)
                        .map(|i| i.size())
                        .sum::<usize>(),
                )
            } else {
                None
            }
        }
    }
}

impl TryFrom<&Create> for Contract {
    type Error = ValidityError;

    fn try_from(tx: &Create) -> Result<Self, Self::Error> {
        let Create {
            bytecode_witness_index,
            witnesses,
            ..
        } = tx;

        witnesses
            .get(*bytecode_witness_index as usize)
            .map(|c| c.as_ref().into())
            .ok_or(ValidityError::TransactionCreateBytecodeWitnessIndex)
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
        let arb_max_fee = 1000;

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
            .max_fee_limit(arb_max_fee)
            .add_random_fee_input()
            .finalize();
        tx.storage_slots.reverse();

        let err = tx
            .check(0.into(), &ConsensusParameters::standard())
            .expect_err("Expected erroneous transaction");

        assert_eq!(ValidityError::TransactionCreateStorageSlotOrder, err);
    }

    #[test]
    fn storage_slots_no_duplicates() {
        let arb_max_fee = 1000;

        let storage_slots = vec![
            StorageSlot::new(Bytes32::zeroed(), Bytes32::zeroed()),
            StorageSlot::new(Bytes32::zeroed(), Bytes32::zeroed()),
        ];

        let err = crate::TransactionBuilder::create(
            vec![].into(),
            Salt::zeroed(),
            storage_slots,
        )
            .max_fee_limit(arb_max_fee)
            .add_random_fee_input()
            .finalize()
            .check(0.into(), &ConsensusParameters::standard())
            .expect_err("Expected erroneous transaction");

        assert_eq!(ValidityError::TransactionCreateStorageSlotOrder, err);
    }
}
