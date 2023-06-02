use crate::checked_transaction::IntoChecked;
use crate::gas::GasCosts;
use crate::prelude::Interpreter;
use fuel_asm::op::{aloc, logd, move_, movi, ret, sw};
use fuel_asm::RegId;
use fuel_tx::{ConsensusParameters, Finalizable, Receipt, TransactionBuilder};

const VM_PAGE_SIZE: usize = 16 * (1 << 10); // 16 KiB

#[test]
fn alloc_and_write() {
    // alloc the heap
    // write some data
    // alloc again (at least by a page size)
    // write more data to the newly allocated space
    // log data that was written before to ensure it's all intact
    let params = ConsensusParameters::DEFAULT;
    let script = vec![
        movi(0x10, 8),
        movi(0x11, 0x1337),
        movi(0x12, 0xbeef),
        movi(0x15, VM_PAGE_SIZE as u32),
        aloc(0x10),
        sw(RegId::HP, 0x11, 0),
        // store address of HP after first write
        move_(0x13, RegId::HP),
        // log data of first write
        logd(RegId::ZERO, RegId::ZERO, 0x13, 0x10),
        // alloc a new page
        aloc(0x15),
        // alloc another word
        aloc(0x10),
        // store a different value
        sw(RegId::HP, 0x12, 0),
        // log the original word again for comparison
        logd(RegId::ZERO, RegId::ZERO, 0x13, 0x10),
        ret(RegId::ZERO),
    ];

    let tx = TransactionBuilder::script(script.into_iter().collect(), vec![])
        .gas_limit(params.max_gas_per_tx)
        .add_random_fee_input()
        .finalize()
        .into_checked(0.into(), &params, &GasCosts::free())
        .expect("expected valid tx");

    let mut vm = Interpreter::with_memory_storage();
    let state = vm.transact(tx).expect("expected valid execution");

    // ensure there are no panics
    state.receipts().iter().for_each(|r| {
        if let Receipt::Panic { .. } = r {
            panic!("unexpected error {:?}", r)
        }
    });

    let log_receipts = state
        .receipts()
        .iter()
        .filter_map(|receipt| {
            if let Receipt::LogData { data, .. } = receipt {
                Some(data.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // expect exact amount of logd receipts
    assert_eq!(log_receipts.len(), 2);
    // expect first two logd receipts to be equal
    assert_eq!(log_receipts[0], log_receipts[1]);
}
