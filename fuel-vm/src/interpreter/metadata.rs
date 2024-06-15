use super::{
    internal::inc_pc,
    ExecutableTransaction,
    Interpreter,
    Memory,
};
use crate::{
    call::CallFrame,
    constraints::reg_key::*,
    consts::*,
    context::Context,
    convert,
    error::SimpleResult,
};

use fuel_asm::{
    GMArgs,
    GTFArgs,
    PanicReason,
    RegId,
};
use fuel_tx::{
    field::{
        BytecodeWitnessIndex,
        Salt,
        Script as ScriptField,
        ScriptData,
        ScriptGasLimit,
        StorageSlots,
    },
    policies::PolicyType,
    Input,
    InputRepr,
    Output,
    OutputRepr,
    UtxoId,
};
use fuel_types::{
    ChainId,
    Immediate12,
    Immediate18,
    RegisterId,
    Word,
};

#[cfg(test)]
mod tests;

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
    Tx: ExecutableTransaction,
{
    pub(crate) fn metadata(
        &mut self,
        ra: RegisterId,
        imm: Immediate18,
    ) -> SimpleResult<()> {
        let tx_offset = self.tx_offset() as Word;
        let chain_id = self.chain_id();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        metadata(
            &self.context,
            &self.frames,
            pc,
            result,
            imm,
            chain_id,
            tx_offset,
        )
    }

    pub(crate) fn get_transaction_field(
        &mut self,
        ra: RegisterId,
        b: Word,
        imm: Immediate12,
    ) -> SimpleResult<()> {
        let tx_offset = self.tx_offset();
        // Tx size is stored just below the tx bytes
        let tx_size_ptr = tx_offset.checked_sub(8).expect("Tx offset is not valid");
        let tx_size = Word::from_be_bytes(
            self.memory()
                .read_bytes(tx_size_ptr)
                .expect("Tx length not in memory"),
        );
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = GTFInput {
            tx: &self.tx,
            input_contracts_index_to_output_index: &self
                .input_contracts_index_to_output_index,
            tx_offset,
            tx_size,
            pc,
        };
        input.get_transaction_field(result, b, imm)
    }
}

pub(crate) fn metadata(
    context: &Context,
    frames: &[CallFrame],
    pc: RegMut<PC>,
    result: &mut Word,
    imm: Immediate18,
    chain_id: ChainId,
    tx_offset: Word,
) -> SimpleResult<()> {
    let parent = context
        .is_internal()
        .then(|| frames.last().map(|f| f.registers()[RegId::FP]))
        .flatten();

    *result = match GMArgs::try_from(imm)? {
        GMArgs::GetVerifyingPredicate => context
            .predicate()
            .map(|p| p.idx() as Word)
            .ok_or(PanicReason::TransactionValidity)?,
        GMArgs::GetChainId => chain_id.into(),
        GMArgs::BaseAssetId => VM_MEMORY_BASE_ASSET_ID_OFFSET as Word,
        GMArgs::TxStart => tx_offset,
        GMArgs::GetCaller => match parent {
            Some(0) => return Err(PanicReason::ExpectedNestedCaller.into()),
            Some(parent) => parent,
            None => return Err(PanicReason::ExpectedInternalContext.into()),
        },
        GMArgs::IsCallerExternal => match parent {
            Some(p) => (p == 0) as Word,
            None => return Err(PanicReason::ExpectedInternalContext.into()),
        },
    };

    inc_pc(pc)?;
    Ok(())
}

struct GTFInput<'vm, Tx> {
    tx: &'vm Tx,
    input_contracts_index_to_output_index: &'vm alloc::collections::BTreeMap<u16, u16>,
    tx_offset: usize,
    tx_size: Word,
    pc: RegMut<'vm, PC>,
}

impl<Tx> GTFInput<'_, Tx> {
    pub(crate) fn get_transaction_field(
        self,
        result: &mut Word,
        b: Word,
        imm: Immediate12,
    ) -> SimpleResult<()>
    where
        Tx: ExecutableTransaction,
    {
        let b = convert::to_usize(b).ok_or(PanicReason::InvalidMetadataIdentifier)?;
        let args = GTFArgs::try_from(imm)?;
        let tx = self.tx;
        let input_contract_to_output_index = self.input_contracts_index_to_output_index;
        let ofs = self.tx_offset;

        // We use saturating_add with tx offset below.
        // In case any addition overflows, this function returns value
        // for the field that's above VM_MAX_RAM.

        let a = match args {
            GTFArgs::Type => Tx::transaction_type(),

            // General
            GTFArgs::ScriptGasLimit => tx
                .as_script()
                .map(|script| *script.script_gas_limit())
                .unwrap_or_default(),
            GTFArgs::PolicyTypes => tx.policies().bits() as Word,
            GTFArgs::PolicyTip => tx
                .policies()
                .get(PolicyType::Tip)
                .ok_or(PanicReason::PolicyIsNotSet)?,
            GTFArgs::PolicyWitnessLimit => tx
                .policies()
                .get(PolicyType::WitnessLimit)
                .ok_or(PanicReason::PolicyIsNotSet)?,
            GTFArgs::PolicyMaturity => tx
                .policies()
                .get(PolicyType::Maturity)
                .ok_or(PanicReason::PolicyIsNotSet)?,
            GTFArgs::PolicyMaxFee => tx
                .policies()
                .get(PolicyType::MaxFee)
                .ok_or(PanicReason::PolicyIsNotSet)?,
            GTFArgs::ScriptInputsCount | GTFArgs::CreateInputsCount => {
                tx.inputs().len() as Word
            }
            GTFArgs::ScriptOutputsCount | GTFArgs::CreateOutputsCount => {
                tx.outputs().len() as Word
            }
            GTFArgs::ScriptWitnessesCount | GTFArgs::CreateWitnessesCount => {
                tx.witnesses().len() as Word
            }
            GTFArgs::ScriptInputAtIndex | GTFArgs::CreateInputAtIndex => ofs
                .saturating_add(tx.inputs_offset_at(b).ok_or(PanicReason::InputNotFound)?)
                as Word,
            GTFArgs::ScriptOutputAtIndex | GTFArgs::CreateOutputAtIndex => {
                ofs.saturating_add(
                    tx.outputs_offset_at(b).ok_or(PanicReason::OutputNotFound)?,
                ) as Word
            }
            GTFArgs::ScriptWitnessAtIndex | GTFArgs::CreateWitnessAtIndex => {
                ofs.saturating_add(
                    tx.witnesses_offset_at(b)
                        .ok_or(PanicReason::WitnessNotFound)?,
                ) as Word
            }
            GTFArgs::TxLength => self.tx_size,

            // Input
            GTFArgs::InputType => {
                tx.inputs()
                    .get(b)
                    .map(InputRepr::from)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinTxId => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.utxo_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputCoinOutputIndex => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::utxo_id)
                    .map(UtxoId::output_index)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinOwner => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.owner_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputCoinAmount => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::amount)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputCoinAssetId => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.asset_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputCoinTxPointer => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.tx_pointer_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputCoinWitnessIndex => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::witness_index)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinPredicateLength => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_len)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinPredicateDataLength => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_data_len)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinPredicateGasUsed => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_gas_used)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinPredicate => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputCoinPredicateData => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_data_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputContractTxId => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.utxo_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputContractOutputIndex => {
                let b = u16::try_from(b)
                    .map_err(|_| PanicReason::InvalidMetadataIdentifier)?;
                input_contract_to_output_index
                    .get(&b)
                    .copied()
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputContractId => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.contract_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputMessageSender => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_sender_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputMessageRecipient => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_recipient_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputMessageAmount => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::amount)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputMessageNonce => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_nonce_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputMessageWitnessIndex => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::witness_index)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputMessageDataLength => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::input_data_len)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputMessagePredicateLength => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_len)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputMessagePredicateDataLength => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_data_len)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputMessagePredicateGasUsed => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_gas_used)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputMessageData => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.data_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputMessagePredicate => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,
            GTFArgs::InputMessagePredicateData => ofs.saturating_add(
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_data_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o.saturating_add(ofs)))
                    .ok_or(PanicReason::InputNotFound)?,
            ) as Word,

            // Output
            GTFArgs::OutputType => {
                tx.outputs()
                    .get(b)
                    .map(OutputRepr::from)
                    .ok_or(PanicReason::OutputNotFound)? as Word
            }
            GTFArgs::OutputCoinTo => ofs.saturating_add(
                tx.outputs()
                    .get(b)
                    .filter(|o| o.is_coin() || o.is_change())
                    .map(Output::repr)
                    .and_then(|r| r.to_offset())
                    .and_then(|ofs| {
                        tx.outputs_offset_at(b).map(|o| o.saturating_add(ofs))
                    })
                    .ok_or(PanicReason::OutputNotFound)?,
            ) as Word,
            GTFArgs::OutputCoinAmount => tx
                .outputs()
                .get(b)
                .filter(|o| o.is_coin())
                .and_then(Output::amount)
                .ok_or(PanicReason::OutputNotFound)?,
            GTFArgs::OutputCoinAssetId => ofs.saturating_add(
                tx.outputs()
                    .get(b)
                    .filter(|o| o.is_coin() || o.is_change())
                    .map(Output::repr)
                    .and_then(|r| r.asset_id_offset())
                    .and_then(|ofs| {
                        tx.outputs_offset_at(b).map(|o| o.saturating_add(ofs))
                    })
                    .ok_or(PanicReason::OutputNotFound)?,
            ) as Word,
            GTFArgs::OutputContractInputIndex => {
                tx.outputs()
                    .get(b)
                    .filter(|o| o.is_contract())
                    .and_then(Output::input_index)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::OutputContractCreatedContractId => ofs.saturating_add(
                tx.outputs()
                    .get(b)
                    .filter(|o| o.is_contract_created())
                    .map(Output::repr)
                    .and_then(|r| r.contract_id_offset())
                    .and_then(|ofs| {
                        tx.outputs_offset_at(b).map(|o| o.saturating_add(ofs))
                    })
                    .ok_or(PanicReason::OutputNotFound)?,
            ) as Word,
            GTFArgs::OutputContractCreatedStateRoot => ofs.saturating_add(
                tx.outputs()
                    .get(b)
                    .filter(|o| o.is_contract_created())
                    .map(Output::repr)
                    .and_then(|r| r.contract_created_state_root_offset())
                    .and_then(|ofs| {
                        tx.outputs_offset_at(b).map(|o| o.saturating_add(ofs))
                    })
                    .ok_or(PanicReason::OutputNotFound)?,
            ) as Word,

            // Witness
            GTFArgs::WitnessDataLength => {
                tx.witnesses()
                    .get(b)
                    .map(|w| w.as_ref().len())
                    .ok_or(PanicReason::WitnessNotFound)? as Word
            }
            GTFArgs::WitnessData => {
                tx.witnesses_offset_at(b)
                    .map(|w| ofs.saturating_add(w).saturating_add(WORD_SIZE))
                    .ok_or(PanicReason::WitnessNotFound)? as Word
            }

            // If it is not any above commands, it is something specific to the
            // transaction type.
            specific_args => {
                let as_script = tx.as_script();
                let as_create = tx.as_create();
                match (as_script, as_create, specific_args) {
                    // Script
                    (Some(script), None, GTFArgs::ScriptLength) => {
                        script.script().len() as Word
                    }
                    (Some(script), None, GTFArgs::ScriptDataLength) => {
                        script.script_data().len() as Word
                    }
                    (Some(script), None, GTFArgs::Script) => {
                        ofs.saturating_add(script.script_offset()) as Word
                    }
                    (Some(script), None, GTFArgs::ScriptData) => {
                        ofs.saturating_add(script.script_data_offset()) as Word
                    }

                    // Create
                    (None, Some(create), GTFArgs::CreateBytecodeWitnessIndex) => {
                        *create.bytecode_witness_index() as Word
                    }
                    (None, Some(create), GTFArgs::CreateStorageSlotsCount) => {
                        create.storage_slots().len() as Word
                    }
                    (None, Some(create), GTFArgs::CreateSalt) => {
                        ofs.saturating_add(create.salt_offset()) as Word
                    }
                    (None, Some(create), GTFArgs::CreateStorageSlotAtIndex) => {
                        // TODO: Maybe we need to return panic error
                        // `StorageSlotsNotFound`?
                        (ofs.saturating_add(
                            create.storage_slots_offset_at(b).unwrap_or_default(),
                        )) as Word
                    }
                    _ => return Err(PanicReason::InvalidMetadataIdentifier.into()),
                }
            }
        };

        *result = a;

        inc_pc(self.pc)?;
        Ok(())
    }
}
