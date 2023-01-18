use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn backtrace() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    #[rustfmt::skip]
    let function_undefined: Vec<Opcode> = vec![
        Opcode::Undefined,
    ];

    let salt: Salt = rng.gen();
    let program: Witness = function_undefined.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_undefined = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_undefined, state_root);

    let bytecode_witness = 0;
    let tx_deploy = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, gas_costs.clone())
    .expect("failed to generate checked tx");

    client.deploy(tx_deploy);

    #[rustfmt::skip]
    let mut function_call: Vec<Opcode> = vec![
        Opcode::MOVI(0x10,  (contract_undefined.as_ref().len() + WORD_SIZE * 2) as Immediate18),
        Opcode::ALOC(0x10),
    ];

    contract_undefined.as_ref().iter().enumerate().for_each(|(i, b)| {
        function_call.push(Opcode::MOVI(0x10, *b as Immediate18));
        function_call.push(Opcode::SB(REG_HP, 0x10, 1 + i as Immediate12));
    });

    function_call.push(Opcode::ADDI(0x10, REG_HP, 1));
    function_call.push(Opcode::CALL(0x10, REG_ZERO, 0x10, REG_CGAS));
    function_call.push(Opcode::RET(REG_ONE));

    let salt: Salt = rng.gen();
    let program: Witness = function_call.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_call = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_call, state_root);

    let bytecode_witness = 0;
    let tx_deploy = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, gas_costs.clone())
    .expect("failed to generate checked tx");

    client.deploy(tx_deploy);

    #[rustfmt::skip]
    let mut script: Vec<Opcode> = vec![
        Opcode::MOVI(0x10, (contract_call.as_ref().len() + WORD_SIZE * 2) as Immediate18),
        Opcode::ALOC(0x10),
    ];

    contract_call.as_ref().iter().enumerate().for_each(|(i, b)| {
        script.push(Opcode::MOVI(0x10, *b as Immediate18));
        script.push(Opcode::SB(REG_HP, 0x10, 1 + i as Immediate12));
    });

    script.push(Opcode::ADDI(0x10, REG_HP, 1));
    script.push(Opcode::CALL(0x10, REG_ZERO, REG_ZERO, REG_CGAS));
    script.push(Opcode::RET(REG_ONE));

    let input_undefined = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_undefined);
    let output_undefined = Output::contract(0, rng.gen(), rng.gen());

    let input_call = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_call);
    let output_call = Output::contract(1, rng.gen(), rng.gen());

    let tx_script = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script.into_iter().collect(),
        vec![],
        vec![input_undefined, input_call],
        vec![output_undefined, output_call],
        vec![],
    )
    .into_checked(height, &params, gas_costs)
    .expect("failed to generate checked tx");

    client.transact(tx_script);

    let backtrace = client
        .backtrace()
        .expect("Expected erroneous state for undefined opcode");

    assert_eq!(backtrace.contract(), &contract_undefined);

    let id = backtrace.call_stack().last().expect("Caller expected").to();
    assert_eq!(id, &contract_undefined);

    let id = backtrace.call_stack().first().expect("Caller expected").to();
    assert_eq!(id, &contract_call);
}
