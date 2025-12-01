use core::panic;

use crate::{
    prelude::*,
    script_with_data_offset,
    tests::test_helpers::assert_success,
    util::test_helpers::TestBuilder,
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    Witness,
    policies::Policies,
};
use fuel_types::canonical::Serialize;
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

#[test]
fn prevent_contract_id_redeployment() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let mut client = MemoryClient::default();

    let input_amount = 1000;
    let spend_amount = 600;
    let asset_id = AssetId::BASE;

    #[rustfmt::skip]
    let function_rvrt: Vec<Instruction> = vec![
        op::rvrt(0),
    ];

    let salt: Salt = rng.r#gen();
    let program: Witness = function_rvrt.into_iter().collect::<Vec<u8>>().into();

    let contract_root = Contract::root_from_code(program.as_ref());
    let state_root = Contract::default_state_root();
    let contract_undefined = Contract::id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_undefined, state_root);

    let policies = Policies::new().with_max_fee(0);

    let mut create = Transaction::create(
        Default::default(),
        policies,
        salt,
        vec![],
        vec![],
        vec![
            output,
            Output::change(rng.r#gen(), 0, asset_id),
            Output::coin(rng.r#gen(), spend_amount, asset_id),
        ],
        vec![program, Witness::default()],
    );
    create.add_unsigned_coin_input(
        rng.r#gen(),
        &Default::default(),
        input_amount,
        asset_id,
        rng.r#gen(),
        Default::default(),
    );

    let consensus_params = ConsensusParameters::standard();

    let create = create
        .into_checked_basic(1.into(), &consensus_params)
        .expect("failed to generate checked tx");

    // deploy contract
    client
        .deploy(create.clone())
        .expect("First create should be executed");
    let mut txtor: Transactor<_, _, _> = client.into();
    // second deployment should fail
    let result = txtor.deploy(create).unwrap_err();
    assert_eq!(
        result,
        InterpreterError::Panic(PanicReason::ContractIdAlreadyDeployed)
    );
}

#[test]
fn mint_consumes_gas_for_new_assets() {
    let mut test_context = TestBuilder::new(2322u64);

    let balance = 1000;
    let gas_limit = 1_000_000;

    let [new_asset, existing_asset] = [true, false].map(|create_new_asset| {
        let mut program = vec![
            op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
            op::lw(0x10, 0x10, 0),
            op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
            op::lw(0x11, 0x11, 0),
            // Allocate 32 bytes for the zeroed `sub_id`.
            op::movi(0x15, Bytes32::LEN as u32),
            op::aloc(0x15),
        ];

        // Mint some of the asset before to make the asset exist before the measured mint
        if !create_new_asset {
            program.push(op::mint(0x11, RegId::HP));
        }

        // The mint we're measuring
        program.extend([
            op::log(RegId::GGAS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::mint(0x11, RegId::HP),
            op::log(RegId::GGAS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::ret(RegId::ONE),
        ]);

        let contract_id = test_context.setup_contract(program, None, None).contract_id;

        let (script_call, _) = script_with_data_offset!(
            data_offset,
            vec![
                op::movi(0x10, data_offset as Immediate18),
                op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
                op::ret(RegId::ONE),
            ],
            test_context.get_tx_params().tx_offset()
        );
        let script_call_data = Call::new(contract_id, 0, balance).to_bytes();

        let result = test_context
            .start_script(script_call.clone(), script_call_data)
            .script_gas_limit(gas_limit)
            .contract_input(contract_id)
            .fee_input()
            .contract_output(&contract_id)
            .execute();

        let mut gas_values = result.receipts().iter().filter_map(|v| match v {
            Receipt::Log { ra, .. } => Some(ra),
            _ => None,
        });

        let gas_before = gas_values.next().expect("Missing log receipt");
        let gas_after = gas_values.next().expect("Missing log receipt");
        gas_before - gas_after
    });

    assert!(new_asset > existing_asset);
}

#[test]
fn mint_and_transfer_out() {
    let mut test_context = TestBuilder::new(2322u64);

    let program = vec![
        // Allocate a buffer
        op::movi(0x15, 64),
        op::aloc(0x15),
        // Mint one of the asset using zeroed sub_id
        op::mint(RegId::ONE, RegId::HP),
        // Compute full asset id
        op::movi(0x11, SubAssetId::LEN as _),
        op::add(0x10, RegId::HP, 0x11),
        op::mcp(0x10, RegId::HP, 0x11),
        op::mcp(RegId::HP, RegId::FP, 0x11),
        op::s256(RegId::HP, RegId::HP, 0x15),
        // Zero output address
        op::addi(0x12, RegId::HP, 32),
        op::mcli(0x12, 32),
        // Transfer it out to zero address
        op::tro(0x12, RegId::ONE, RegId::ONE, RegId::HP),
        op::ret(RegId::ONE),
    ];

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script_call, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_call_data = Call::new(contract_id, 0, 0).to_bytes();

    let result = test_context
        .start_script(script_call.clone(), script_call_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .variable_output(AssetId::zeroed())
        .execute();

    assert_success(result.receipts());

    for r in result.receipts() {
        let Receipt::TransferOut { to, amount, .. } = r else {
            continue;
        };

        assert_eq!(*to, Address::zeroed());
        assert_eq!(*amount, 1);

        return;
    }

    panic!("Missing TransferOut receipt");
}
