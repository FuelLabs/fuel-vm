use fuel_vm::consts::*;
use fuel_vm::prelude::*;

#[test]
fn loaded_value_cannot_be_written_to_read_only_register() {
    for load_instruction in [Opcode::LB(0, 0, 0), Opcode::LW(0, 0, 0)] {
        let script = vec![load_instruction, Opcode::RET(REG_ONE)];

        let tx = Transaction::script(
            0,
            1_000_000,
            0,
            script.into_iter().collect(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .check(0, &ConsensusParameters::default())
        .expect("failed to check tx");

        let receipts = Transactor::new(MemoryStorage::default(), Default::default())
            .transact(tx)
            .receipts()
            .expect("Failed to execute script!")
            .to_owned();

        if let Receipt::Panic { reason, .. } = receipts.get(0).unwrap() {
            assert_eq!(*reason.reason(), PanicReason::ReservedRegisterNotWritable);
        } else {
            panic!("The contract, unexpectedly, did not panic after executing {load_instruction:?}");
        }
    }
}
