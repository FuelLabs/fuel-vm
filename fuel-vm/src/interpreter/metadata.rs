use super::{
    internal::inc_pc,
    ExecutableTransaction,
    Interpreter,
};
use crate::{
    call::CallFrame,
    constraints::reg_key::*,
    consts::*,
    context::Context,
    convert,
    error::SimpleResult,
    interpreter::memory::read_bytes,
};

use fuel_asm::{
    GMArgs,
    GTFArgs,
    PanicReason,
    RegId,
};
use fuel_tx::{
    field::{
        BytecodeLength,
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

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn metadata(
        &mut self,
        ra: RegisterId,
        imm: Immediate18,
    ) -> SimpleResult<()> {
        let chain_id = self.chain_id();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        metadata(&self.context, &self.frames, pc, result, imm, chain_id)
    }

    pub(crate) fn get_transaction_field(
        &mut self,
        ra: RegisterId,
        b: Word,
        imm: Immediate12,
    ) -> SimpleResult<()> {
        let tx_offset = self.tx_offset();
        let tx_size = Word::from_be_bytes(
            read_bytes(
                &self.memory,
                (tx_offset - 8)// Tx size is stored just below the tx bytes
                    .try_into()
                    .expect("tx offset impossibly large"),
            )
            .expect("Tx length not in memory"),
        );
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = GTFInput {
            tx: &self.tx,
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
) -> SimpleResult<()> {
    let external = context.is_external();
    let args = GMArgs::try_from(imm)?;

    if external {
        match args {
            GMArgs::GetVerifyingPredicate => {
                *result = context
                    .predicate()
                    .map(|p| p.idx() as Word)
                    .ok_or(PanicReason::TransactionValidity)?;
            }

            GMArgs::GetChainId => {
                *result = chain_id.into();
            }

            _ => return Err(PanicReason::ExpectedInternalContext.into()),
        }
    } else {
        let parent = frames
            .last()
            .map(|f| f.registers()[RegId::FP])
            .expect("External context will always have a frame");

        match args {
            GMArgs::IsCallerExternal => {
                *result = (parent == 0) as Word;
            }

            GMArgs::GetCaller if parent != 0 => {
                *result = parent;
            }

            GMArgs::GetChainId => {
                *result = chain_id.into();
            }
            _ => return Err(PanicReason::ExpectedInternalContext.into()),
        }
    }

    inc_pc(pc)?;
    Ok(())
}

struct GTFInput<'vm, Tx> {
    tx: &'vm Tx,
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
        let ofs = self.tx_offset;

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
            GTFArgs::ScriptInputAtIndex | GTFArgs::CreateInputAtIndex => {
                (ofs + tx.inputs_offset_at(b).ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::ScriptOutputAtIndex | GTFArgs::CreateOutputAtIndex => {
                (ofs + tx.outputs_offset_at(b).ok_or(PanicReason::OutputNotFound)?)
                    as Word
            }
            GTFArgs::ScriptWitnessAtIndex | GTFArgs::CreateWitnessAtIndex => {
                (ofs + tx
                    .witnesses_offset_at(b)
                    .ok_or(PanicReason::WitnessNotFound)?) as Word
            }
            GTFArgs::TxStartAddress => ofs as Word,
            GTFArgs::TxLength => self.tx_size,

            // Input
            GTFArgs::InputType => {
                tx.inputs()
                    .get(b)
                    .map(InputRepr::from)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinTxId => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.utxo_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputCoinOutputIndex => {
                tx.inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::utxo_id)
                    .map(UtxoId::output_index)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputCoinOwner => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.owner_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputCoinAmount => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::amount)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputCoinAssetId => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.asset_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputCoinTxPointer => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .map(Input::repr)
                    .and_then(|r| r.tx_pointer_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
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
            GTFArgs::InputCoinPredicate => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputCoinPredicateData => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_coin())
                    .and_then(Input::predicate_data_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputContractTxId => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.utxo_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputContractOutputIndex => {
                tx.find_output_contract(b)
                    .map(|(idx, _o)| idx)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::InputContractId => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.contract_id_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputMessageSender => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_sender_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputMessageRecipient => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_recipient_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputMessageAmount => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::amount)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputMessageNonce => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_nonce_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
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
            GTFArgs::InputMessageData => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.data_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputMessagePredicate => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputMessagePredicateData => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .and_then(Input::predicate_data_offset)
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }

            // Output
            GTFArgs::OutputType => {
                tx.outputs()
                    .get(b)
                    .map(OutputRepr::from)
                    .ok_or(PanicReason::OutputNotFound)? as Word
            }
            GTFArgs::OutputCoinTo => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_coin() || o.is_change())
                    .map(Output::repr)
                    .and_then(|r| r.to_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputCoinAmount => tx
                .outputs()
                .get(b)
                .filter(|o| o.is_coin())
                .and_then(Output::amount)
                .ok_or(PanicReason::OutputNotFound)?,
            GTFArgs::OutputCoinAssetId => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_coin() || o.is_change())
                    .map(Output::repr)
                    .and_then(|r| r.asset_id_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputContractInputIndex => {
                tx.outputs()
                    .get(b)
                    .filter(|o| o.is_contract())
                    .and_then(Output::input_index)
                    .ok_or(PanicReason::InputNotFound)? as Word
            }
            GTFArgs::OutputContractCreatedContractId => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_contract_created())
                    .map(Output::repr)
                    .and_then(|r| r.contract_id_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputContractCreatedStateRoot => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_contract_created())
                    .map(Output::repr)
                    .and_then(|r| r.contract_created_state_root_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }

            // Witness
            GTFArgs::WitnessDataLength => {
                tx.witnesses()
                    .get(b)
                    .map(|w| w.as_ref().len())
                    .ok_or(PanicReason::WitnessNotFound)? as Word
            }
            GTFArgs::WitnessData => {
                tx.witnesses_offset_at(b)
                    .map(|w| ofs + w + WORD_SIZE)
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
                        (ofs + script.script_offset()) as Word
                    }
                    (Some(script), None, GTFArgs::ScriptData) => {
                        (ofs + script.script_data_offset()) as Word
                    }

                    // Create
                    (None, Some(create), GTFArgs::CreateBytecodeLength) => {
                        *create.bytecode_length() as Word
                    }
                    (None, Some(create), GTFArgs::CreateBytecodeWitnessIndex) => {
                        *create.bytecode_witness_index() as Word
                    }
                    (None, Some(create), GTFArgs::CreateStorageSlotsCount) => {
                        create.storage_slots().len() as Word
                    }
                    (None, Some(create), GTFArgs::CreateSalt) => {
                        (ofs + create.salt_offset()) as Word
                    }
                    (None, Some(create), GTFArgs::CreateStorageSlotAtIndex) => {
                        // TODO: Maybe we need to return panic error
                        // `StorageSlotsNotFound`?
                        (ofs + create.storage_slots_offset_at(b).unwrap_or_default())
                            as Word
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
