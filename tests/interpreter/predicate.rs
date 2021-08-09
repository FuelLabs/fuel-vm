use fuel_tx::bytes;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn predicate() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let predicate_data = 0x23 as Word;
    let predicate_data = predicate_data.to_be_bytes().to_vec();
    let predicate_data_len = bytes::padded_len(predicate_data.as_slice()) as Immediate12;

    let mut predicate = vec![];

    predicate.push(Opcode::ADDI(0x10, REG_ZERO, 0x11));
    predicate.push(Opcode::ADDI(0x11, 0x10, 0x12));
    predicate.push(Opcode::ADDI(0x12, REG_ZERO, 0x08));
    predicate.push(Opcode::ALOC(0x12));
    predicate.push(Opcode::ADDI(0x12, REG_HP, 0x01));
    predicate.push(Opcode::SW(0x12, 0x11, 0));
    predicate.push(Opcode::ADDI(0x10, REG_ZERO, 0x08));
    predicate.push(Opcode::XIL(0x20, 0));
    predicate.push(Opcode::XIS(0x11, 0));
    predicate.push(Opcode::ADD(0x11, 0x11, 0x20));
    predicate.push(Opcode::SUBI(0x11, 0x11, predicate_data_len));
    predicate.push(Opcode::MEQ(0x10, 0x11, 0x12, 0x10));
    predicate.push(Opcode::RET(0x10));

    let predicate = predicate
        .into_iter()
        .map(|op| u32::from(op).to_be_bytes())
        .flatten()
        .collect();

    let maturity = 0;
    let salt: Salt = rng.gen();
    let witness = vec![];

    let contract = Contract::from(witness.as_ref());
    let contract = <Interpreter<MemoryStorage>>::contract_id(vm.as_mut(), &contract, &salt)
        .expect("Failed to calculate contract ID from storage!");

    let input = Input::coin(
        rng.gen(),
        rng.gen(),
        0,
        rng.gen(),
        0,
        maturity,
        predicate,
        predicate_data,
    );
    let output = Output::contract_created(contract);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let bytecode_witness_index = 0;
    let static_contracts = vec![];
    let inputs = vec![input];
    let outputs = vec![output];
    let witnesses = vec![witness.into()];

    let tx = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness_index,
        salt,
        static_contracts,
        inputs,
        outputs,
        witnesses,
    );

    vm.transact(tx).expect("Failed to transact!");
}

#[test]
fn predicate_false() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let predicate_data = 0x24 as Word;
    let predicate_data = predicate_data.to_be_bytes().to_vec();
    let predicate_data_len = bytes::padded_len(predicate_data.as_slice()) as Immediate12;

    let mut predicate = vec![];

    predicate.push(Opcode::ADDI(0x10, REG_ZERO, 0x11));
    predicate.push(Opcode::ADDI(0x11, 0x10, 0x12));
    predicate.push(Opcode::ADDI(0x12, REG_ZERO, 0x08));
    predicate.push(Opcode::ALOC(0x12));
    predicate.push(Opcode::ADDI(0x12, REG_HP, 0x01));
    predicate.push(Opcode::SW(0x12, 0x11, 0));
    predicate.push(Opcode::ADDI(0x10, REG_ZERO, 0x08));
    predicate.push(Opcode::XIL(0x20, 0));
    predicate.push(Opcode::XIS(0x11, 0));
    predicate.push(Opcode::ADD(0x11, 0x11, 0x20));
    predicate.push(Opcode::SUBI(0x11, 0x11, predicate_data_len));
    predicate.push(Opcode::MEQ(0x10, 0x11, 0x12, 0x10));
    predicate.push(Opcode::RET(0x10));

    let predicate = predicate
        .into_iter()
        .map(|op| u32::from(op).to_be_bytes())
        .flatten()
        .collect();

    let maturity = 0;
    let salt: Salt = rng.gen();
    let witness = vec![];

    let contract = Contract::from(witness.as_ref());
    let contract = <Interpreter<MemoryStorage>>::contract_id(vm.as_mut(), &contract, &salt)
        .expect("Failed to calculate contract ID from storage!");

    let input = Input::coin(
        rng.gen(),
        rng.gen(),
        0,
        rng.gen(),
        0,
        maturity,
        predicate,
        predicate_data,
    );
    let output = Output::contract_created(contract);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let bytecode_witness_index = 0;
    let static_contracts = vec![];
    let inputs = vec![input];
    let outputs = vec![output];
    let witnesses = vec![witness.into()];

    let tx = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness_index,
        salt,
        static_contracts,
        inputs,
        outputs,
        witnesses,
    );

    assert!(vm.transact(tx).is_err());
}
