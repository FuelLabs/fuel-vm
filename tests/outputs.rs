use fuel_vm::consts::REG_ONE;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Testing of post-execution output handling

#[test]
fn full_change_with_no_fees() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute();

    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount == &input_amount && color == &Color::default())
    );
}

#[test]
fn byte_fees_are_deducted_from_base_asset_change() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 1;

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute();

    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount < &input_amount && color == &Color::default())
    );
}

#[test]
fn used_gas_is_deducted_from_base_asset_change() {
    let input_amount = 1000;
    let gas_price = 1;
    let byte_price = 0;

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute();

    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount < &input_amount && color == &Color::default())
    );
}

#[test]
fn used_gas_is_deducted_from_base_asset_change_on_revert() {
    let input_amount = 1000;
    let gas_price = 1;
    let byte_price = 0;

    let outputs = TestBuilder::new(2322u64)
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
        .execute();

    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount < &input_amount && color == &Color::default())
    );
}

#[test]
fn correct_change_is_provided_for_coin_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 600;
    let color = Color::default();

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .coin_output(color, spend_amount)
        .execute();

    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount == &(input_amount - spend_amount) && color == &Color::default())
    );
}

#[test]
fn correct_change_is_provided_for_withdrawal_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 650;
    let color = Color::default();

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .withdrawal_output(color, spend_amount)
        .execute();

    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount == &(input_amount - spend_amount) && color == &Color::default())
    );
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

// TODO: implement these test cases when TR opcode is supported
#[test]
#[ignore]
fn base_asset_change_is_reduced_by_contract_transfer() {}

#[test]
#[ignore]
fn base_asset_change_is_not_reduced_by_contract_transfer_on_revert() {}

#[test]
#[ignore]
fn asset_change_reduced_by_contract_transfer() {}

#[test]
#[ignore]
fn asset_change_not_reduced_by_contract_transfer_on_revert() {}

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
        }
    }

    pub fn gas_price(&mut self, price: Word) -> &mut TestBuilder {
        self.gas_price = price;
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
        let mut client = MemoryClient::default();
        client.transact(tx);
        let txtor: Transactor<_> = client.into();
        let outputs = txtor.state_transition().unwrap().tx().outputs();
        outputs.to_vec()
    }
}
