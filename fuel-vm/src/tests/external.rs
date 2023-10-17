use fuel_asm::{
    op,
    GTFArgs,
    RegId,
    Word,
};
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    Receipt,
    Script,
    ScriptExecutionResult,
    TransactionBuilder,
};
use fuel_vm::{
    prelude::{
        Interpreter,
        IntoChecked,
        MemoryClient,
    },
    storage::MemoryStorage,
};
use itertools::Itertools;

#[test]
fn default_ecal() {
    let script = vec![
        op::ecal(RegId::ZERO, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::default();
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, vec![])
        .gas_price(0)
        .gas_limit(1_000_000)
        .maturity(Default::default())
        .add_random_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");

    let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() else {
        panic!("Expected a result receipt");
    };
    assert_eq!(*result, ScriptExecutionResult::Success);
}

#[test]
fn provide_ecal_fn() {
    let mut vm: Interpreter<MemoryStorage, Script> = Interpreter::with_memory_storage();
    vm.set_ecal(|vm, a, b, c, d| {
        // This ecal fn computes saturatign sum and product of inputs (a,b,c,d),
        // and stores them in a and b respectively. It charges only a single gas.

        vm.gas_charge(1)?;

        let args = [
            vm.registers()[a],
            vm.registers()[b],
            vm.registers()[c],
            vm.registers()[d],
        ];

        let sum = args.into_iter().reduce(Word::saturating_add).unwrap();
        let product = args.into_iter().reduce(Word::saturating_mul).unwrap();

        vm.registers_mut()[a] = sum;
        vm.registers_mut()[b] = product;

        Ok(())
    });

    let script_data = [
        2u64.to_be_bytes(),
        3u64.to_be_bytes(),
        4u64.to_be_bytes(),
        5u64.to_be_bytes(),
    ]
    .into_iter()
    .flatten()
    .collect_vec();
    let script = vec![
        op::gtf_args(0x10, 0x00, GTFArgs::ScriptData),
        op::lw(0x20, 0x10, 0),
        op::lw(0x21, 0x10, 1),
        op::lw(0x22, 0x10, 2),
        op::lw(0x23, 0x10, 3),
        op::ecal(0x20, 0x21, 0x22, 0x23),
        op::log(0x20, 0x21, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::from_txtor(vm.into());
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(0)
        .gas_limit(1_000_000)
        .maturity(Default::default())
        .add_random_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");

    let Receipt::Log { ra, rb, .. } = receipts.first().unwrap() else {
        panic!("Expected a log receipt");
    };

    assert_eq!(*ra, 2 + 3 + 4 + 5);
    assert_eq!(*rb, 2 * 3 * 4 * 5);
}
