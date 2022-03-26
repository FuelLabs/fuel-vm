//! FuelVM utilities

/// A utility macro for writing scripts with the data offset included. Since the
/// script data offset depends on the length of the script, this macro will
/// evaluate the length and then rewrite the resultant script output with the
/// correct offset (using the offset parameter).
///
/// # Example
///
/// ```
/// use itertools::Itertools;
/// use fuel_types::{Word, Immediate18};
/// use fuel_vm::consts::{REG_ONE, REG_ZERO};
/// use fuel_vm::prelude::{Opcode, Call, SerializableVec, ContractId};
/// use fuel_vm::script_with_data_offset;
///
/// // Example of making a contract call script using script_data for the call info and asset id.
/// let contract_id = ContractId::from([0x11; 32]);
/// let call = Call::new(contract_id, 0, 0).to_bytes();
/// let asset_id = [0x00; 32];
/// let transfer_amount: Word = 100;
/// let gas_to_forward = 1_000_000;
/// let script_data = [call.as_ref(), asset_id.as_ref()]
///                     .into_iter()
///                     .flatten()
///                     .copied()
///                     .collect_vec();
///
/// // Use the macro since we don't know the exact offset for script_data.
/// let (script, data_offset) = script_with_data_offset!(data_offset, vec![
///     // use data_offset to reference the location of the call bytes inside script_data
///     Opcode::MOVI(0x10, data_offset),
///     Opcode::MOVI(0x11, transfer_amount as Immediate18),
///     // use data_offset again to reference the location of the asset id inside of script data
///     Opcode::MOVI(0x12, data_offset + call.len() as Immediate18),
///     Opcode::MOVI(0x13,  gas_to_forward as Immediate18),
///     Opcode::CALL(0x10, 0x11, 0x12, 0x13),
///     Opcode::RET(REG_ONE),
/// ]);
/// ```
#[macro_export]
macro_rules! script_with_data_offset {
    ($offset:ident, $script:expr) => {{
        use fuel_types::{bytes, Immediate18};
        use $crate::consts::VM_TX_MEMORY;
        use $crate::prelude::Transaction;
        let $offset = 0 as Immediate18;
        let script_bytes: Vec<u8> = { $script }.into_iter().collect();
        let data_offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script_bytes.as_slice());
        let $offset = data_offset as Immediate18;
        ($script, $offset)
    }};
}

#[allow(missing_docs)]
#[cfg(any(test, feature = "test-helpers"))]
/// Testing utilities
pub mod test_helpers {
    use crate::consts::{REG_ONE, REG_ZERO, VM_TX_MEMORY};
    use crate::prelude::{InterpreterStorage, MemoryClient, MemoryStorage, Transactor};
    use crate::state::StateTransition;
    use fuel_asm::Opcode;
    use fuel_tx::{Input, Output, StorageSlot, Transaction, Witness};
    use fuel_types::bytes::{Deserializable, SizedBytes};
    use fuel_types::{AssetId, ContractId, Immediate12, Salt, Word};
    use itertools::Itertools;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};

    pub struct CreatedContract {
        pub tx: Transaction,
        pub contract_id: ContractId,
        pub salt: Salt,
    }

    pub struct TestBuilder {
        rng: StdRng,
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        script: Vec<Opcode>,
        script_data: Vec<u8>,
        witness: Vec<Witness>,
        storage: MemoryStorage,
    }

    impl TestBuilder {
        pub fn new(seed: u64) -> Self {
            TestBuilder {
                rng: StdRng::seed_from_u64(seed),
                gas_price: 0,
                gas_limit: 100,
                byte_price: 0,
                inputs: Default::default(),
                outputs: Default::default(),
                script: vec![Opcode::RET(REG_ONE)],
                script_data: vec![],
                witness: vec![Witness::default()],
                storage: MemoryStorage::default(),
            }
        }

        pub fn gas_price(&mut self, price: Word) -> &mut TestBuilder {
            self.gas_price = price;
            self
        }

        pub fn gas_limit(&mut self, limit: Word) -> &mut TestBuilder {
            self.gas_limit = limit;
            self
        }

        pub fn byte_price(&mut self, price: Word) -> &mut TestBuilder {
            self.byte_price = price;
            self
        }

        pub fn change_output(&mut self, asset_id: AssetId) -> &mut TestBuilder {
            self.outputs.push(Output::change(self.rng.gen(), 0, asset_id));
            self
        }

        pub fn coin_output(&mut self, asset_id: AssetId, amount: Word) -> &mut TestBuilder {
            self.outputs.push(Output::coin(self.rng.gen(), amount, asset_id));
            self
        }

        pub fn withdrawal_output(&mut self, asset_id: AssetId, amount: Word) -> &mut TestBuilder {
            self.outputs.push(Output::withdrawal(self.rng.gen(), amount, asset_id));
            self
        }

        pub fn variable_output(&mut self, asset_id: AssetId) -> &mut TestBuilder {
            self.outputs.push(Output::variable(self.rng.gen(), 0, asset_id));
            self
        }

        pub fn contract_output(&mut self, id: &ContractId) -> &mut TestBuilder {
            let input_idx = self
                .inputs
                .iter()
                .find_position(|input| matches!(input, Input::Contract {contract_id, ..} if contract_id == id))
                .expect("expected contract input with matching contract id");
            self.outputs
                .push(Output::contract(input_idx.0 as u8, self.rng.gen(), self.rng.gen()));
            self
        }

        pub fn coin_input(&mut self, asset_id: AssetId, amount: Word) -> &mut TestBuilder {
            self.inputs.push(Input::coin(
                self.rng.gen(),
                self.rng.gen(),
                amount,
                asset_id,
                0,
                0,
                vec![],
                vec![],
            ));
            self
        }

        pub fn contract_input(&mut self, contract_id: ContractId) -> &mut TestBuilder {
            self.inputs.push(Input::contract(
                self.rng.gen(),
                self.rng.gen(),
                self.rng.gen(),
                contract_id,
            ));
            self
        }

        pub fn script(&mut self, script: Vec<Opcode>) -> &mut TestBuilder {
            self.script = script;
            self
        }

        pub fn script_data(&mut self, script_data: Vec<u8>) -> &mut TestBuilder {
            self.script_data = script_data;
            self
        }

        pub fn witness(&mut self, witness: Vec<Witness>) -> &mut TestBuilder {
            self.witness = witness;
            self
        }

        pub fn storage(&mut self, storage: MemoryStorage) -> &mut TestBuilder {
            self.storage = storage;
            self
        }

        pub fn build(&mut self) -> Transaction {
            Transaction::script(
                self.gas_price,
                self.gas_limit,
                self.byte_price,
                0,
                self.script.iter().copied().collect(),
                self.script_data.clone(),
                self.inputs.clone(),
                self.outputs.clone(),
                self.witness.clone(),
            )
        }

        pub fn build_get_balance_tx(contract_id: &ContractId, asset_id: &AssetId) -> Transaction {
            let (script, _) = script_with_data_offset!(
                data_offset,
                vec![
                    Opcode::MOVI(0x11, data_offset),
                    Opcode::ADDI(0x12, 0x11, AssetId::LEN as Immediate12),
                    Opcode::BAL(0x10, 0x11, 0x12),
                    Opcode::LOG(0x10, REG_ZERO, REG_ZERO, REG_ZERO),
                    Opcode::RET(REG_ONE)
                ]
            );

            let script_data: Vec<u8> = [asset_id.as_ref(), contract_id.as_ref()]
                .into_iter()
                .flatten()
                .copied()
                .collect();

            TestBuilder::new(2322u64)
                .gas_price(0)
                .byte_price(0)
                .gas_limit(1_000_000)
                .script(script)
                .script_data(script_data)
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
            let storage_root = crate::contract::Contract::initial_state_root(&storage_slots);
            let contract = crate::contract::Contract::from(program.as_ref());
            let contract_root = contract.root();
            let contract_id = contract.id(&salt, &contract_root, &storage_root);

            let tx = Transaction::create(
                self.gas_price,
                self.gas_limit,
                self.byte_price,
                0,
                0,
                salt,
                vec![],
                storage_slots,
                vec![],
                vec![Output::contract_created(contract_id, storage_root)],
                vec![program],
            );

            // setup a contract in current test state
            let state = self.execute_tx(tx);

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

        fn execute_tx(&mut self, tx: Transaction) -> StateTransition {
            let mut client = MemoryClient::new(self.storage.clone());
            client.transact(tx);
            let storage = client.as_ref().clone();
            let txtor: Transactor<_> = client.into();
            let state = txtor.state_transition().unwrap().into_owned();
            let interpreter = txtor.interpreter();
            // verify serialized tx == referenced tx
            let tx_mem =
                &interpreter.memory()[VM_TX_MEMORY..(VM_TX_MEMORY + interpreter.transaction().serialized_size())];
            let deser_tx = Transaction::from_bytes(tx_mem).unwrap();
            assert_eq!(deser_tx.outputs(), interpreter.transaction().outputs());
            // save storage between client instances
            self.storage = storage;
            state
        }

        /// Build test tx and execute it
        pub fn execute(&mut self) -> StateTransition {
            let tx = self.build();
            self.execute_tx(tx)
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
            let tx = TestBuilder::build_get_balance_tx(contract_id, asset_id);
            let state = self.execute_tx(tx);
            let receipts = state.receipts();
            receipts[0].ra().expect("Balance expected")
        }
    }
}
