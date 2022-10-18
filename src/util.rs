//! FuelVM utilities

/// A utility macro for writing scripts with the data offset included. Since the
/// script data offset depends on the length of the script, this macro will
/// evaluate the length and then rewrite the resultant script output with the
/// correct offset (using the offset parameter).
///
/// # Example
///
/// ```
/// use fuel_types::{Immediate18, Word};
/// use fuel_vm::consts::{REG_ONE, REG_ZERO};
/// use fuel_vm::prelude::{Call, ConsensusParameters, ContractId, Opcode, SerializableVec};
/// use fuel_vm::script_with_data_offset;
/// use itertools::Itertools;
///
/// // Example of making a contract call script using script_data for the call info and asset id.
/// let contract_id = ContractId::from([0x11; 32]);
/// let call = Call::new(contract_id, 0, 0).to_bytes();
/// let asset_id = [0x00; 32];
/// let transfer_amount: Word = 100;
/// let gas_to_forward = 1_000_000;
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
///         Opcode::MOVI(0x10, data_offset),
///         Opcode::MOVI(0x11, transfer_amount as Immediate18),
///         // use data_offset again to reference the location of the asset id inside of script data
///         Opcode::MOVI(0x12, data_offset + call.len() as Immediate18),
///         Opcode::MOVI(0x13, gas_to_forward as Immediate18),
///         Opcode::CALL(0x10, 0x11, 0x12, 0x13),
///         Opcode::RET(REG_ONE),
///     ],
///     ConsensusParameters::DEFAULT.tx_offset(true)
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
            let script_bytes: ::std::vec::Vec<u8> = ::std::iter::IntoIterator::into_iter({ $script }).collect();
            // compute the script data offset within the VM memory given the script length
            {
                use $crate::fuel_tx::field::Script as ScriptField;
                use $crate::fuel_tx::Script;
                use $crate::fuel_types::bytes::padded_len;
                use $crate::prelude::Immediate18;
                ($tx_offset + Script::script_offset_static() + padded_len(script_bytes.as_slice())) as Immediate18
            }
        };
        // re-evaluate and return the finalized script with the correct data offset length set.
        ($script, $offset)
    }};
}

#[allow(missing_docs)]
#[cfg(any(test, feature = "test-helpers"))]
/// Testing utilities
pub mod test_helpers {
    use crate::consts::*;
    use crate::memory_client::MemoryClient;
    use crate::state::StateTransition;
    use crate::storage::{InterpreterStorage, MemoryStorage};
    use crate::transactor::Transactor;
    use anyhow::anyhow;

    use crate::interpreter::{CheckedMetadata, ExecutableTransaction};
    use fuel_asm::{Opcode, PanicReason};
    use fuel_tx::field::Outputs;
    use fuel_tx::{
        Checked, ConsensusParameters, Contract, Create, Input, IntoChecked, Output, Receipt, Script, StorageSlot,
        Transaction, TransactionBuilder, Witness,
    };
    use fuel_types::bytes::{Deserializable, SizedBytes};
    use fuel_types::{Address, AssetId, ContractId, Immediate12, Salt, Word};
    use itertools::Itertools;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};

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
        params: ConsensusParameters,
        block_height: u32,
    }

    impl TestBuilder {
        pub fn new(seed: u64) -> Self {
            TestBuilder {
                rng: StdRng::seed_from_u64(seed),
                gas_price: 0,
                gas_limit: 100,
                builder: TransactionBuilder::script(vec![Opcode::RET(REG_ONE)].into_iter().collect(), vec![]),
                storage: MemoryStorage::default(),
                params: ConsensusParameters::default(),
                block_height: 0,
            }
        }

        pub fn start_script(&mut self, script: Vec<Opcode>, script_data: Vec<u8>) -> &mut Self {
            self.builder = TransactionBuilder::script(script.into_iter().collect(), script_data);
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
            self.builder.add_output(Output::change(self.rng.gen(), 0, asset_id));
            self
        }

        pub fn coin_output(&mut self, asset_id: AssetId, amount: Word) -> &mut TestBuilder {
            self.builder.add_output(Output::coin(self.rng.gen(), amount, asset_id));
            self
        }

        pub fn message_output(&mut self) -> &mut TestBuilder {
            self.builder.add_output(Output::message(Address::zeroed(), 0));
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
                .find_position(|input| matches!(input, Input::Contract {contract_id, ..} if contract_id == id))
                .expect("expected contract input with matching contract id");

            self.builder
                .add_output(Output::contract(input_idx.0 as u8, self.rng.gen(), self.rng.gen()));

            self
        }

        pub fn coin_input(&mut self, asset_id: AssetId, amount: Word) -> &mut TestBuilder {
            self.builder
                .add_unsigned_coin_input(self.rng.gen(), self.rng.gen(), amount, asset_id, self.rng.gen(), 0);
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

        pub fn params(&mut self, params: ConsensusParameters) -> &mut TestBuilder {
            self.params = params;
            self
        }

        pub fn block_height(&mut self, block_height: u32) -> &mut TestBuilder {
            self.block_height = block_height;
            self
        }

        pub const fn tx_offset(&self) -> usize {
            self.params.tx_offset(true)
        }

        pub fn build(&mut self) -> Checked<Script> {
            self.builder.finalize_checked(self.block_height as Word, &self.params)
        }

        pub fn build_get_balance_tx(
            params: &ConsensusParameters,
            contract_id: &ContractId,
            asset_id: &AssetId,
        ) -> Checked<Script> {
            let (script, _) = script_with_data_offset!(
                data_offset,
                vec![
                    Opcode::MOVI(0x11, data_offset),
                    Opcode::ADDI(0x12, 0x11, AssetId::LEN as Immediate12),
                    Opcode::BAL(0x10, 0x11, 0x12),
                    Opcode::LOG(0x10, REG_ZERO, REG_ZERO, REG_ZERO),
                    Opcode::RET(REG_ONE)
                ],
                params.tx_offset(true)
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
                .contract_output(contract_id)
                .build()
        }

        pub fn setup_contract(
            &mut self,
            contract: Vec<Opcode>,
            initial_balance: Option<(AssetId, Word)>,
            initial_state: Option<Vec<StorageSlot>>,
        ) -> CreatedContract {
            let storage_slots = if let Some(slots) = initial_state {
                slots
            } else {
                Default::default()
            };

            let salt: Salt = self.rng.gen();
            let program: Witness = contract.iter().copied().collect::<Vec<u8>>().into();
            let storage_root = Contract::initial_state_root(storage_slots.iter());
            let contract = Contract::from(program.as_ref());
            let contract_root = contract.root();
            let contract_id = contract.id(&salt, &contract_root, &storage_root);

            let tx = Transaction::create(
                self.gas_price,
                self.gas_limit,
                0,
                0,
                salt,
                storage_slots,
                vec![],
                vec![Output::contract_created(contract_id, storage_root)],
                vec![program],
            )
            .into_checked(self.block_height as Word, &self.params)
            .expect("failed to check tx");

            // setup a contract in current test state
            let state = self.execute_tx(tx).expect("Expected vm execution to be successful");

            // set initial contract balance
            if let Some((asset_id, amount)) = initial_balance {
                self.storage
                    .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, amount)
                    .unwrap();
            }

            CreatedContract {
                tx: state.tx().clone(),
                contract_id,
                salt,
            }
        }

        pub fn execute_tx<Tx>(&mut self, checked: Checked<Tx>) -> anyhow::Result<StateTransition<Tx>>
        where
            Tx: ExecutableTransaction,
            <Tx as IntoChecked>::Metadata: CheckedMetadata,
        {
            self.storage.set_block_height(self.block_height);
            let mut transactor = Transactor::new(self.storage.clone(), self.params);

            transactor.transact(checked);

            let storage = transactor.as_mut().clone();

            if let Some(e) = transactor.error() {
                return Err(anyhow!("{:?}", e));
            }

            let state = transactor.to_owned_state_transition().unwrap();

            let interpreter = transactor.interpreter();

            // verify serialized tx == referenced tx
            let transaction: Transaction = interpreter.transaction().clone().into();
            let tx_offset = self.params.tx_offset(false);
            let tx_mem = &interpreter.memory()[tx_offset..(tx_offset + transaction.serialized_size())];
            let deser_tx = Transaction::from_bytes(tx_mem).unwrap();

            assert_eq!(deser_tx, transaction);

            // save storage between client instances
            self.storage = storage;

            Ok(state)
        }

        /// Build test tx and execute it
        pub fn execute(&mut self) -> StateTransition<Script> {
            let tx = self.build();

            self.execute_tx(tx).expect("expected successful vm execution")
        }

        pub fn execute_get_outputs(&mut self) -> Vec<Output> {
            self.execute().tx().outputs().to_vec()
        }

        pub fn execute_get_change(&mut self, find_asset_id: AssetId) -> Word {
            let outputs = self.execute_get_outputs();
            let change = outputs.into_iter().find_map(|output| {
                if let Output::Change { amount, asset_id, .. } = output {
                    if &asset_id == &find_asset_id {
                        Some(amount)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            change.expect(format!("no change matching asset ID {:x} was found", &find_asset_id).as_str())
        }

        pub fn get_contract_balance(&mut self, contract_id: &ContractId, asset_id: &AssetId) -> Word {
            let tx = TestBuilder::build_get_balance_tx(&self.params, contract_id, asset_id);
            let state = self
                .execute_tx(tx)
                .expect("expected successful vm execution in this context");
            let receipts = state.receipts();
            receipts[0].ra().expect("Balance expected")
        }
    }

    pub fn check_expected_reason_for_opcodes(opcodes: Vec<Opcode>, expected_reason: PanicReason) {
        let client = MemoryClient::default();

        check_expected_reason_for_opcodes_with_client(client, opcodes, expected_reason);
    }

    fn check_expected_reason_for_opcodes_with_client(
        client: MemoryClient,
        opcodes: Vec<Opcode>,
        expected_reason: PanicReason,
    ) {
        let gas_price = 0;
        let params = ConsensusParameters::default().with_max_gas_per_tx(Word::MAX / 2);
        let gas_limit = params.max_gas_per_tx;
        let maturity = 0;
        let height = 0;

        let tx_deploy_loader = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            opcodes.into_iter().collect(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .into_checked(height, &params)
        .expect("failed to check tx");

        check_reason_for_transaction(client, tx_deploy_loader, expected_reason);
    }

    pub fn check_reason_for_transaction(
        mut client: MemoryClient,
        checked_tx: Checked<Script>,
        expected_reason: PanicReason,
    ) {
        let receipts = client.transact(checked_tx);

        if let Receipt::Panic { id: _, reason, .. } = receipts.get(0).expect("No receipt") {
            assert_eq!(
                &expected_reason,
                reason.reason(),
                "Expected {}, found {}",
                expected_reason,
                reason.reason()
            );
        } else {
            panic!("Script should have panicked");
        }
    }
}

#[allow(missing_docs)]
#[cfg(all(feature = "profile-gas", any(test, feature = "test-helpers")))]
/// Gas testing utilities
pub mod gas_profiling {
    use crate::prelude::*;

    use std::sync::{Arc, Mutex};

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
        fn on_transaction(&mut self, _state: &Result<ProgramState, InterpreterError>, data: &ProfilingData) {
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
