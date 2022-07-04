#![allow(clippy::iter_cloned_collect)] // https://github.com/rust-lang/rust-clippy/issues/9119

use fuel_crypto::Hasher;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn metadata() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    #[rustfmt::skip]
    let routine_metadata_is_caller_external: Vec<Opcode> = vec![
        Opcode::gm(0x10, GMArgs::IsCallerExternal),
        Opcode::gm(0x11, GMArgs::GetCaller),
        Opcode::LOG(0x10, 0x00, 0x00, 0x00),
        Opcode::MOVI(0x20,  ContractId::LEN as Immediate18),
        Opcode::LOGD(0x00, 0x00, 0x11, 0x20),
        Opcode::RET(REG_ONE),
    ];

    let salt: Salt = rng.gen();
    let program: Witness = routine_metadata_is_caller_external
        .into_iter()
        .collect::<Vec<u8>>()
        .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_metadata = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_metadata, state_root);

    let bytecode_witness = 0;
    let tx = Transaction::create(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    );

    // Deploy the contract into the blockchain
    assert!(Transactor::new(&mut storage, Default::default())
        .transact(tx)
        .is_success());

    let mut routine_call_metadata_contract: Vec<Opcode> = vec![
        Opcode::gm(0x10, GMArgs::IsCallerExternal),
        Opcode::LOG(0x10, 0x00, 0x00, 0x00),
        Opcode::MOVI(0x10, (Bytes32::LEN + 2 * Bytes8::LEN) as Immediate18),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x10, REG_HP, 1),
    ];

    contract_metadata.as_ref().iter().enumerate().for_each(|(i, b)| {
        routine_call_metadata_contract.push(Opcode::MOVI(0x11, *b as Immediate18));
        routine_call_metadata_contract.push(Opcode::SB(0x10, 0x11, i as Immediate12));
    });

    routine_call_metadata_contract.push(Opcode::CALL(0x10, REG_ZERO, 0x10, REG_CGAS));
    routine_call_metadata_contract.push(Opcode::RET(REG_ONE));

    let salt: Salt = rng.gen();
    let program: Witness = routine_call_metadata_contract.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_call = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_call, state_root);

    let bytecode_witness = 0;
    let tx = Transaction::create(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    );

    // Deploy the contract into the blockchain
    assert!(Transactor::new(&mut storage, Default::default())
        .transact(tx)
        .is_success());

    let mut inputs = vec![];
    let mut outputs = vec![];

    inputs.push(Input::contract(rng.gen(), rng.gen(), rng.gen(), contract_call));
    outputs.push(Output::contract(0, rng.gen(), rng.gen()));

    inputs.push(Input::contract(rng.gen(), rng.gen(), rng.gen(), contract_metadata));
    outputs.push(Output::contract(1, rng.gen(), rng.gen()));

    let mut script = vec![
        Opcode::MOVI(0x10, (Bytes32::LEN + 2 * Bytes8::LEN) as Immediate18),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x10, REG_HP, 1),
    ];

    contract_call.as_ref().iter().enumerate().for_each(|(i, b)| {
        script.push(Opcode::MOVI(0x11, *b as Immediate18));
        script.push(Opcode::SB(0x10, 0x11, i as Immediate12));
    });

    script.push(Opcode::CALL(0x10, REG_ZERO, 0x10, REG_CGAS));
    script.push(Opcode::RET(REG_ONE));

    let script = script.iter().copied().collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        inputs,
        outputs,
        vec![],
    );

    let receipts = Transactor::new(&mut storage, Default::default())
        .transact(tx)
        .receipts()
        .expect("Failed to transact")
        .to_owned();

    let ra = receipts[1]
        .ra()
        .expect("IsCallerExternal should set $rA as boolean flag");
    assert_eq!(1, ra);

    let ra = receipts[3]
        .ra()
        .expect("IsCallerExternal should set $rA as boolean flag");
    assert_eq!(0, ra);

    let contract_call = Hasher::hash(contract_call.as_ref());
    let digest = receipts[4].digest().expect("GetCaller should return contract Id");
    assert_eq!(&contract_call, digest);
}
