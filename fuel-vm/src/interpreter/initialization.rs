use super::{
    ExecutableTransaction,
    InitialBalances,
    Interpreter,
    Memory,
    RuntimeBalances,
};
use crate::{
    checked_transaction::{
        IntoChecked,
        Ready,
    },
    consts::*,
    context::Context,
    error::InterpreterError,
    prelude::RuntimeError,
    storage::InterpreterStorage,
};
use fuel_asm::RegId;
use fuel_tx::{
    field::{
        Script,
        ScriptGasLimit,
    }, Input, InputV1, Output
};
use fuel_types::Word;

use crate::interpreter::CheckedMetadata;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    /// Initialize the VM with a given transaction
    fn init_inner(
        &mut self,
        mut tx: Tx,
        initial_balances: InitialBalances,
        runtime_balances: RuntimeBalances,
        gas_limit: Word,
    ) -> Result<(), RuntimeError<S::DataError>> {
        tx.prepare_sign();
        self.tx = tx;
        self.input_contracts = self
            .tx
            .inputs()
            .iter()
            .filter_map(|i| match i {
                Input::V1(InputV1::Contract(contract)) => Some(contract.contract_id),
                _ => None,
            })
            .collect();

        self.input_contracts_index_to_output_index = self
            .tx
            .outputs()
            .iter()
            .enumerate()
            .filter_map(|(output_idx, o)| match o {
                Output::Contract(fuel_tx::output::contract::Contract {
                    input_index,
                    ..
                }) => Some((
                    *input_index,
                    u16::try_from(output_idx)
                        .expect("The maximum number of outputs is `u16::MAX`"),
                )),
                _ => None,
            })
            .collect();

        self.initial_balances = initial_balances.clone();

        self.frames.clear();
        self.receipts.clear();
        self.memory_mut().reset();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[RegId::ONE] = 1;

        // Set heap area
        self.registers[RegId::HP] = VM_MAX_RAM;

        // Initialize stack
        macro_rules! push_stack {
            ($v:expr) => {{
                let data = $v;
                let old_ssp = self.registers[RegId::SSP];
                let new_ssp = old_ssp
                    .checked_add(data.len() as Word)
                    .expect("VM initialization data must fit into the stack");
                self.memory_mut().grow_stack(new_ssp)?;
                self.registers[RegId::SSP] = new_ssp;
                self.memory_mut()
                    .write_noownerchecks(old_ssp, data.len())
                    .expect("VM initialization data must fit into the stack")
                    .copy_from_slice(data);
            }};
        }

        push_stack!(&*self.transaction().id(&self.chain_id()));

        let base_asset_id = self.interpreter_params.base_asset_id;
        push_stack!(&*base_asset_id);

        runtime_balances.to_vm(self);

        let tx_size = self.transaction().size() as Word;
        self.set_gas(gas_limit);

        push_stack!(&tx_size.to_be_bytes());

        let tx_bytes = self.tx.to_bytes();
        push_stack!(tx_bytes.as_slice());

        self.registers[RegId::SP] = self.registers[RegId::SSP];

        Ok(())
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    /// Initialize the VM for a predicate context
    pub fn init_predicate(
        &mut self,
        context: Context,
        tx: Tx,
        gas_limit: Word,
    ) -> Result<(), InterpreterError<S::DataError>> {
        self.context = context;
        let initial_balances: InitialBalances = Default::default();
        let runtime_balances = initial_balances.clone().try_into()?;

        let range = self
            .context
            .predicate()
            .expect("The context is not predicate")
            .program()
            .words();

        self.init_inner(tx, initial_balances, runtime_balances, gas_limit)?;
        self.registers[RegId::PC] = range.start as fuel_asm::Word;
        self.registers[RegId::IS] = range.start as fuel_asm::Word;

        Ok(())
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    <S as InterpreterStorage>::DataError: From<S::DataError>,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
{
    /// Initialize the VM with a given transaction, backed by a storage provider that
    /// allows execution of contract opcodes.
    ///
    /// For predicate estimation and verification, check [`Self::init_predicate`]
    pub fn init_script(
        &mut self,
        ready_tx: Ready<Tx>,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let block_height = self.storage.block_height().map_err(RuntimeError::Storage)?;

        self.context = Context::Script { block_height };

        let (_, checked) = ready_tx.decompose();
        let (tx, metadata): (Tx, Tx::Metadata) = checked.into();

        let gas_limit = tx
            .as_script()
            .map(|script| *script.script_gas_limit())
            .unwrap_or_default();

        let initial_balances = metadata.balances();
        let runtime_balances = initial_balances.try_into()?;
        self.init_inner(tx, metadata.balances(), runtime_balances, gas_limit)?;

        if let Some(script) = self.transaction().as_script() {
            let offset = self.tx_offset().saturating_add(script.script_offset()) as Word;

            debug_assert!(offset < VM_MAX_RAM);

            self.registers[RegId::PC] = offset;
            self.registers[RegId::IS] = offset;
        }

        Ok(())
    }
}
