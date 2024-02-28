use alloc::vec;
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
use fuel_vm::prelude::{
    Interpreter,
    IntoChecked,
    MemoryClient,
};
use itertools::Itertools;

/// An ECAL opcode handler function, which charges for `noop` and does nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopEcal;

impl ::fuel_vm::interpreter::EcalHandler for NoopEcal {
    fn ecal<S, Tx>(
        vm: &mut ::fuel_vm::prelude::Interpreter<S, Tx, Self>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> ::fuel_vm::error::SimpleResult<()> {
        vm.gas_charge(vm.gas_costs().noop)
    }
}

#[test]
fn noop_ecal() {
    let script = vec![
        op::ecal(RegId::ZERO, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::<NoopEcal>::new(
        fuel_vm::prelude::MemoryStorage::default(),
        Default::default(),
    );
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, vec![])
        .script_gas_limit(1_000_000)
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

#[derive(Debug, Default, Clone, Copy)]
pub struct SumProdEcal;

impl ::fuel_vm::interpreter::EcalHandler for SumProdEcal {
    /// This ecal fn computes saturating sum and product of inputs (a,b,c,d),
    /// and stores them in a and b respectively. It charges only a single gas.
    fn ecal<S, Tx>(
        vm: &mut ::fuel_vm::prelude::Interpreter<S, Tx, Self>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> ::fuel_vm::error::SimpleResult<()> {
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
    }
}

#[test]
fn provide_ecal_fn() {
    let vm: Interpreter<_, Script, SumProdEcal> = Interpreter::with_memory_storage();

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
        .script_gas_limit(1_000_000)
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
