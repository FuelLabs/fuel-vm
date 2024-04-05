use alloc::vec;

use crate::interpreter::memory::Memory;

use super::*;
use fuel_vm::consts::*;

#[test]
fn test_log() -> SimpleResult<()> {
    let mut memory: Memory = vec![1u8; MEM_SIZE].try_into().unwrap();
    let context = Context::Script {
        block_height: Default::default(),
    };
    let mut receipts = Default::default();

    let fp = 0;
    let is = 0;
    let mut pc = 4;
    let input = LogInput {
        memory: &mut memory,
        context: &context,
        receipts: &mut receipts,
        fp: Reg::new(&fp),
        is: Reg::new(&is),
        pc: RegMut::new(&mut pc),
    };
    input.log(1, 2, 3, 4)?;

    assert_eq!(pc, 8);
    assert_eq!(receipts.len(), 1);

    let expected = Receipt::log(Default::default(), 1, 2, 3, 4, 4, 0);
    assert_eq!(receipts[0], expected);

    let input = LogInput {
        memory: &mut memory,
        context: &context,
        receipts: &mut receipts,
        fp: Reg::new(&fp),
        is: Reg::new(&is),
        pc: RegMut::new(&mut pc),
    };
    input.log_data(1, 2, 3, 4)?;

    assert_eq!(pc, 12);
    assert_eq!(receipts.len(), 2);

    let expected = Receipt::log_data_with_len(
        Default::default(),
        1,
        2,
        3,
        4,
        *receipts[1].digest().unwrap(),
        8,
        0,
        Some(vec![1u8; 4]),
    );
    assert_eq!(receipts[1], expected);

    Ok(())
}
