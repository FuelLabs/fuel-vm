use crate::{
    transaction::{
        consensus_parameters::TxParameters,
        field::{
            GasLimit,
            GasPrice,
            Inputs,
            Maturity,
            Outputs,
            ReceiptsRoot,
            Script as ScriptField,
            ScriptData,
            Witnesses,
        },
        metadata::CommonMetadata,
        validity::{
            check_common_part,
            check_size,
            FormatValidityChecks,
        },
        Chargeable,
    },
    CheckError,
    ConsensusParameters,
    Input,
    Output,
    TransactionRepr,
    Witness,
};
use derivative::Derivative;
use fuel_types::{
    bytes,
    bytes::WORD_SIZE,
    canonical::Serialize,
    fmt_truncated_hex,
    BlockHeight,
    Bytes32,
    ChainId,
    Word,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use hashbrown::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ScriptMetadata {
    pub common: CommonMetadata,
    pub script_data_offset: usize,
}

#[derive(Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Script)]
#[derivative(Eq, PartialEq, Hash, Debug)]
pub struct Script {
    pub(crate) gas_price: Word,
    pub(crate) gas_limit: Word,
    pub(crate) maturity: BlockHeight,
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    pub(crate) script: Vec<u8>,
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    pub(crate) script_data: Vec<u8>,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
    pub(crate) witnesses: Vec<Witness>,
    pub(crate) receipts_root: Bytes32,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
    #[canonical(skip)]
    pub(crate) metadata: Option<ScriptMetadata>,
}

impl Default for Script {
    fn default() -> Self {
        // Create a valid transaction with a single return instruction
        //
        // The Return op is mandatory for the execution of any context
        let script = fuel_asm::op::ret(0x10).to_bytes().to_vec();

        Self {
            gas_price: Default::default(),
            gas_limit: TxParameters::DEFAULT.max_gas_per_tx,
            maturity: Default::default(),
            script,
            script_data: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            receipts_root: Default::default(),
            metadata: None,
        }
    }
}

impl crate::UniqueIdentifier for Script {
    fn id(&self, chain_id: &ChainId) -> Bytes32 {
        if let Some(id) = self.cached_id() {
            return id
        }

        let mut clone = self.clone();

        // Empties fields that should be zero during the signing.
        *clone.receipts_root_mut() = Default::default();
        clone.inputs_mut().iter_mut().for_each(Input::prepare_sign);
        clone
            .outputs_mut()
            .iter_mut()
            .for_each(Output::prepare_sign);
        clone.witnesses_mut().clear();

        crate::transaction::compute_transaction_id(chain_id, &mut clone)
    }

    fn cached_id(&self) -> Option<Bytes32> {
        self.metadata.as_ref().map(|m| m.common.id)
    }
}

impl Chargeable for Script {
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

impl FormatValidityChecks for Script {
    fn check_signatures(&self, chain_id: &ChainId) -> Result<(), CheckError> {
        use crate::UniqueIdentifier;

        let id = self.id(chain_id);

        // There will be at most len(witnesses) signatures to cache
        let mut recovery_cache = Some(HashMap::with_capacity(self.witnesses().len()));

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
    ) -> Result<(), CheckError> {
        check_size(self, consensus_params.tx_params())?;

        check_common_part(
            self,
            block_height,
            consensus_params.tx_params(),
            consensus_params.predicate_params(),
            consensus_params.base_asset_id(),
        )?;
        let script_params = consensus_params.script_params();
        if self.script.len() > script_params.max_script_length as usize {
            Err(CheckError::TransactionScriptLength)?;
        }

        if self.script_data.len() > script_params.max_script_data_length as usize {
            Err(CheckError::TransactionScriptDataLength)?;
        }

        self.outputs
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| match output {
                Output::ContractCreated { .. } => {
                    Err(CheckError::TransactionScriptOutputContractCreated { index })
                }
                _ => Ok(()),
            })?;

        Ok(())
    }
}

impl crate::Cacheable for Script {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), CheckError> {
        self.metadata = None;
        self.metadata = Some(ScriptMetadata {
            common: CommonMetadata::compute(self, chain_id),
            script_data_offset: self.script_data_offset(),
        });
        Ok(())
    }
}

mod field {
    use super::*;

    impl GasPrice for Script {
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

    impl GasLimit for Script {
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

    impl Maturity for Script {
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

    impl ReceiptsRoot for Script {
        #[inline(always)]
        fn receipts_root(&self) -> &Bytes32 {
            &self.receipts_root
        }

        #[inline(always)]
        fn receipts_root_mut(&mut self) -> &mut Bytes32 {
            &mut self.receipts_root
        }

        #[inline(always)]
        fn receipts_root_offset_static() -> usize {
            Self::maturity_offset_static() + WORD_SIZE
                + WORD_SIZE // Script size
                + WORD_SIZE // Script data size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE // Witnesses size
        }
    }

    impl ScriptField for Script {
        #[inline(always)]
        fn script(&self) -> &Vec<u8> {
            &self.script
        }

        #[inline(always)]
        fn script_mut(&mut self) -> &mut Vec<u8> {
            &mut self.script
        }

        #[inline(always)]
        fn script_offset_static() -> usize {
            Self::receipts_root_offset_static() + Bytes32::LEN // Receipts root
        }
    }

    impl ScriptData for Script {
        #[inline(always)]
        fn script_data(&self) -> &Vec<u8> {
            &self.script_data
        }

        #[inline(always)]
        fn script_data_mut(&mut self) -> &mut Vec<u8> {
            &mut self.script_data
        }

        #[inline(always)]
        fn script_data_offset(&self) -> usize {
            if let Some(ScriptMetadata {
                script_data_offset, ..
            }) = &self.metadata
            {
                return *script_data_offset
            }

            self.script_offset() + bytes::padded_len(self.script.as_slice())
        }
    }

    impl Inputs for Script {
        #[inline(always)]
        fn inputs(&self) -> &Vec<Input> {
            &self.inputs
        }

        #[inline(always)]
        fn inputs_mut(&mut self) -> &mut Vec<Input> {
            &mut self.inputs
        }

        #[inline(always)]
        fn num_inputs(&self) -> usize {
            self.inputs.len()
        }

        #[inline(always)]
        fn inputs_offset(&self) -> usize {
            if let Some(ScriptMetadata {
                common: CommonMetadata { inputs_offset, .. },
                ..
            }) = &self.metadata
            {
                return *inputs_offset
            }

            self.script_data_offset() + bytes::padded_len(self.script_data.as_slice())
        }

        #[inline(always)]
        fn inputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(ScriptMetadata {
                common:
                    CommonMetadata {
                        inputs_offset_at, ..
                    },
                ..
            }) = &self.metadata
            {
                return inputs_offset_at.get(idx).cloned()
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
            if let Some(ScriptMetadata {
                common:
                    CommonMetadata {
                        inputs_predicate_offset_at,
                        ..
                    },
                ..
            }) = &self.metadata
            {
                return inputs_predicate_offset_at.get(idx).cloned().unwrap_or(None)
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

    impl Outputs for Script {
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
            if let Some(ScriptMetadata {
                common: CommonMetadata { outputs_offset, .. },
                ..
            }) = &self.metadata
            {
                return *outputs_offset
            }

            self.inputs_offset() + self.inputs().iter().map(|i| i.size()).sum::<usize>()
        }

        #[inline(always)]
        fn outputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(ScriptMetadata {
                common:
                    CommonMetadata {
                        outputs_offset_at, ..
                    },
                ..
            }) = &self.metadata
            {
                return outputs_offset_at.get(idx).cloned()
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

    impl Witnesses for Script {
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
            if let Some(ScriptMetadata {
                common:
                    CommonMetadata {
                        witnesses_offset, ..
                    },
                ..
            }) = &self.metadata
            {
                return *witnesses_offset
            }

            self.outputs_offset() + self.outputs().iter().map(|i| i.size()).sum::<usize>()
        }

        #[inline(always)]
        fn witnesses_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(ScriptMetadata {
                common:
                    CommonMetadata {
                        witnesses_offset_at,
                        ..
                    },
                ..
            }) = &self.metadata
            {
                return witnesses_offset_at.get(idx).cloned()
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
