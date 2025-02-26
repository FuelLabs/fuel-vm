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
/// use fuel_types::{Immediate18, Word, canonical::Serialize};
/// use fuel_vm::prelude::{Call, TxParameters, ContractId, Opcode};
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
#[cfg(feature = "alloc")]
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
            let script_bytes: $crate::alloc::vec::Vec<u8> =
                ::core::iter::IntoIterator::into_iter({ $script }).collect();
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
                let value: Immediate18 = $tx_offset
                    .saturating_add(Script::script_offset_static())
                    .saturating_add(
                        padded_len(script_bytes.as_slice()).unwrap_or(usize::MAX),
                    )
                    .try_into()
                    .expect("script data offset is too large");
                value
            }
        };
        // re-evaluate and return the finalized script with the correct data offset length
        // set.
        ($script, $offset)
    }};
}

#[allow(missing_docs)]
#[cfg(feature = "random")]
#[cfg(any(test, feature = "test-helpers"))]
/// Testing utilities
pub mod test_helpers {
    use alloc::{
        vec,
        vec::Vec,
    };

    use crate::{
        checked_transaction::{
            builder::TransactionBuilderExt,
            Checked,
            IntoChecked,
        },
        interpreter::Memory,
        memory_client::MemoryClient,
        state::StateTransition,
        storage::{
            ContractsAssetsStorage,
            MemoryStorage,
        },
        transactor::Transactor,
        verification::Verifier,
    };
    use anyhow::anyhow;

    use crate::{
        interpreter::{
            CheckedMetadata,
            ExecutableTransaction,
            InterpreterParams,
            MemoryInstance,
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
        field::{
            Outputs,
            ReceiptsRoot,
        },
        BlobBody,
        BlobIdExt,
        ConsensusParameters,
        Contract,
        ContractParameters,
        Create,
        FeeParameters,
        Finalizable,
        GasCosts,
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
        canonical::{
            Deserialize,
            Serialize,
        },
        Address,
        AssetId,
        BlobId,
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
        pub rng: StdRng,
        gas_price: Word,
        max_fee_limit: Word,
        script_gas_limit: Word,
        builder: TransactionBuilder<Script>,
        storage: MemoryStorage,
        block_height: BlockHeight,
        consensus_params: ConsensusParameters,
    }

    impl TestBuilder {
        pub fn new(seed: u64) -> Self {
            let bytecode = core::iter::once(op::ret(RegId::ONE)).collect();
            TestBuilder {
                rng: StdRng::seed_from_u64(seed),
                gas_price: 0,
                max_fee_limit: 0,
                script_gas_limit: 100,
                builder: TransactionBuilder::script(bytecode, vec![]),
                storage: MemoryStorage::default(),
                block_height: Default::default(),
                consensus_params: ConsensusParameters::standard(),
            }
        }

        pub fn get_block_height(&self) -> BlockHeight {
            self.block_height
        }

        pub fn start_script_bytes(
            &mut self,
            script: Vec<u8>,
            script_data: Vec<u8>,
        ) -> &mut Self {
            self.start_script_inner(script, script_data)
        }

        pub fn start_script(
            &mut self,
            script: Vec<Instruction>,
            script_data: Vec<u8>,
        ) -> &mut Self {
            let script = script.into_iter().collect();
            self.start_script_inner(script, script_data)
        }

        fn start_script_inner(
            &mut self,
            script: Vec<u8>,
            script_data: Vec<u8>,
        ) -> &mut Self {
            self.builder = TransactionBuilder::script(script, script_data);
            self.builder.script_gas_limit(self.script_gas_limit);
            self
        }

        pub fn gas_price(&mut self, price: Word) -> &mut TestBuilder {
            self.gas_price = price;
            self
        }

        pub fn max_fee_limit(&mut self, max_fee_limit: Word) -> &mut TestBuilder {
            self.max_fee_limit = max_fee_limit;
            self
        }

        pub fn script_gas_limit(&mut self, limit: Word) -> &mut TestBuilder {
            self.builder.script_gas_limit(limit);
            self.script_gas_limit = limit;
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
                u16::try_from(input_idx.0).expect("The input index is more than allowed"),
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
                fuel_crypto::SecretKey::random(&mut self.rng),
                self.rng.gen(),
                amount,
                asset_id,
                Default::default(),
            );
            self
        }

        pub fn fee_input(&mut self) -> &mut TestBuilder {
            self.builder.add_fee_input();
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

        pub fn with_fee_params(&mut self, fee_params: FeeParameters) -> &mut TestBuilder {
            self.consensus_params.set_fee_params(fee_params);
            self
        }

        pub fn with_free_gas_costs(&mut self) -> &mut TestBuilder {
            let gas_costs = GasCosts::free();
            self.consensus_params.set_gas_costs(gas_costs);
            self
        }

        pub fn base_asset_id(&mut self, base_asset_id: AssetId) -> &mut TestBuilder {
            self.consensus_params.set_base_asset_id(base_asset_id);
            self
        }

        pub fn build(&mut self) -> Checked<Script> {
            self.builder.max_fee_limit(self.max_fee_limit);
            self.builder.with_tx_params(*self.get_tx_params());
            self.builder
                .with_contract_params(*self.get_contract_params());
            self.builder
                .with_predicate_params(*self.get_predicate_params());
            self.builder.with_script_params(*self.get_script_params());
            self.builder.with_fee_params(*self.get_fee_params());
            self.builder.with_base_asset_id(*self.get_base_asset_id());
            self.builder
                .finalize_checked_with_storage(self.block_height, &self.storage)
        }

        pub fn get_tx_params(&self) -> &TxParameters {
            self.consensus_params.tx_params()
        }

        pub fn get_predicate_params(&self) -> &PredicateParameters {
            self.consensus_params.predicate_params()
        }

        pub fn get_script_params(&self) -> &ScriptParameters {
            self.consensus_params.script_params()
        }

        pub fn get_contract_params(&self) -> &ContractParameters {
            self.consensus_params.contract_params()
        }

        pub fn get_fee_params(&self) -> &FeeParameters {
            self.consensus_params.fee_params()
        }

        pub fn get_base_asset_id(&self) -> &AssetId {
            self.consensus_params.base_asset_id()
        }

        pub fn get_block_gas_limit(&self) -> u64 {
            self.consensus_params.block_gas_limit()
        }

        pub fn get_block_transaction_size_limit(&self) -> u64 {
            self.consensus_params.block_transaction_size_limit()
        }

        pub fn get_privileged_address(&self) -> &Address {
            self.consensus_params.privileged_address()
        }

        pub fn get_chain_id(&self) -> ChainId {
            self.consensus_params.chain_id()
        }

        pub fn get_gas_costs(&self) -> &GasCosts {
            self.consensus_params.gas_costs()
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
                    op::addi(
                        0x12,
                        0x11,
                        Immediate12::try_from(AssetId::LEN)
                            .expect("`AssetId::LEN` is 32 bytes")
                    ),
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
                .script_gas_limit(1_000_000)
                .contract_input(*contract_id)
                .fee_input()
                .contract_output(contract_id)
                .build()
        }

        pub fn setup_contract_bytes(
            &mut self,
            contract: Vec<u8>,
            initial_balance: Option<(AssetId, Word)>,
            initial_state: Option<Vec<StorageSlot>>,
        ) -> CreatedContract {
            self.setup_contract_inner(contract, initial_balance, initial_state)
        }

        pub fn setup_contract(
            &mut self,
            contract: Vec<Instruction>,
            initial_balance: Option<(AssetId, Word)>,
            initial_state: Option<Vec<StorageSlot>>,
        ) -> CreatedContract {
            let contract = contract.into_iter().collect();

            self.setup_contract_inner(contract, initial_balance, initial_state)
        }

        fn setup_contract_inner(
            &mut self,
            contract: Vec<u8>,
            initial_balance: Option<(AssetId, Word)>,
            initial_state: Option<Vec<StorageSlot>>,
        ) -> CreatedContract {
            let storage_slots = initial_state.unwrap_or_default();

            let salt: Salt = self.rng.gen();
            let program: Witness = contract.into();
            let storage_root = Contract::initial_state_root(storage_slots.iter());
            let contract = Contract::from(program.as_ref());
            let contract_root = contract.root();
            let contract_id = contract.id(&salt, &contract_root, &storage_root);

            let tx = TransactionBuilder::create(program, salt, storage_slots)
                .max_fee_limit(self.max_fee_limit)
                .maturity(Default::default())
                .add_fee_input()
                .add_contract_created()
                .finalize()
                .into_checked(self.block_height, &self.consensus_params)
                .expect("failed to check tx");

            // setup a contract in current test state
            let state = self
                .deploy(tx)
                .expect("Expected vm execution to be successful");

            // set initial contract balance
            if let Some((asset_id, amount)) = initial_balance {
                self.storage
                    .contract_asset_id_balance_insert(&contract_id, &asset_id, amount)
                    .unwrap();
            }

            CreatedContract {
                tx: state.tx().clone(),
                contract_id,
                salt,
            }
        }

        pub fn setup_blob(&mut self, data: Vec<u8>) {
            let id = BlobId::compute(data.as_slice());

            let tx = TransactionBuilder::blob(BlobBody {
                id,
                witness_index: 0,
            })
            .add_witness(data.into())
            .max_fee_limit(self.max_fee_limit)
            .maturity(Default::default())
            .add_fee_input()
            .finalize()
            .into_checked(self.block_height, &self.consensus_params)
            .expect("failed to check tx");

            let interpreter_params =
                InterpreterParams::new(self.gas_price, &self.consensus_params);
            let mut transactor = Transactor::<_, _, _>::new(
                MemoryInstance::new(),
                self.storage.clone(),
                interpreter_params,
            );

            self.execute_tx_inner(&mut transactor, tx)
                .expect("Expected vm execution to be successful");
        }

        fn execute_tx_inner<M, Tx, Ecal, V>(
            &mut self,
            transactor: &mut Transactor<M, MemoryStorage, Tx, Ecal, V>,
            checked: Checked<Tx>,
        ) -> anyhow::Result<(StateTransition<Tx>, V)>
        where
            M: Memory,
            Tx: ExecutableTransaction,
            <Tx as IntoChecked>::Metadata: CheckedMetadata,
            Ecal: crate::interpreter::EcalHandler,
            V: Verifier + Clone,
        {
            self.storage.set_block_height(self.block_height);

            transactor.transact(checked);

            let storage = transactor.as_mut().clone();

            if let Some(e) = transactor.error() {
                return Err(anyhow!("{:?}", e));
            }
            let is_reverted = transactor.is_reverted();

            let state = transactor.to_owned_state_transition().unwrap();

            let interpreter = transactor.interpreter();

            let verifier = interpreter.verifier().clone();

            // verify serialized tx == referenced tx
            let transaction: Transaction = interpreter.transaction().clone().into();
            let tx_offset = self.get_tx_params().tx_offset();
            let mut tx_mem = interpreter
                .memory()
                .read(tx_offset, transaction.size())
                .unwrap();
            let mut deser_tx = Transaction::decode(&mut tx_mem).unwrap();

            // Patch the tx with correct receipts root
            if let Transaction::Script(ref mut s) = deser_tx {
                *s.receipts_root_mut() = interpreter.compute_receipts_root();
            }

            assert_eq!(deser_tx, transaction);
            if !is_reverted {
                // save storage between client instances
                self.storage = storage;
            }

            Ok((state, verifier))
        }

        pub fn deploy(
            &mut self,
            checked: Checked<Create>,
        ) -> anyhow::Result<StateTransition<Create>> {
            let interpreter_params =
                InterpreterParams::new(self.gas_price, &self.consensus_params);
            let mut transactor = Transactor::<_, _, _>::new(
                MemoryInstance::new(),
                self.storage.clone(),
                interpreter_params,
            );

            Ok(self.execute_tx_inner(&mut transactor, checked)?.0)
        }

        pub fn execute_tx(
            &mut self,
            checked: Checked<Script>,
        ) -> anyhow::Result<StateTransition<Script>> {
            let interpreter_params =
                InterpreterParams::new(self.gas_price, &self.consensus_params);
            let mut transactor = Transactor::<_, _, _>::new(
                MemoryInstance::new(),
                self.storage.clone(),
                interpreter_params,
            );

            Ok(self.execute_tx_inner(&mut transactor, checked)?.0)
        }

        pub fn execute_tx_with_backtrace(
            &mut self,
            checked: Checked<Script>,
            gas_price: u64,
        ) -> anyhow::Result<(StateTransition<Script>, Option<Backtrace>)> {
            let interpreter_params =
                InterpreterParams::new(gas_price, &self.consensus_params);
            let mut transactor = Transactor::<_, _, _>::new(
                MemoryInstance::new(),
                self.storage.clone(),
                interpreter_params,
            );

            let state = self.execute_tx_inner(&mut transactor, checked)?.0;
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
                self.consensus_params.tx_params().tx_offset(),
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

    pub fn check_expected_reason_for_instructions_with_client<M>(
        mut client: MemoryClient<M>,
        instructions: Vec<Instruction>,
        expected_reason: PanicReason,
    ) where
        M: Memory,
    {
        let tx_params = TxParameters::default().with_max_gas_per_tx(Word::MAX / 2);
        // The gas should be huge enough to cover the execution but still much less than
        // `MAX_GAS_PER_TX`.
        let gas_limit = tx_params.max_gas_per_tx() / 2;
        let maturity = Default::default();
        let height = Default::default();
        let zero_fee_limit = 0;

        // setup contract with state tests
        let contract: Witness = instructions.into_iter().collect::<Vec<u8>>().into();
        let salt = Default::default();
        let code_root = Contract::root_from_code(contract.as_ref());
        let storage_slots = vec![];
        let state_root = Contract::initial_state_root(storage_slots.iter());
        let contract_id =
            Contract::from(contract.as_ref()).id(&salt, &code_root, &state_root);

        let contract_deployer = TransactionBuilder::create(contract, salt, storage_slots)
            .max_fee_limit(zero_fee_limit)
            .with_tx_params(tx_params)
            .add_fee_input()
            .add_contract_created()
            .finalize_checked(height);

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
            .max_fee_limit(zero_fee_limit)
            .script_gas_limit(gas_limit)
            .maturity(maturity)
            .with_tx_params(tx_params)
            .add_input(Input::contract(
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                contract_id,
            ))
            .add_fee_input()
            .add_output(Output::contract(0, Default::default(), Default::default()))
            .finalize_checked(height);

        check_reason_for_transaction(client, tx_deploy_loader, expected_reason);
    }

    pub fn check_reason_for_transaction<M>(
        mut client: MemoryClient<M>,
        checked_tx: Checked<Script>,
        expected_reason: PanicReason,
    ) where
        M: Memory,
    {
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
