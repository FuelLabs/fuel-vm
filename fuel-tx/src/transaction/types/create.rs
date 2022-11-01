use crate::transaction::{
    checkable::{check_common_part, Checkable},
    field::{
        BytecodeLength, BytecodeWitnessIndex, GasLimit, GasPrice, Inputs, Maturity, Outputs,
        Salt as SaltField, StorageSlots, Witnesses,
    },
    metadata::CommonMetadata,
};
use crate::{
    Chargeable, CheckError, ConsensusParameters, Contract, Input, Output, StorageSlot, Witness,
};
use derivative::Derivative;
use fuel_types::bytes::{SizedBytes, WORD_SIZE};
use fuel_types::{bytes, AssetId, Salt, Word};

#[cfg(feature = "std")]
use std::io;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use fuel_types::bytes::SerializableVec;

#[derive(Default, Debug, Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derivative(Eq, PartialEq, Hash)]
pub struct Create {
    pub(crate) gas_price: Word,
    pub(crate) gas_limit: Word,
    pub(crate) maturity: Word,
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

#[cfg(feature = "std")]
impl crate::UniqueIdentifier for Create {
    fn id(&self) -> fuel_types::Bytes32 {
        if let Some(CommonMetadata { id, .. }) = self.metadata {
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

        fuel_crypto::Hasher::hash(clone.to_bytes().as_slice())
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

impl Checkable for Create {
    #[cfg(feature = "std")]
    fn check_signatures(&self) -> Result<(), CheckError> {
        use crate::UniqueIdentifier;

        let id = self.id();

        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| input.check_signature(index, &id, &self.witnesses))?;

        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: Word,
        parameters: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        check_common_part(self, block_height, parameters)?;

        let bytecode_witness_len = self
            .witnesses
            .get(self.bytecode_witness_index as usize)
            .map(|w| w.as_ref().len() as Word)
            .ok_or(CheckError::TransactionCreateBytecodeWitnessIndex)?;

        if bytecode_witness_len > parameters.contract_max_size
            || bytecode_witness_len / 4 != self.bytecode_length
        {
            return Err(CheckError::TransactionCreateBytecodeLen);
        }

        // Restrict to subset of u16::MAX, allowing this to be increased in the future
        // in a non-breaking way.
        if self.storage_slots.len() > parameters.max_storage_slots as usize {
            return Err(CheckError::TransactionCreateStorageSlotMax);
        }

        if !self
            .storage_slots
            .as_slice()
            .windows(2)
            .all(|s| s[0] <= s[1])
        {
            return Err(CheckError::TransactionCreateStorageSlotOrder);
        }

        // TODO The computed contract ADDRESS (see below) is not equal to the
        // contractADDRESS of the one OutputType.ContractCreated output

        self.inputs
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                if let Input::Contract { .. } = input {
                    return Err(CheckError::TransactionCreateInputContract { index });
                }

                Ok(())
            })?;

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

    fn precompute(&mut self) {
        self.metadata = None;
        self.metadata = Some(CommonMetadata::compute(self));
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

#[cfg(feature = "std")]
pub mod checked {
    use crate::checked_transaction::{initial_free_balances, AvailableBalances};
    use crate::{
        Cacheable, CheckError, Checkable, Checked, ConsensusParameters, Create, IntoChecked,
        TransactionFee,
    };
    use fuel_types::{AssetId, Word};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The mapping of initial free balances
        pub initial_free_balances: BTreeMap<AssetId, Word>,
        /// The block height this tx was verified with
        pub block_height: Word,
        /// The fees and gas usage
        pub fee: TransactionFee,
    }

    impl IntoChecked for Create {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: Word,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute();
            self.check_without_signatures(block_height, params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                initial_free_balances,
                fee,
            } = initial_free_balances(&self, params)?;

            let metadata = CheckedMetadata {
                initial_free_balances,
                block_height,
                fee,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

mod field {
    use super::*;

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
        fn maturity(&self) -> &Word {
            &self.maturity
        }

        #[inline(always)]
        fn maturity_mut(&mut self) -> &mut Word {
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
        fn storage_slots_mut(&mut self) -> &mut Vec<StorageSlot> {
            &mut self.storage_slots
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
            if let Some(CommonMetadata { outputs_offset, .. }) = &self.metadata {
                return *outputs_offset;
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
            if let Some(CommonMetadata {
                witnesses_offset, ..
            }) = &self.metadata
            {
                return *witnesses_offset;
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
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let buf = bytes::store_number_unchecked(buf, crate::TransactionRepr::Create as Word);
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

        let mut buf = {
            let buf = bytes::store_number_unchecked(buf, *gas_price);
            let buf = bytes::store_number_unchecked(buf, *gas_limit);
            let buf = bytes::store_number_unchecked(buf, *maturity);
            let buf = bytes::store_number_unchecked(buf, *bytecode_length);
            let buf = bytes::store_number_unchecked(buf, *bytecode_witness_index);
            let buf = bytes::store_number_unchecked(buf, storage_slots.len() as Word);
            let buf = bytes::store_number_unchecked(buf, inputs.len() as Word);
            let buf = bytes::store_number_unchecked(buf, outputs.len() as Word);
            let buf = bytes::store_number_unchecked(buf, witnesses.len() as Word);
            let mut buf = bytes::store_array_unchecked(buf, salt);

            for storage_slot in storage_slots.iter_mut() {
                let storage_len = storage_slot.read(buf)?;
                buf = &mut buf[storage_len..];
            }

            buf
        };

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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut n = crate::consts::TRANSACTION_CREATE_FIXED_SIZE;
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let (identifier, buf): (Word, _) = unsafe { bytes::restore_number_unchecked(buf) };
        let identifier = crate::TransactionRepr::try_from(identifier)?;
        if identifier != crate::TransactionRepr::Create {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier to the `Create` is invalid!",
            ));
        }

        // Safety: buffer size is checked
        let (gas_price, buf) = unsafe { bytes::restore_number_unchecked(buf) };
        let (gas_limit, buf) = unsafe { bytes::restore_number_unchecked(buf) };
        let (maturity, buf) = unsafe { bytes::restore_number_unchecked(buf) };
        let (bytecode_length, buf) = unsafe { bytes::restore_number_unchecked(buf) };
        let (bytecode_witness_index, buf) = unsafe { bytes::restore_u8_unchecked(buf) };
        let (storage_slots_len, buf) = unsafe { bytes::restore_u16_unchecked(buf) };
        let (inputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
        let (outputs_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
        let (witnesses_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
        let (salt, mut buf) = unsafe { bytes::restore_array_unchecked(buf) };

        let salt = salt.into();

        let mut storage_slots = vec![StorageSlot::default(); storage_slots_len as usize];
        n += StorageSlot::SLOT_SIZE * storage_slots_len as usize;
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
        self.outputs
            .iter_mut()
            .try_for_each(|output| output.flush())?;
        self.witnesses
            .iter_mut()
            .try_for_each(|witness| witness.flush())?;
        self.storage_slots
            .iter_mut()
            .try_for_each(|slot| slot.flush())?;

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
