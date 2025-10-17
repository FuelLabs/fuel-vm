use crate::{
    Chargeable,
    ConsensusParameters,
    Contract,
    GasCosts,
    Input,
    Output,
    PrepareSign,
    StorageSlot,
    TransactionRepr,
    ValidityError,
    transaction::{
        field::{
            BytecodeWitnessIndex,
            Salt as SaltField,
            StorageSlots,
        },
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
    Bytes4,
    Bytes32,
    ChainId,
    ContractId,
    Salt,
    Word,
    bytes::WORD_SIZE,
    canonical,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(all(test, feature = "std"))]
mod ser_de_tests;

pub type Create = ChargeableTransaction<CreateBody, CreateMetadata>;

impl Create {
    pub fn bytecode(&self) -> Result<&[u8], ValidityError> {
        let Create {
            body:
                CreateBody {
                    bytecode_witness_index,
                    ..
                },
            witnesses,
            ..
        } = self;

        witnesses
            .get(*bytecode_witness_index as usize)
            .map(|c| c.as_ref())
            .ok_or(ValidityError::TransactionCreateBytecodeWitnessIndex)
    }
}

#[derive(Default, Debug, Clone, Educe)]
#[educe(Eq, PartialEq, Hash)]
pub struct CreateMetadata {
    pub contract_id: ContractId,
    pub contract_root: Bytes32,
    pub state_root: Bytes32,
}

impl CreateMetadata {
    /// Computes the `Metadata` for the `tx` transaction.
    pub fn compute(tx: &Create) -> Result<Self, ValidityError> {
        let salt = tx.salt();
        let storage_slots = tx.storage_slots();
        let bytecode = tx.bytecode()?;
        let contract_root = Contract::root_from_code(bytecode);
        let state_root = Contract::initial_state_root(storage_slots.iter());
        let contract_id = Contract::id(salt, &contract_root, &state_root);

        Ok(Self {
            contract_id,
            contract_root,
            state_root,
        })
    }
}

#[derive(Default, Debug, Clone, Educe, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Create)]
#[educe(Eq, PartialEq, Hash)]
pub struct CreateBody {
    pub(crate) bytecode_witness_index: u16,
    pub(crate) salt: Salt,
    pub(crate) storage_slots: Vec<StorageSlot>,
}

impl PrepareSign for CreateBody {
    fn prepare_sign(&mut self) {}
}

impl Chargeable for Create {
    #[inline(always)]
    fn metered_bytes_size(&self) -> usize {
        canonical::Serialize::size(self)
    }

    fn gas_used_by_metadata(&self, gas_costs: &GasCosts) -> Word {
        let Create {
            body:
                CreateBody {
                    bytecode_witness_index,
                    storage_slots,
                    ..
                },
            witnesses,
            ..
        } = self;

        let contract_len = witnesses
            .get(*bytecode_witness_index as usize)
            .map(|c| c.as_ref().len())
            .unwrap_or(0);

        let contract_root_gas = gas_costs.contract_root().resolve(contract_len as Word);
        let state_root_length = storage_slots.len() as Word;
        let state_root_gas = gas_costs.state_root().resolve(state_root_length);

        // See https://github.com/FuelLabs/fuel-specs/blob/master/src/identifiers/contract-id.md
        let contract_id_input_length =
            Bytes4::LEN + Salt::LEN + Bytes32::LEN + Bytes32::LEN;
        let contract_id_gas = gas_costs.s256().resolve(contract_id_input_length as Word);
        let bytes = canonical::Serialize::size(self);
        // Gas required to calculate the `tx_id`.
        let tx_id_gas = gas_costs.s256().resolve(bytes as u64);

        contract_root_gas
            .saturating_add(state_root_gas)
            .saturating_add(contract_id_gas)
            .saturating_add(tx_id_gas)
    }
}

impl UniqueFormatValidityChecks for Create {
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        let contract_params = consensus_params.contract_params();
        let base_asset_id = consensus_params.base_asset_id();

        let bytecode_witness_len = self
            .witnesses
            .get(self.body.bytecode_witness_index as usize)
            .map(|w| w.as_ref().len() as Word)
            .ok_or(ValidityError::TransactionCreateBytecodeWitnessIndex)?;

        if bytecode_witness_len > contract_params.contract_max_size() {
            return Err(ValidityError::TransactionCreateBytecodeLen);
        }

        // Restrict to subset of u16::MAX, allowing this to be increased in the future
        // in a non-breaking way.
        if self.body.storage_slots.len() as u64 > contract_params.max_storage_slots() {
            return Err(ValidityError::TransactionCreateStorageSlotMax);
        }

        // Verify storage slots are sorted
        if !self
            .body
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
            .try_for_each(|(index, input)| {
                if let Some(asset_id) = input.asset_id(consensus_params.base_asset_id())
                    && asset_id != consensus_params.base_asset_id()
                {
                    return Err(ValidityError::TransactionInputContainsNonBaseAssetId {
                        index,
                    });
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

        debug_assert!(
            self.metadata.is_some(),
            "`check_without_signatures` is called without cached metadata"
        );
        let (state_root_calculated, contract_id_calculated) =
            if let Some(metadata) = &self.metadata {
                (metadata.body.state_root, metadata.body.contract_id)
            } else {
                let metadata = CreateMetadata::compute(self)?;
                (metadata.state_root, metadata.contract_id)
            };

        let mut contract_created = false;
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

                Output::Change { asset_id, .. } if asset_id != base_asset_id => {
                    Err(ValidityError::TransactionChangeChangeUsesNotBaseAsset { index })
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

        if !contract_created {
            return Err(ValidityError::TransactionOutputDoesntContainContractCreated);
        }

        Ok(())
    }
}

impl crate::Cacheable for Create {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.metadata = None;
        self.metadata = Some(ChargeableMetadata {
            common: CommonMetadata::compute(self, chain_id)?,
            body: CreateMetadata::compute(self)?,
        });
        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::{
        ChargeableBody,
        StorageSlotRef,
    };

    impl BytecodeWitnessIndex for Create {
        #[inline(always)]
        fn bytecode_witness_index(&self) -> &u16 {
            &self.body.bytecode_witness_index
        }

        #[inline(always)]
        fn bytecode_witness_index_mut(&mut self) -> &mut u16 {
            &mut self.body.bytecode_witness_index
        }

        #[inline(always)]
        fn bytecode_witness_index_offset_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl SaltField for Create {
        #[inline(always)]
        fn salt(&self) -> &Salt {
            &self.body.salt
        }

        #[inline(always)]
        fn salt_mut(&mut self) -> &mut Salt {
            &mut self.body.salt
        }

        #[inline(always)]
        fn salt_offset_static() -> usize {
            Self::bytecode_witness_index_offset_static().saturating_add(WORD_SIZE)
        }
    }

    impl StorageSlots for Create {
        #[inline(always)]
        fn storage_slots(&self) -> &Vec<StorageSlot> {
            &self.body.storage_slots
        }

        #[inline(always)]
        fn storage_slots_mut(&mut self) -> StorageSlotRef<'_> {
            StorageSlotRef {
                storage_slots: &mut self.body.storage_slots,
            }
        }

        #[inline(always)]
        fn storage_slots_offset_static() -> usize {
            Self::salt_offset_static().saturating_add(
                Salt::LEN
                + WORD_SIZE // Storage slots size
                + WORD_SIZE // Policies size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE, // Witnesses size
            )
        }

        fn storage_slots_offset_at(&self, idx: usize) -> Option<usize> {
            if idx < self.body.storage_slots.len() {
                Some(
                    Self::storage_slots_offset_static()
                        .checked_add(idx.checked_mul(StorageSlot::SLOT_SIZE)?)?,
                )
            } else {
                None
            }
        }
    }

    impl ChargeableBody<CreateBody> for Create {
        fn body(&self) -> &CreateBody {
            &self.body
        }

        fn body_mut(&mut self) -> &mut CreateBody {
            &mut self.body
        }

        fn body_offset_end(&self) -> usize {
            Self::storage_slots_offset_static().saturating_add(
                self.body
                    .storage_slots
                    .len()
                    .saturating_mul(StorageSlot::SLOT_SIZE),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        builder::Finalizable,
        transaction::validity::FormatValidityChecks,
    };
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
        .add_fee_input()
        .finalize();
        tx.body.storage_slots.reverse();

        let err = tx
            .check(0.into(), &ConsensusParameters::standard())
            .expect_err("Expected erroneous transaction");

        assert_eq!(ValidityError::TransactionCreateStorageSlotOrder, err);
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
        .add_fee_input()
        .finalize()
        .check(0.into(), &ConsensusParameters::standard())
        .expect_err("Expected erroneous transaction");

        assert_eq!(ValidityError::TransactionCreateStorageSlotOrder, err);
    }
}
