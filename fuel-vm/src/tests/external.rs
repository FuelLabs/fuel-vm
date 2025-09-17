use crate::{
    error::SimpleResult,
    interpreter::EcalHandler,
    prelude::*,
};
use alloc::vec;
use fuel_asm::{
    GTFArgs,
    RegId,
    Word,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    PanicReason,
    Receipt,
    Script,
    ScriptExecutionResult,
    TransactionBuilder,
};
use itertools::Itertools;
use test_case::test_case;

use crate::tests::test_helpers::{
    assert_panics,
    run_script,
};

#[test]
fn attempt_ecal_without_handler() {
    let receipts = run_script(vec![op::ecal(
        RegId::ZERO,
        RegId::ZERO,
        RegId::ZERO,
        RegId::ZERO,
    )]);

    assert_panics(&receipts, PanicReason::EcalError);
}

/// An ECAL opcode handler function, which charges for `noop` and does nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopEcal;

impl EcalHandler for NoopEcal {
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        vm.gas_charge(vm.gas_costs().noop())
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

    let mut client = MemoryClient::<_, NoopEcal>::new(
        MemoryInstance::new(),
        MemoryStorage::default(),
        Default::default(),
    );
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, vec![])
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
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

impl EcalHandler for SumProdEcal {
    /// This ecal fn computes saturating sum and product of inputs (a,b,c,d),
    /// and stores them in a and b respectively. It charges only a single gas.
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()> {
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
    let vm: Interpreter<_, _, Script, SumProdEcal> = Interpreter::with_memory_storage();

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
        .add_fee_input()
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

#[derive(Debug, Default, Clone, Copy)]
pub struct ComplexEcal {
    state: u64,
}

impl EcalHandler for ComplexEcal {
    const INC_PC: bool = false;

    /// Ecal meant for testing cornercase behavior of the handler.
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        a: RegId,
        _b: RegId,
        _c: RegId,
        _d: RegId,
    ) -> SimpleResult<()> {
        vm.gas_charge(1)?;

        if vm.ecal_state().state > 10 {
            // Just some nonsensical error
            return Err(PanicReason::NotEnoughBalance.into());
        }

        let a = vm.registers()[a];

        vm.registers_mut()[RegId::PC] = if a == 0 {
            return Err(PanicReason::EcalError.into());
        } else {
            vm.registers_mut()[RegId::PC].wrapping_sub(4 * a)
        };

        vm.ecal_state_mut().state += 1;

        Ok(())
    }
}

#[test_case(0, PanicReason::EcalError; "ecal itself errors")]
#[test_case(1, PanicReason::NotEnoughBalance; "ecal hits loop limit")]
#[test_case(10, PanicReason::MemoryNotExecutable; "ecal jumps out of bounds")]
fn complex_ecal_fn(val: u32, result: PanicReason) {
    let vm: Interpreter<_, _, Script, ComplexEcal> = Interpreter::with_memory_storage();

    let script = vec![
        op::movi(0x20, val),
        op::ecal(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::from_txtor(vm.into());
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, vec![])
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");

    assert_panics(receipts, result);
}
