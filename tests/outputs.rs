use fuel_vm::{
    consts::{REG_ONE, REG_ZERO},
    prelude::*,
    script_with_data_offset,
};
use itertools::Itertools;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Testing of post-execution output handling

#[test]
fn full_change_with_no_fees() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

    assert_eq!(change, input_amount);
}

#[test]
fn byte_fees_are_deducted_from_base_asset_change() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 1;

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

    assert!(change < input_amount);
}

#[test]
fn used_gas_is_deducted_from_base_asset_change() {
    let input_amount = 1000;
    let gas_price = 1;
    let byte_price = 0;

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

    assert!(change < input_amount);
}

#[test]
fn used_gas_is_deducted_from_base_asset_change_on_revert() {
    let input_amount = 1000;
    let gas_price = 1;
    let byte_price = 0;

    let change = TestBuilder::new(2322u64)
        .script(vec![
            // Log some dummy data to burn extra gas
            Opcode::LOG(REG_ONE, REG_ONE, REG_ONE, REG_ONE),
            // Revert transaction
            Opcode::RVRT(REG_ONE),
        ])
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

    assert!(change < input_amount);
}

#[test]
fn correct_change_is_provided_for_coin_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 600;
    let color = Color::default();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .coin_output(color, spend_amount)
        .execute_get_change(color);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn correct_change_is_provided_for_withdrawal_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 650;
    let color = Color::default();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .withdrawal_output(color, spend_amount)
        .execute_get_change(color);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
#[should_panic(expected = "ValidationError(TransactionOutputChangeColorDuplicated)")]
fn change_is_not_duplicated_for_each_base_asset_change_output() {
    // create multiple change outputs for the base asset and ensure the total change is correct
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let color = Color::default();

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .change_output(color)
        .execute();

    let mut total_change = 0;
    for output in outputs {
        if let Output::Change { amount, .. } = output {
            total_change += amount;
        }
    }
    // verify total change matches the input amount
    assert_eq!(total_change, input_amount);
}

#[test]
fn change_is_reduced_by_external_transfer() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let input_amount = 1000;
    let transfer_amount: Word = 400;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = Color::default();

    // setup state for test
    let mut storage = MemoryStorage::default();
    // simple dummy contract for transferring value to
    let contract_code = vec![Opcode::RET(REG_ONE)].into_iter().collect::<Vec<u8>>();
    let contract = Contract::from(contract_code.as_ref());
    let salt: Salt = rng.gen();
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);
    storage.storage_contract_insert(&contract_id, &contract).unwrap();
    storage
        .storage_contract_root_insert(&contract_id, &salt, &contract_root)
        .unwrap();

    // setup script for transfer
    let script = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            Opcode::ADDI(0x10, REG_ZERO, data_offset as Immediate12),
            // set reg 0x11 to transfer amount
            Opcode::ADDI(0x11, REG_ZERO, transfer_amount as Immediate12),
            // set reg 0x12 to color
            Opcode::ADDI(0x12, REG_ZERO, (data_offset + 32) as Immediate12),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the color at 0x12
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ]
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - transfer_amount);
}

#[test]
fn change_is_not_reduced_by_external_transfer_on_revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let input_amount = 1000;
    // attempt overspend to cause a revert
    let transfer_amount: Word = input_amount + 100;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = Color::default();

    // setup state for test
    let mut storage = MemoryStorage::default();
    // simple dummy contract for transferring value to
    let contract_code = vec![Opcode::RET(REG_ONE)].into_iter().collect::<Vec<u8>>();
    let contract = Contract::from(contract_code.as_ref());
    let salt: Salt = rng.gen();
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);
    storage.storage_contract_insert(&contract_id, &contract).unwrap();
    storage
        .storage_contract_root_insert(&contract_id, &salt, &contract_root)
        .unwrap();

    // setup script for transfer
    let script = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            Opcode::ADDI(0x10, REG_ZERO, data_offset as Immediate12),
            // set reg 0x11 to transfer amount
            Opcode::ADDI(0x11, REG_ZERO, transfer_amount as Immediate12),
            // set reg 0x12 to color
            Opcode::ADDI(0x12, REG_ZERO, (data_offset + 32) as Immediate12),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the color at 0x12
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ]
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount);
}

#[test]
#[ignore]
fn variable_output_increased_by_contract_transfer_out() {}

#[test]
#[ignore]
fn variable_output_not_increased_by_contract_transfer_out_on_revert() {}

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
