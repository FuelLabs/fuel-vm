//! FuelVM utilities

/// A utility macro for writing scripts with the data offset included. Since the script data offset
/// depends on the length of the script, this macro will evaluate the length and then rewrite the
/// resultant script output with the correct offset (using the offset parameter).
///
/// # Example
///
/// ```
/// use itertools::Itertools;
/// use fuel_types::Word;
/// use fuel_vm::consts::{REG_ONE, REG_ZERO};
/// use fuel_vm::prelude::{Opcode, Call, SerializableVec, ContractId, Immediate12};
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
/// let script = script_with_data_offset!(data_offset, vec![
///     // use data_offset to reference the location of the call bytes inside script_data
///     Opcode::ADDI(0x10, REG_ZERO, data_offset as Immediate12),
///     Opcode::ADDI(0x11, REG_ZERO, transfer_amount as Immediate12),
///     // use data_offset again to reference the location of the asset id inside of script data
///     Opcode::ADDI(0x12, REG_ZERO, (data_offset + call.len()) as Immediate12),
///     Opcode::ADDI(0x13, REG_ZERO, gas_to_forward as Immediate12),
///     Opcode::CALL(0x10, 0x11, 0x12, 0x13),
///     Opcode::RET(REG_ONE),
/// ]);
/// ```
#[macro_export]
macro_rules! script_with_data_offset {
    ($offset:ident, $script:expr) => {{
        use fuel_types::bytes;
        use fuel_vm::consts::VM_TX_MEMORY;
        use fuel_vm::prelude::Transaction;
        let $offset = 0;
        let script_bytes: Vec<u8> = { $script }.into_iter().collect();
        let data_offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script_bytes.as_slice());
        let $offset = data_offset;
        $script
    }};
}

#[cfg(any(test, feature = "test-helpers"))]
/// Testing utilities
pub mod test_helpers {
    use crate::consts::REG_ONE;
    use crate::prelude::{MemoryClient, MemoryStorage, Transactor};
    use fuel_asm::Opcode;
    use fuel_tx::{Input, Output, Transaction, Witness};
    use fuel_types::{Color, ContractId, Word};
    use itertools::Itertools;
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};

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

        pub fn change_output(&mut self, color: Color) -> &mut TestBuilder {
            self.outputs.push(Output::change(self.rng.gen(), 0, color));
            self
        }

        pub fn coin_output(&mut self, color: Color, amount: Word) -> &mut TestBuilder {
            self.outputs.push(Output::coin(self.rng.gen(), amount, color));
            self
        }

        pub fn withdrawal_output(&mut self, color: Color, amount: Word) -> &mut TestBuilder {
            self.outputs.push(Output::withdrawal(self.rng.gen(), amount, color));
            self
        }

        pub fn variable_output(&mut self, color: Color) -> &mut TestBuilder {
            self.outputs.push(Output::variable(self.rng.gen(), 0, color));
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

        pub fn coin_input(&mut self, color: Color, amount: Word) -> &mut TestBuilder {
            self.inputs.push(Input::coin(
                self.rng.gen(),
                self.rng.gen(),
                amount,
                color,
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

        pub fn execute(&mut self) -> Vec<Output> {
            let tx = self.build();
            let mut client = MemoryClient::new(self.storage.clone());
            client.transact(tx);
            let txtor: Transactor<_> = client.into();
            let outputs = txtor.state_transition().unwrap().tx().outputs();
            outputs.to_vec()
        }

        pub fn execute_get_change(&mut self, find_color: Color) -> Word {
            let outputs = self.execute();
            let change = outputs.into_iter().find_map(|output| {
                if let Output::Change { amount, color, .. } = output {
                    if &color == &find_color {
                        Some(amount)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            change.expect(format!("no change matching color {:x} was found", &find_color).as_str())
        }
    }
}
