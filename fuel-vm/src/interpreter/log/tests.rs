use super::*;

#[test]
fn test_log() -> Result<(), RuntimeError> {
    let mut memory: Box<[u8; VM_MEMORY_SIZE]> = vec![1u8; VM_MEMORY_SIZE].try_into().unwrap();
    let context = Context::Script { block_height: 0 };
    let mut receipts = vec![];
    let mut script = Some(Script::default());

    let fp = 0;
    let is = 0;
    let mut pc = 4;
    let input = LogInput {
        memory: &mut memory,
        tx_offset: 0,
        context: &context,
        receipts: &mut receipts,
        script: script.as_mut(),
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
        tx_offset: 0,
        context: &context,
        receipts: &mut receipts,
        script: script.as_mut(),
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
        vec![1u8; 4],
        8,
        0,
    );
    assert_eq!(receipts[1], expected);

    Ok(())
}
