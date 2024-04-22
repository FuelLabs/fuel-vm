use crate::{
    field::WitnessLimit,
    transaction::{
        field::{
            ReceiptsRoot,
            Script as ScriptField,
            ScriptData,
            ScriptGasLimit,
            Witnesses,
        },
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
    Output,
    TransactionRepr,
    ValidityError,
};
use derivative::Derivative;
use fuel_types::{
    bytes,
    bytes::WORD_SIZE,
    canonical::Serialize,
    fmt_truncated_hex,
    Bytes32,
    ChainId,
    Word,
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub type Script = ChargeableTransaction<ScriptBody, ScriptMetadata>;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptMetadata {
    pub script_data_offset: usize,
}

#[derive(Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Script)]
#[derivative(Eq, PartialEq, Hash, Debug)]
pub struct ScriptBody {
    pub(crate) script_gas_limit: Word,
    pub(crate) receipts_root: Bytes32,
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    pub(crate) script: Vec<u8>,
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    pub(crate) script_data: Vec<u8>,
}

impl Default for ScriptBody {
    fn default() -> Self {
        // Create a valid transaction with a single return instruction
        //
        // The Return op is mandatory for the execution of any context
        let script = fuel_asm::op::ret(0x10).to_bytes().to_vec();

        Self {
            script_gas_limit: Default::default(),
            receipts_root: Default::default(),
            script,
            script_data: Default::default(),
        }
    }
}

impl PrepareSign for ScriptBody {
    fn prepare_sign(&mut self) {
        // Prepare script for execution by clearing malleable fields.
        self.receipts_root = Default::default();
    }
}

impl Chargeable for Script {
    #[inline(always)]
    fn max_gas(&self, gas_costs: &GasCosts, fee: &FeeParameters) -> fuel_asm::Word {
        // The basic implementation of the `max_gas` + `gas_limit`.
        let remaining_allowed_witness = self
            .witness_limit()
            .saturating_sub(self.witnesses().size_dynamic() as u64)
            .saturating_mul(fee.gas_per_byte());

        self.min_gas(gas_costs, fee)
            .saturating_add(remaining_allowed_witness)
            .saturating_add(self.body.script_gas_limit)
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

impl UniqueFormatValidityChecks for Script {
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        let script_params = consensus_params.script_params();
        if self.body.script.len() as u64 > script_params.max_script_length() {
            Err(ValidityError::TransactionScriptLength)?;
        }

        if self.body.script_data.len() as u64 > script_params.max_script_data_length() {
            Err(ValidityError::TransactionScriptDataLength)?;
        }

        self.outputs
            .iter()
            .enumerate()
            .try_for_each(|(index, output)| match output {
                Output::ContractCreated { .. } => {
                    Err(ValidityError::TransactionOutputContainsContractCreated { index })
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

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.metadata = None;
        self.metadata = Some(ChargeableMetadata {
            common: CommonMetadata::compute(self, chain_id)?,
            body: ScriptMetadata {
                script_data_offset: self.script_data_offset(),
            },
        });
        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::ChargeableBody;

    impl ScriptGasLimit for Script {
        #[inline(always)]
        fn script_gas_limit(&self) -> &Word {
            &self.body.script_gas_limit
        }

        #[inline(always)]
        fn script_gas_limit_mut(&mut self) -> &mut Word {
            &mut self.body.script_gas_limit
        }

        #[inline(always)]
        fn script_gas_limit_offset_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl ReceiptsRoot for Script {
        #[inline(always)]
        fn receipts_root(&self) -> &Bytes32 {
            &self.body.receipts_root
        }

        #[inline(always)]
        fn receipts_root_mut(&mut self) -> &mut Bytes32 {
            &mut self.body.receipts_root
        }

        #[inline(always)]
        fn receipts_root_offset_static() -> usize {
            Self::script_gas_limit_offset_static().saturating_add(WORD_SIZE)
        }
    }

    impl ScriptField for Script {
        #[inline(always)]
        fn script(&self) -> &Vec<u8> {
            &self.body.script
        }

        #[inline(always)]
        fn script_mut(&mut self) -> &mut Vec<u8> {
            &mut self.body.script
        }

        #[inline(always)]
        fn script_offset_static() -> usize {
            Self::receipts_root_offset_static().saturating_add(
                Bytes32::LEN // Receipts root
                + WORD_SIZE // Script size
                + WORD_SIZE // Script data size
                + WORD_SIZE // Policies size
                + WORD_SIZE // Inputs size
                + WORD_SIZE // Outputs size
                + WORD_SIZE, // Witnesses size
            )
        }
    }

    impl ScriptData for Script {
        #[inline(always)]
        fn script_data(&self) -> &Vec<u8> {
            &self.body.script_data
        }

        #[inline(always)]
        fn script_data_mut(&mut self) -> &mut Vec<u8> {
            &mut self.body.script_data
        }

        #[inline(always)]
        fn script_data_offset(&self) -> usize {
            if let Some(ChargeableMetadata { body, .. }) = &self.metadata {
                return body.script_data_offset;
            }

            self.script_offset().saturating_add(
                bytes::padded_len(self.body.script.as_slice()).unwrap_or(usize::MAX),
            )
        }
    }

    impl ChargeableBody<ScriptBody> for Script {
        fn body(&self) -> &ScriptBody {
            &self.body
        }

        fn body_mut(&mut self) -> &mut ScriptBody {
            &mut self.body
        }

        fn body_offset_end(&self) -> usize {
            self.script_data_offset().saturating_add(
                bytes::padded_len(self.body.script_data.as_slice()).unwrap_or(usize::MAX),
            )
        }
    }
}
