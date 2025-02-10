use alloc::{
    vec,
    vec::Vec,
};

use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    field::ReceiptsRoot,
    ConsensusParameters,
    Finalizable,
    GasCosts,
    Script,
    TransactionBuilder,
};

use crate::{
    prelude::{
        Interpreter,
        IntoChecked,
    },
    state::ProgramState,
};

#[test]
fn receipts_are_produced_correctly_with_stepping() {
    let script = vec![
        op::movi(0x20, 1234),
        op::log(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, Vec::new())
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &params)
        .expect("failed to check tx")
        .into_ready(0, &GasCosts::default(), params.fee_params(), None)
        .expect("failed to ready tx");

    let mut vm = Interpreter::<_, _, Script>::with_memory_storage();
    vm.transact(tx.clone()).expect("panicked");
    let receipts_without_debugger = vm.receipts().to_vec();
    let receipts_root_without_debugger = vm.transaction().receipts_root();

    let mut vm = Interpreter::<_, _, Script>::with_memory_storage();
    vm.set_single_stepping(true);
    let mut t = *vm.transact(tx).expect("panicked").state();
    loop {
        match t {
            ProgramState::Return(_)
            | ProgramState::ReturnData(_)
            | ProgramState::Revert(_) => {
                break;
            }
            ProgramState::RunProgram(_) => {
                t = vm.resume().expect("panicked");
            }
            ProgramState::VerifyPredicate(_) => {
                unreachable!("no predicates in this test")
            }
        }
    }
    let receipts_with_debugger = vm.receipts();
    let receipts_root_with_debugger = vm.transaction().receipts_root();

    assert_eq!(receipts_without_debugger, receipts_with_debugger);
    assert_eq!(receipts_root_without_debugger, receipts_root_with_debugger);
}
