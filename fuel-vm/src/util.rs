//! FuelVM utilities

/// A utility macro for writing scripts with the data offset included. Since the
/// script data offset depends on the length of the script, this macro will
/// evaluate the length and then rewrite the resultant script output with the
/// correct offset (using the offset parameter).
///
/// # Example
///
/// ```
/// use fuel_asm::{op, RegId};
/// use fuel_types::{Immediate18, Word};
/// use fuel_vm::prelude::{Call, TxParameters, ContractId, Opcode, SerializableVec};
/// use fuel_vm::script_with_data_offset;
/// use itertools::Itertools;
///
/// // Example of making a contract call script using script_data for the call info and asset id.
/// let contract_id = ContractId::from([0x11; 32]);
/// let call = Call::new(contract_id, 0, 0).to_bytes();
/// let asset_id = [0x00; 32];
/// let transfer_amount: Word = 100;
/// let gas_to_forward = 100_000;
/// let script_data = [call.as_ref(), asset_id.as_ref()]
///     .into_iter()
///     .flatten()
///     .copied()
///     .collect_vec();
///
/// // Use the macro since we don't know the exact offset for script_data.
/// let (script, data_offset) = script_with_data_offset!(
///     data_offset,
///     vec![
///         // use data_offset to reference the location of the call bytes inside script_data
///         op::movi(0x10, data_offset),
///         op::movi(0x11, transfer_amount as Immediate18),
///         // use data_offset again to reference the location of the asset id inside of script data
///         op::movi(0x12, data_offset + call.len() as Immediate18),
///         op::movi(0x13, gas_to_forward as Immediate18),
///         op::call(0x10, 0x11, 0x12, 0x13),
///         op::ret(RegId::ONE),
///     ],
///     TxParameters::DEFAULT.tx_offset()
/// );
/// ```
#[macro_export]
macro_rules! script_with_data_offset {
    ($offset:ident, $script:expr, $tx_offset:expr) => {{
        let $offset = {
            // first set offset to 0 before evaluating script expression
            let $offset = {
                use $crate::prelude::Immediate18;
                0 as Immediate18
            };
            // evaluate script expression with zeroed data offset to get the script length
            let script_bytes: ::std::vec::Vec<u8> =
                ::std::iter::IntoIterator::into_iter({ $script }).collect();
            // compute the script data offset within the VM memory given the script length
            {
                use $crate::{
                    fuel_tx::{
                        field::Script as ScriptField,
                        Script,
                    },
                    fuel_types::bytes::padded_len,
                    prelude::Immediate18,
                };
                ($tx_offset
                    + Script::script_offset_static()
                    + padded_len(script_bytes.as_slice())) as Immediate18
            }
        };
        // re-evaluate and return the finalized script with the correct data offset length
        // set.
        ($script, $offset)
    }};
}

#[allow(missing_docs)]
#[cfg(any(test, feature = "test-helpers"))]
/// Testing utilities
pub mod test_helpers {
    use crate::{
        checked_transaction::{
            builder::TransactionBuilderExt,
            Checked,
            IntoChecked,
        },
        gas::GasCosts,
        memory_client::MemoryClient,
        state::StateTransition,
        storage::{
            ContractsAssetsStorage,
            MemoryStorage,
        },
        transactor::Transactor,
    };
    use anyhow::anyhow;

    use crate::{
        checked_transaction::ConsensusParams,
        interpreter::{
            CheckedMetadata,
            ExecutableTransaction,
            InterpreterParams,
        },
        prelude::{
            Backtrace,
            Call,
        },
    };
    use fuel_asm::{
        op,
        GTFArgs,
        Instruction,
        PanicReason,
        RegId,
    };
    use fuel_tx::{
        field::Outputs,
        Contract,
        ContractParameters,
        Create,
        FeeParameters,
        Finalizable,
        Input,
        Output,
        PredicateParameters,
        Receipt,
        Script,
        ScriptParameters,
        StorageSlot,
        Transaction,
        TransactionBuilder,
        TxParameters,
        Witness,
    };
    use fuel_types::{
        bytes::{
            Deserializable,
            SerializableVec,
            SizedBytes,
        },
        Address,
        AssetId,
        BlockHeight,
        ChainId,
        ContractId,
        Immediate12,
        Salt,
        Word,
    };
    use itertools::Itertools;
    use rand::{
        prelude::StdRng,
        Rng,
        SeedableRng,
    };

    pub struct CreatedContract {
        pub tx: Create,
        pub contract_id: ContractId,
        pub salt: Salt,
    }

    pub struct TestBuilder {
        rng: StdRng,
        gas_price: Word,
        gas_limit: Word,
        builder: TransactionBuilder<Script>,
        storage: MemoryStorage,
        gas_costs: GasCosts,
        block_height: BlockHeight,
        tx_params: TxParameters,
        predicate_params: PredicateParameters,
        script_params: ScriptParameters,
        contract_params: ContractParameters,
        fee_params: FeeParameters,
        chain_id: ChainId,
    }

    impl TestBuilder {
        pub fn new(seed: u64) -> Self {
            let bytecode = core::iter::once(op::ret(RegId::ONE)).collect();
            TestBuilder {
                rng: StdRng::seed_from_u64(seed),
                gas_price: 0,
                gas_limit: 100,
                builder: TransactionBuilder::script(bytecode, vec![]),
                storage: MemoryStorage::default(),
                gas_costs: Default::default(),
                block_height: Default::default(),
                tx_params: Default::default(),
                predicate_params: Default::default(),
                script_params: Default::default(),
                contract_params: Default::default(),
                fee_params: Default::default(),
                chain_id: ChainId::new(0),
            }
        }

        pub fn get_block_height(&self) -> BlockHeight {
            self.block_height
        }

        pub fn start_script(
            &mut self,
            script: Vec<Instruction>,
            script_data: Vec<u8>,
        ) -> &mut Self {
            let bytecode = script.into_iter().collect();
            self.builder = TransactionBuilder::script(bytecode, script_data);
            self.builder.gas_price(self.gas_price);
            self.builder.gas_limit(self.gas_limit);
            self
        }

        pub fn gas_price(&mut self, price: Word) -> &mut TestBuilder {
            self.builder.gas_price(price);
            self.gas_price = price;
            self
        }

        pub fn gas_limit(&mut self, limit: Word) -> &mut TestBuilder {
            self.builder.gas_limit(limit);
            self.gas_limit = limit;
            self
        }

        pub fn change_output(&mut self, asset_id: AssetId) -> &mut TestBuilder {
            self.builder
                .add_output(Output::change(self.rng.gen(), 0, asset_id));
            self
        }

        pub fn coin_output(
            &mut self,
            asset_id: AssetId,
            amount: Word,
        ) -> &mut TestBuilder {
            self.builder
                .add_output(Output::coin(self.rng.gen(), amount, asset_id));
            self
        }

        pub fn variable_output(&mut self, asset_id: AssetId) -> &mut TestBuilder {
            self.builder
                .add_output(Output::variable(Address::zeroed(), 0, asset_id));
            self
        }

        pub fn contract_output(&mut self, id: &ContractId) -> &mut TestBuilder {
            let input_idx = self
                .builder
                .inputs()
                .iter()
                .find_position(|input| matches!(input, Input::Contract(contract) if &contract.contract_id == id))
                .expect("expected contract input with matching contract id");

            self.builder.add_output(Output::contract(
                input_idx.0 as u8,
                self.rng.gen(),
                self.rng.gen(),
            ));

            self
        }

        pub fn coin_input(
            &mut self,
            asset_id: AssetId,
            amount: Word,
        ) -> &mut TestBuilder {
            self.builder.add_unsigned_coin_input(
                self.rng.gen(),
                self.rng.gen(),
                amount,
                asset_id,
                self.rng.gen(),
                Default::default(),
            );
            self
        }

        pub fn fee_input(&mut self) -> &mut TestBuilder {
            self.builder.add_random_fee_input();
            self
        }

        pub fn contract_input(&mut self, contract_id: ContractId) -> &mut TestBuilder {
            self.builder.add_input(Input::contract(
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                contract_id,
            ));
            self
        }

        pub fn witness(&mut self, witness: Witness) -> &mut TestBuilder {
            self.builder.add_witness(witness);
            self
        }

        pub fn storage(&mut self, storage: MemoryStorage) -> &mut TestBuilder {
            self.storage = storage;
            self
        }

        pub fn block_height(&mut self, block_height: BlockHeight) -> &mut TestBuilder {
            self.block_height = block_height;
            self
        }

        pub fn build(&mut self) -> Checked<Script> {
            self.builder.with_tx_params(self.tx_params);
            self.builder.with_predicate_params(self.predicate_params);
            self.builder.with_script_params(self.script_params);
            self.builder.with_contract_params(self.contract_params);
            self.builder.with_fee_params(self.fee_params);
            self.builder
                .finalize_checked(self.block_height, self.gas_costs.clone())
        }

        pub fn get_tx_params(&self) -> &TxParameters {
            &self.tx_params
        }

        pub fn get_predicate_params(&self) -> &PredicateParameters {
            &self.predicate_params
        }

        pub fn get_script_params(&self) -> &ScriptParameters {
            &self.script_params
        }

        pub fn get_contract_params(&self) -> &ContractParameters {
            &self.contract_params
        }

        pub fn get_fee_params(&self) -> &FeeParameters {
            &self.fee_params
        }

        pub fn get_chain_id(&self) -> ChainId {
            self.chain_id
        }

        pub fn get_gas_costs(&self) -> &GasCosts {
            &self.gas_costs
        }

        pub fn with_fee_params(&mut self, fee_params: FeeParameters) -> &mut TestBuilder {
            self.fee_params = fee_params;
            self
        }

        pub fn build_get_balance_tx(
            contract_id: &ContractId,
            asset_id: &AssetId,
            tx_offset: usize,
        ) -> Checked<Script> {
            let (script, _) = script_with_data_offset!(
                data_offset,
                vec![
                    op::movi(0x11, data_offset),
                    op::addi(0x12, 0x11, AssetId::LEN as Immediate12),
                    op::bal(0x10, 0x11, 0x12),
                    op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
                    op::ret(RegId::ONE),
                ],
                tx_offset
            );

            let script_data: Vec<u8> = [asset_id.as_ref(), contract_id.as_ref()]
                .into_iter()
                .flatten()
                .copied()
                .collect();

            TestBuilder::new(2322u64)
                .start_script(script, script_data)
                .gas_price(0)
                .gas_limit(1_000_000)
                .contract_input(*contract_id)
                .fee_input()
                .contract_output(contract_id)
                .build()
        }

        pub fn setup_contract(
            &mut self,
            contract: Vec<Instruction>,
            initial_balance: Option<(AssetId, Word)>,
            initial_state: Option<Vec<StorageSlot>>,
        ) -> CreatedContract {
            let storage_slots = if let Some(slots) = initial_state {
                slots
            } else {
                Default::default()
            };

            let salt: Salt = self.rng.gen();
            let program: Witness = contract
                .into_iter()
                .flat_map(Instruction::to_bytes)
                .collect::<Vec<u8>>()
                .into();
            let storage_root = Contract::initial_state_root(storage_slots.iter());
            let contract = Contract::from(program.as_ref());
            let contract_root = contract.root();
            let contract_id = contract.id(&salt, &contract_root, &storage_root);

            let consensus_params = ConsensusParams::new(
                &self.tx_params,
                &self.predicate_params,
                &self.script_params,
                &self.contract_params,
                &self.fee_params,
            );

            let tx = TransactionBuilder::create(program, salt, storage_slots)
                .gas_price(self.gas_price)
                .gas_limit(self.gas_limit)
                .maturity(Default::default())
                .add_random_fee_input()
                .add_output(Output::contract_created(contract_id, storage_root))
                .finalize()
                .into_checked(
                    self.block_height,
                    consensus_params,
                    self.chain_id,
                    self.gas_costs.clone(),
                )
                .expect("failed to check tx");

            // setup a contract in current test state
            let state = self
                .deploy(tx)
                .expect("Expected vm execution to be successful");

            // set initial contract balance
            if let Some((asset_id, amount)) = initial_balance {
                self.storage
                    .merkle_contract_asset_id_balance_insert(
                        &contract_id,
                        &asset_id,
                        amount,
                    )
                    .unwrap();
            }

            CreatedContract {
                tx: state.tx().clone(),
                contract_id,
                salt,
            }
        }

        fn execute_tx_inner<Tx>(
            &mut self,
            transactor: &mut Transactor<MemoryStorage, Tx>,
            checked: Checked<Tx>,
        ) -> anyhow::Result<StateTransition<Tx>>
        where
            Tx: ExecutableTransaction,
            <Tx as IntoChecked>::Metadata: CheckedMetadata,
        {
            self.storage.set_block_height(self.block_height);

            transactor.transact(checked);

            let storage = transactor.as_mut().clone();

            if let Some(e) = transactor.error() {
                return Err(anyhow!("{:?}", e))
            }
            let is_reverted = transactor.is_reverted();

            let state = transactor.to_owned_state_transition().unwrap();

            let interpreter = transactor.interpreter();

            // verify serialized tx == referenced tx
            let transaction: Transaction = interpreter.transaction().clone().into();
            let tx_offset = self.tx_params.tx_offset();
            let tx_mem = &interpreter.memory()
                [tx_offset..(tx_offset + transaction.serialized_size())];
            let deser_tx = Transaction::from_bytes(tx_mem).unwrap();

            assert_eq!(deser_tx, transaction);
            if is_reverted {
                return Ok(state)
            }

            // save storage between client instances
            self.storage = storage;

            Ok(state)
        }

        pub fn deploy(
            &mut self,
            checked: Checked<Create>,
        ) -> anyhow::Result<StateTransition<Create>> {
            let interpreter_params = InterpreterParams {
                gas_costs: self.gas_costs.clone(),
                max_inputs: self.tx_params.max_inputs,
                contract_max_size: self.contract_params.contract_max_size,
                tx_offset: self.tx_params.tx_offset(),
                max_message_data_length: self.predicate_params.max_message_data_length,
                chain_id: self.chain_id,
                fee_params: self.fee_params,
            };
            let mut transactor =
                Transactor::new(self.storage.clone(), interpreter_params);

            self.execute_tx_inner(&mut transactor, checked)
        }

        pub fn execute_tx(
            &mut self,
            checked: Checked<Script>,
        ) -> anyhow::Result<StateTransition<Script>> {
            let interpreter_params = InterpreterParams {
                gas_costs: self.gas_costs.clone(),
                max_inputs: self.tx_params.max_inputs,
                contract_max_size: self.contract_params.contract_max_size,
                tx_offset: self.tx_params.tx_offset(),
                max_message_data_length: self.predicate_params.max_message_data_length,
                chain_id: self.chain_id,
                fee_params: self.fee_params,
            };
            let mut transactor =
                Transactor::new(self.storage.clone(), interpreter_params);

            self.execute_tx_inner(&mut transactor, checked)
        }

        pub fn execute_tx_with_backtrace(
            &mut self,
            checked: Checked<Script>,
        ) -> anyhow::Result<(StateTransition<Script>, Option<Backtrace>)> {
            let interpreter_params = InterpreterParams {
                gas_costs: self.gas_costs.clone(),
                max_inputs: self.tx_params.max_inputs,
                contract_max_size: self.contract_params.contract_max_size,
                tx_offset: self.tx_params.tx_offset(),
                max_message_data_length: self.predicate_params.max_message_data_length,
                chain_id: self.chain_id,
                fee_params: self.fee_params,
            };
            let mut transactor =
                Transactor::new(self.storage.clone(), interpreter_params);

            let state = self.execute_tx_inner(&mut transactor, checked)?;
            let backtrace = transactor.backtrace();

            Ok((state, backtrace))
        }

        /// Build test tx and execute it
        pub fn execute(&mut self) -> StateTransition<Script> {
            let tx = self.build();

            self.execute_tx(tx)
                .expect("expected successful vm execution")
        }

        pub fn get_storage(&self) -> &MemoryStorage {
            &self.storage
        }

        pub fn execute_get_outputs(&mut self) -> Vec<Output> {
            self.execute().tx().outputs().to_vec()
        }

        pub fn execute_get_change(&mut self, find_asset_id: AssetId) -> Word {
            let outputs = self.execute_get_outputs();
            find_change(outputs, find_asset_id)
        }

        pub fn get_contract_balance(
            &mut self,
            contract_id: &ContractId,
            asset_id: &AssetId,
        ) -> Word {
            let tx = TestBuilder::build_get_balance_tx(
                contract_id,
                asset_id,
                self.tx_params.tx_offset(),
            );
            let state = self
                .execute_tx(tx)
                .expect("expected successful vm execution in this context");
            let receipts = state.receipts();
            receipts[0].ra().expect("Balance expected")
        }
    }

    pub fn check_expected_reason_for_instructions(
        instructions: Vec<Instruction>,
        expected_reason: PanicReason,
    ) {
        let client = MemoryClient::default();

        check_expected_reason_for_instructions_with_client(
            client,
            instructions,
            expected_reason,
        );
    }

    fn check_expected_reason_for_instructions_with_client(
        mut client: MemoryClient,
        instructions: Vec<Instruction>,
        expected_reason: PanicReason,
    ) {
        let gas_price = 0;
        let tx_params = TxParameters::default().with_max_gas_per_tx(Word::MAX / 2);
        let gas_limit = tx_params.max_gas_per_tx;
        let maturity = Default::default();
        let height = Default::default();

        // setup contract with state tests
        let contract: Witness = instructions.into_iter().collect::<Vec<u8>>().into();
        let salt = Default::default();
        let code_root = Contract::root_from_code(contract.as_ref());
        let storage_slots = vec![];
        let state_root = Contract::initial_state_root(storage_slots.iter());
        let contract_id =
            Contract::from(contract.as_ref()).id(&salt, &code_root, &state_root);

        let contract_deployer = TransactionBuilder::create(contract, salt, storage_slots)
            .with_tx_params(tx_params)
            .add_output(Output::contract_created(contract_id, state_root))
            .add_random_fee_input()
            .finalize_checked(height, client.gas_costs().to_owned());

        client
            .deploy(contract_deployer)
            .expect("valid contract deployment");

        // call deployed contract
        let script = [
            // load call data to 0x10
            op::gtf(0x10, 0x0, Immediate12::from(GTFArgs::ScriptData)),
            // call the transfer contract
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();
        let script_data: Vec<u8> = [Call::new(contract_id, 0, 0).to_bytes().as_slice()]
            .into_iter()
            .flatten()
            .copied()
            .collect();

        let tx_deploy_loader = TransactionBuilder::script(script, script_data)
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .maturity(maturity)
            .with_tx_params(tx_params)
            .add_input(Input::contract(
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                contract_id,
            ))
            .add_random_fee_input()
            .add_output(Output::contract(0, Default::default(), Default::default()))
            .finalize_checked(height, client.gas_costs().to_owned());

        check_reason_for_transaction(client, tx_deploy_loader, expected_reason);
    }

    pub fn check_reason_for_transaction(
        mut client: MemoryClient,
        checked_tx: Checked<Script>,
        expected_reason: PanicReason,
    ) {
        let receipts = client.transact(checked_tx);

        let panic_found = receipts.iter().any(|receipt| {
            if let Receipt::Panic { id: _, reason, .. } = receipt {
                assert_eq!(
                    &expected_reason,
                    reason.reason(),
                    "Expected {}, found {}",
                    expected_reason,
                    reason.reason()
                );
                true
            } else {
                false
            }
        });

        if !panic_found {
            panic!("Script should have panicked");
        }
    }

    pub fn find_change(outputs: Vec<Output>, find_asset_id: AssetId) -> Word {
        let change = outputs.into_iter().find_map(|output| {
            if let Output::Change {
                amount, asset_id, ..
            } = output
            {
                if asset_id == find_asset_id {
                    Some(amount)
                } else {
                    None
                }
            } else {
                None
            }
        });
        change.unwrap_or_else(|| {
            panic!("no change matching asset ID {:x} was found", &find_asset_id)
        })
    }
}

#[allow(missing_docs)]
#[cfg(all(feature = "profile-gas", any(test, feature = "test-helpers")))]
/// Gas testing utilities
pub mod gas_profiling {
    use crate::prelude::*;

    use std::sync::{
        Arc,
        Mutex,
    };

    #[derive(Clone)]
    pub struct GasProfiler {
        data: Arc<Mutex<Option<ProfilingData>>>,
    }

    impl Default for GasProfiler {
        fn default() -> Self {
            Self {
                data: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl ProfileReceiver for GasProfiler {
        fn on_transaction(
            &mut self,
            _state: &Result<ProgramState, InterpreterError>,
            data: &ProfilingData,
        ) {
            let mut guard = self.data.lock().unwrap();
            *guard = Some(data.clone());
        }
    }

    impl GasProfiler {
        pub fn data(&self) -> Option<ProfilingData> {
            self.data.lock().ok().and_then(|g| g.as_ref().cloned())
        }

        pub fn total_gas(&self) -> Word {
            self.data()
                .map(|d| d.gas().iter().map(|(_, gas)| gas).sum())
                .unwrap_or_default()
        }
    }
}
