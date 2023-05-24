use crate::transaction::{
    compute_transaction_id,
    field::{
        BytecodeLength, BytecodeWitnessIndex, GasLimit, GasPrice, Inputs, Maturity, Outputs, Salt as SaltField,
        StorageSlots, Witnesses,
    },
    metadata::CommonMetadata,
    validity::{check_common_part, FormatValidityChecks},
};
use crate::{Chargeable, CheckError, ConsensusParameters, Contract, Input, Output, StorageSlot, TxId, Witness};
use derivative::Derivative;
use fuel_types::{bytes, AssetId, BlockHeight, ChainId, Salt, Word};
use fuel_types::{
    bytes::{SizedBytes, WORD_SIZE},
    mem_layout, MemLayout, MemLocType,
};

#[cfg(feature = "std")]
use std::io;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(all(test, feature = "std"))]
mod ser_de_tests;

#[derive(Default, Debug, Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub(crate) metadata: Option<CommonMetadata>,
}

mem_layout!(
    CreateLayout for Create
    repr: u8 = WORD_SIZE,
    gas_price: Word = WORD_SIZE,
    gas_limit: Word = WORD_SIZE,
    maturity: u32 = WORD_SIZE,
    bytecode_length: Word = WORD_SIZE,
    bytecode_witness_index: u8 = WORD_SIZE,
    storage_slots_len: Word = WORD_SIZE,
    inputs_len: Word = WORD_SIZE,
    outputs_len: Word = WORD_SIZE,
    witnesses_len: Word = WORD_SIZE,
    salt: Salt = {Salt::LEN}
);

#[cfg(feature = "std")]
impl crate::UniqueIdentifier for Create {
    fn id(&self, chain_id: &ChainId) -> TxId {
        if let Some(id) = self.cached_id() {
            return id;
        }

        let mut clone = self.clone();

        // Empties fields that should be zero during the signing.
        clone.inputs_mut().iter_mut().for_each(Input::prepare_sign);
        clone.outputs_mut().iter_mut().for_each(Output::prepare_sign);
        clone.witnesses_mut().clear();

        compute_transaction_id(chain_id, &mut clone)
    }

    fn cached_id(&self) -> Option<TxId> {
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
}

impl FormatValidityChecks for Create {
    #[cfg(feature = "std")]
    fn check_signatures(&self, chain_id: &ChainId) -> Result<(), CheckError> {
        use crate::UniqueIdentifier;

        let id = self.id(chain_id);

        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| input.check_signature(index, &id, &self.witnesses, chain_id))?;

        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: BlockHeight,
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        check_common_part(self, block_height, parameters)?;

        let bytecode_witness_len = self
            .witnesses
            .get(self.bytecode_witness_index as usize)
            .map(|w| w.as_ref().len() as Word)
            .ok_or(CheckError::TransactionCreateBytecodeWitnessIndex)?;

        if bytecode_witness_len > parameters.contract_max_size || bytecode_witness_len / 4 != self.bytecode_length {
            return Err(CheckError::TransactionCreateBytecodeLen);
        }

        // Restrict to subset of u16::MAX, allowing this to be increased in the future
        // in a non-breaking way.
        if self.storage_slots.len() > parameters.max_storage_slots as usize {
            return Err(CheckError::TransactionCreateStorageSlotMax);
        }

        // Verify storage slots are sorted
        if !self.storage_slots.as_slice().windows(2).all(|s| s[0] <= s[1]) {
            return Err(CheckError::TransactionCreateStorageSlotOrder);
        }

        // TODO The computed contract ADDRESS (see below) is not equal to the
        // contractADDRESS of the one OutputType.ContractCreated output

        self.inputs
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| match input {
                Input::Contract(_) => Err(CheckError::TransactionCreateInputContract { index }),
                Input::MessageDataSigned(_) | Input::MessageDataPredicate(_) => {
                    Err(CheckError::TransactionCreateMessageData { index })
                }
                _ => Ok(()),
            })?;

        let mut contract_created = false;
        self.outputs
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| match output {
                Output::Contract { .. } => Err(CheckError::TransactionCreateOutputContract { index }),

                Output::Variable { .. } => Err(CheckError::TransactionCreateOutputVariable { index }),

                Output::Change { asset_id, .. } if asset_id != &AssetId::BASE => {
                    Err(CheckError::TransactionCreateOutputChangeNotBaseAsset { index })
                }

                // TODO: Output::ContractCreated { contract_id, state_root } if contract_id == &id && state_root == &storage_root
                //  maybe move from `fuel-vm` to here
                Output::ContractCreated { .. } if contract_created => {
                    Err(CheckError::TransactionCreateOutputContractCreatedMultiple { index })
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

    fn precompute(&mut self, chain_id: &ChainId) {
        self.metadata = None;
        self.metadata = Some(CommonMetadata::compute(self, chain_id));
    }
}

impl SizedBytes for Create {
    fn serialized_size(&self) -> usize {
        self.witnesses_offset() + self.witnesses().iter().map(|w| w.serialized_size()).sum::<usize>()
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
            WORD_SIZE /* `Transaction` enum discriminant */
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
            Self::storage_slots_offset_static() + self.storage_slots.len() * StorageSlot::SLOT_SIZE
        }

        #[inline(always)]
        fn inputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CommonMetadata {
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
                            .map(|i| i.serialized_size())
                            .sum::<usize>(),
                )
            } else {
                None
            }
        }

        #[inline(always)]
        fn inputs_predicate_offset_at(&self, idx: usize) -> Option<(usize, usize)> {
            if let Some(CommonMetadata {
                inputs_predicate_offset_at: inputs_predicate_offset,
                ..
            }) = &self.metadata
            {
                return inputs_predicate_offset.get(idx).cloned().unwrap_or(None);
            }

            self.inputs().get(idx).and_then(|input| {
                input
                    .predicate_offset()
                    .and_then(|predicate| self.inputs_offset_at(idx).map(|inputs| inputs + predicate))
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
            if let Some(CommonMetadata { outputs_offset, .. }) = &self.metadata {
                return *outputs_offset;
            }

            self.inputs_offset() + self.inputs().iter().map(|i| i.serialized_size()).sum::<usize>()
        }

        #[inline(always)]
        fn outputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CommonMetadata {
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
            if let Some(CommonMetadata { witnesses_offset, .. }) = &self.metadata {
                return *witnesses_offset;
            }

            self.outputs_offset() + self.outputs().iter().map(|i| i.serialized_size()).sum::<usize>()
        }

        #[inline(always)]
        fn witnesses_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(CommonMetadata {
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
                            .map(|i| i.serialized_size())
                            .sum::<usize>(),
                )
            } else {
                None
            }
        }
    }
}

#[cfg(feature = "std")]
impl io::Read for Create {
    fn read(&mut self, full_buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if full_buf.len() < n {
            return Err(bytes::eof());
        }
        let buf: &mut [_; Self::LEN] = full_buf
            .get_mut(..Self::LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        bytes::store_number_at(
            buf,
            Self::layout(Self::LAYOUT.repr),
            crate::TransactionRepr::Create as u8,
        );
        let Create {
            gas_price,
            gas_limit,
            maturity,
            bytecode_length,
            bytecode_witness_index,
            salt,
            storage_slots,
            inputs,
            outputs,
            witnesses,
            ..
        } = self;

        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.gas_price), *gas_price);
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.gas_limit), *gas_limit);
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.maturity), **maturity);
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.bytecode_length), *bytecode_length);
        bytes::store_number_at(
            buf,
            Self::layout(Self::LAYOUT.bytecode_witness_index),
            *bytecode_witness_index,
        );
        bytes::store_number_at(
            buf,
            Self::layout(Self::LAYOUT.storage_slots_len),
            storage_slots.len() as Word,
        );
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.inputs_len), inputs.len() as Word);
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.outputs_len), outputs.len() as Word);
        bytes::store_number_at(buf, Self::layout(Self::LAYOUT.witnesses_len), witnesses.len() as Word);
        bytes::store_at(buf, Self::layout(Self::LAYOUT.salt), salt);

        let buf = full_buf.get_mut(Self::LEN..).ok_or(bytes::eof())?;
        let mut slot_len = 0;
        for (storage_slot, buf) in storage_slots
            .iter_mut()
            .zip(buf.chunks_exact_mut(StorageSlot::SLOT_SIZE))
        {
            let storage_len = storage_slot.read(buf)?;
            slot_len += storage_len;
            if storage_len != StorageSlot::SLOT_SIZE {
                return Err(bytes::eof());
            }
        }

        let mut buf = full_buf.get_mut(Self::LEN + slot_len..).ok_or(bytes::eof())?;
        for input in self.inputs.iter_mut() {
            let input_len = input.read(buf)?;
            buf = &mut buf[input_len..];
        }

        for output in self.outputs.iter_mut() {
            let output_len = output.read(buf)?;
            buf = &mut buf[output_len..];
        }

        for witness in self.witnesses.iter_mut() {
            let witness_len = witness.read(buf)?;
            buf = &mut buf[witness_len..];
        }

        Ok(n)
    }
}

#[cfg(feature = "std")]
impl io::Write for Create {
    fn write(&mut self, full_buf: &[u8]) -> io::Result<usize> {
        let buf: &[_; Self::LEN] = full_buf
            .get(..Self::LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;
        let mut n = crate::consts::TRANSACTION_CREATE_FIXED_SIZE;

        let identifier = bytes::restore_u8_at(buf, Self::layout(Self::LAYOUT.repr));
        let identifier = crate::TransactionRepr::try_from(identifier as Word)?;
        if identifier != crate::TransactionRepr::Create {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier to the `Create` is invalid!",
            ));
        }

        let gas_price = bytes::restore_number_at(buf, Self::layout(Self::LAYOUT.gas_price));
        let gas_limit = bytes::restore_number_at(buf, Self::layout(Self::LAYOUT.gas_limit));
        let maturity = bytes::restore_u32_at(buf, Self::layout(Self::LAYOUT.maturity)).into();
        let bytecode_length = bytes::restore_number_at(buf, Self::layout(Self::LAYOUT.bytecode_length));
        let bytecode_witness_index = bytes::restore_u8_at(buf, Self::layout(Self::LAYOUT.bytecode_witness_index));
        let storage_slots_len = bytes::restore_usize_at(buf, Self::layout(Self::LAYOUT.storage_slots_len));
        let inputs_len = bytes::restore_usize_at(buf, Self::layout(Self::LAYOUT.inputs_len));
        let outputs_len = bytes::restore_usize_at(buf, Self::layout(Self::LAYOUT.outputs_len));
        let witnesses_len = bytes::restore_usize_at(buf, Self::layout(Self::LAYOUT.witnesses_len));
        let salt = bytes::restore_at(buf, Self::layout(Self::LAYOUT.salt));

        let salt = salt.into();

        let mut buf = full_buf.get(Self::LEN..).ok_or(bytes::eof())?;
        let mut storage_slots = vec![StorageSlot::default(); storage_slots_len];
        n += StorageSlot::SLOT_SIZE * storage_slots_len;
        for storage_slot in storage_slots.iter_mut() {
            let _ = storage_slot.write(buf)?;
            buf = &buf[StorageSlot::SLOT_SIZE..];
        }

        let mut inputs = vec![Input::default(); inputs_len];
        for input in inputs.iter_mut() {
            let input_len = input.write(buf)?;
            buf = &buf[input_len..];
            n += input_len;
        }

        let mut outputs = vec![Output::default(); outputs_len];
        for output in outputs.iter_mut() {
            let output_len = output.write(buf)?;
            buf = &buf[output_len..];
            n += output_len;
        }

        let mut witnesses = vec![Witness::default(); witnesses_len];
        for witness in witnesses.iter_mut() {
            let witness_len = witness.write(buf)?;
            buf = &buf[witness_len..];
            n += witness_len;
        }

        *self = Create {
            gas_price,
            gas_limit,
            maturity,
            bytecode_length,
            bytecode_witness_index,
            salt,
            storage_slots,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        };

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inputs.iter_mut().try_for_each(|input| input.flush())?;
        self.outputs.iter_mut().try_for_each(|output| output.flush())?;
        self.witnesses.iter_mut().try_for_each(|witness| witness.flush())?;
        self.storage_slots.iter_mut().try_for_each(|slot| slot.flush())?;

        Ok(())
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

        let mut storage_slots_reverse = storage_slots;

        storage_slots_reverse.reverse();

        let err = Create {
            gas_price: 0,
            gas_limit: 0,
            maturity: Default::default(),
            bytecode_length: 0,
            bytecode_witness_index: 0,
            storage_slots: storage_slots_reverse,
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![Witness::default()],
            salt: Default::default(),
            metadata: None,
        }
        .check(0.into(), &ConsensusParameters::default())
        .expect_err("Expected erroneous transaction");

        assert_eq!(CheckError::TransactionCreateStorageSlotOrder, err);
    }
}
