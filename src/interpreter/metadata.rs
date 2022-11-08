use super::{ExecutableTransaction, Interpreter};
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::{GMArgs, GTFArgs, PanicReason};
use fuel_tx::field::{
    BytecodeLength, BytecodeWitnessIndex, ReceiptsRoot, Salt, Script as ScriptField, ScriptData, StorageSlots,
};
use fuel_tx::{Input, InputRepr, Output, OutputRepr, UtxoId};
use fuel_types::{Immediate12, Immediate18, RegisterId, Word};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn metadata(&mut self, ra: RegisterId, imm: Immediate18) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        let external = self.is_external_context();
        let args = GMArgs::try_from(imm)?;

        if external {
            match args {
                GMArgs::GetVerifyingPredicate => {
                    self.registers[ra] = self
                        .context
                        .predicate()
                        .map(|p| p.idx() as Word)
                        .ok_or(PanicReason::TransactionValidity)?;
                }

                _ => return Err(PanicReason::ExpectedInternalContext.into()),
            }
        } else {
            let parent = self
                .frames
                .last()
                .map(|f| f.registers()[REG_FP])
                .expect("External context will always have a frame");

            match args {
                GMArgs::IsCallerExternal => {
                    self.registers[ra] = (parent == 0) as Word;
                }

                GMArgs::GetCaller if parent != 0 => {
                    self.registers[ra] = parent;
                }

                _ => return Err(PanicReason::ExpectedInternalContext.into()),
            }
        }

        self.inc_pc()
    }

    pub(crate) fn get_transaction_field(
        &mut self,
        ra: RegisterId,
        b: Word,
        imm: Immediate12,
    ) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        let b = b as usize;
        let args = GTFArgs::try_from(imm)?;
        let tx = self.transaction();
        let ofs = self.tx_offset();

        let a = match args {
            GTFArgs::Type => Tx::transaction_type(),

            // General
            GTFArgs::ScriptGasPrice | GTFArgs::CreateGasPrice => tx.price(),
            GTFArgs::ScriptGasLimit | GTFArgs::CreateGasLimit => tx.limit(),
            GTFArgs::ScriptMaturity | GTFArgs::CreateMaturity => *tx.maturity(),
            GTFArgs::ScriptInputsCount | GTFArgs::CreateInputsCount => tx.inputs().len() as Word,
            GTFArgs::ScriptOutputsCount | GTFArgs::CreateOutputsCount => tx.outputs().len() as Word,
            GTFArgs::ScriptWitnessesCound | GTFArgs::CreateWitnessesCount => tx.witnesses().len() as Word,
            GTFArgs::ScriptInputAtIndex | GTFArgs::CreateInputAtIndex => {
                (ofs + tx.inputs_offset_at(b).ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::ScriptOutputAtIndex | GTFArgs::CreateOutputAtIndex => {
                (ofs + tx.outputs_offset_at(b).ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::ScriptWitnessAtIndex | GTFArgs::CreateWitnessAtIndex => {
                (ofs + tx.witnesses_offset_at(b).ok_or(PanicReason::WitnessNotFound)?) as Word
            }

            // Input
            GTFArgs::InputType => tx
                .inputs()
                .get(b)
                .map(InputRepr::from)
                .ok_or(PanicReason::InputNotFound)? as Word,
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
            GTFArgs::InputCoinOutputIndex => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::utxo_id)
                .map(UtxoId::output_index)
                .ok_or(PanicReason::InputNotFound)? as Word,
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
            GTFArgs::InputCoinWitnessIndex => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::witness_index)
                .ok_or(PanicReason::InputNotFound)? as Word,
            GTFArgs::InputCoinMaturity => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::maturity)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputCoinPredicateLength => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::predicate_len)
                .ok_or(PanicReason::InputNotFound)? as Word,
            GTFArgs::InputCoinPredicateDataLength => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_coin())
                .and_then(Input::predicate_data_len)
                .ok_or(PanicReason::InputNotFound)? as Word,
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
            GTFArgs::InputContractOutputIndex => tx
                .find_output_contract(b)
                .map(|(idx, _o)| idx)
                .ok_or(PanicReason::InputNotFound)? as Word,
            GTFArgs::InputContractBalanceRoot => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.contract_balance_root_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputContractStateRoot => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.contract_state_root_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
            }
            GTFArgs::InputContractTxPointer => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_contract())
                    .map(Input::repr)
                    .and_then(|r| r.tx_pointer_offset())
                    .and_then(|ofs| tx.inputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::InputNotFound)?) as Word
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
            GTFArgs::InputMessageId => {
                (ofs + tx
                    .inputs()
                    .get(b)
                    .filter(|i| i.is_message())
                    .map(Input::repr)
                    .and_then(|r| r.message_id_offset())
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
            GTFArgs::InputMessageNonce => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::nonce)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputMessageWitnessIndex => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::witness_index)
                .ok_or(PanicReason::InputNotFound)? as Word,
            GTFArgs::InputMessageDataLength => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::input_data)
                .map(|d| d.len() as Word)
                .ok_or(PanicReason::InputNotFound)?,
            GTFArgs::InputMessagePredicateLength => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::predicate_len)
                .ok_or(PanicReason::InputNotFound)? as Word,
            GTFArgs::InputMessagePredicateDataLength => tx
                .inputs()
                .get(b)
                .filter(|i| i.is_message())
                .and_then(Input::predicate_data_len)
                .ok_or(PanicReason::InputNotFound)? as Word,
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
            GTFArgs::OutputType => tx
                .outputs()
                .get(b)
                .map(OutputRepr::from)
                .ok_or(PanicReason::OutputNotFound)? as Word,
            GTFArgs::OutputCoinTo => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_coin())
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
                    .filter(|o| o.is_coin())
                    .map(Output::repr)
                    .and_then(|r| r.asset_id_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputContractInputIndex => tx
                .outputs()
                .get(b)
                .filter(|o| o.is_contract())
                .and_then(Output::input_index)
                .ok_or(PanicReason::InputNotFound)? as Word,
            GTFArgs::OutputContractBalanceRoot => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_contract())
                    .map(Output::repr)
                    .and_then(|r| r.contract_balance_root_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputContractStateRoot => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_contract())
                    .map(Output::repr)
                    .and_then(|r| r.contract_state_root_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputMessageRecipient => {
                (ofs + tx
                    .outputs()
                    .get(b)
                    .filter(|o| o.is_message())
                    .map(Output::repr)
                    .and_then(|r| r.recipient_offset())
                    .and_then(|ofs| tx.outputs_offset_at(b).map(|o| o + ofs))
                    .ok_or(PanicReason::OutputNotFound)?) as Word
            }
            GTFArgs::OutputMessageAmount => tx
                .outputs()
                .get(b)
                .filter(|o| o.is_message())
                .and_then(Output::amount)
                .ok_or(PanicReason::OutputNotFound)?,
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
            GTFArgs::WitnessDataLength => tx
                .witnesses()
                .get(b)
                .map(|w| w.as_ref().len())
                .ok_or(PanicReason::WitnessNotFound)? as Word,
            GTFArgs::WitnessData => tx
                .witnesses_offset_at(b)
                .map(|w| ofs + w + WORD_SIZE)
                .ok_or(PanicReason::WitnessNotFound)? as Word,

            // If it is not any above commands, it is something specific to the transaction type.
            specific_args => {
                let as_script = tx.as_script();
                let as_create = tx.as_create();
                match (as_script, as_create, specific_args) {
                    // Script
                    (Some(script), None, GTFArgs::ScriptLength) => script.script().len() as Word,
                    (Some(script), None, GTFArgs::ScriptDataLength) => script.script_data().len() as Word,
                    (Some(script), None, GTFArgs::ScriptReceiptsRoot) => (ofs + script.receipts_root_offset()) as Word,
                    (Some(script), None, GTFArgs::Script) => (ofs + script.script_offset()) as Word,
                    (Some(script), None, GTFArgs::ScriptData) => (ofs + script.script_data_offset()) as Word,

                    // Create
                    (None, Some(create), GTFArgs::CreateBytecodeLength) => *create.bytecode_length() as Word,
                    (None, Some(create), GTFArgs::CreateBytecodeWitnessIndex) => {
                        *create.bytecode_witness_index() as Word
                    }
                    (None, Some(create), GTFArgs::CreateStorageSlotsCount) => create.storage_slots().len() as Word,
                    (None, Some(create), GTFArgs::CreateSalt) => (ofs + create.salt_offset()) as Word,
                    (None, Some(create), GTFArgs::CreateStorageSlotAtIndex) => {
                        // TODO: Maybe we need to return panic error `StorageSlotsNotFound`?
                        (ofs + create.storage_slots_offset_at(b).unwrap_or_default()) as Word
                    }
                    _ => return Err(PanicReason::InvalidMetadataIdentifier.into()),
                }
            }
        };

        self.registers[ra] = a;

        self.inc_pc()
    }
}
