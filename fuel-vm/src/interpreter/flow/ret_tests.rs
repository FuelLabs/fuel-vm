use alloc::{
    vec,
    vec::Vec,
};

use crate::error::PanicOrBug;

use super::*;

#[test]
fn test_return() {
    let mut frame_reg: [Word; VM_REGISTER_COUNT] = core::array::from_fn(|i| i as Word);
    frame_reg[RegId::CGAS] = 100;
    let mut expected = frame_reg;
    let frame = CallFrame::new(
        ContractId::default(),
        AssetId::default(),
        frame_reg,
        0,
        0,
        0,
    )
    .unwrap();
    let mut frames = vec![frame];
    let mut registers = [0; VM_REGISTER_COUNT];
    registers[RegId::CGAS] = 99;
    registers[RegId::GGAS] = 100;
    registers[RegId::RET] = 101;
    registers[RegId::RETL] = 102;
    registers[RegId::HP] = 1234;

    expected[RegId::CGAS] = 199;
    expected[RegId::GGAS] = 100;
    expected[RegId::RET] = 101;
    expected[RegId::RETL] = 102;
    expected[RegId::HP] = 1234;
    expected[RegId::PC] += 4;
    let mut context = Context::Call {
        block_height: Default::default(),
    };

    let mut receipts = Default::default();
    let mut memory: MemoryInstance = vec![0u8; MEM_SIZE].try_into().unwrap();
    input(
        &mut frames,
        &mut registers,
        &mut receipts,
        &mut memory,
        &mut context,
    )
    .return_from_context(Receipt::ret(Default::default(), 0, 0, 0))
    .unwrap();
    assert_eq!(registers, expected);

    input(
        &mut frames,
        &mut registers,
        &mut receipts,
        &mut memory,
        &mut context,
    )
    .ret(1)
    .unwrap();
    expected[RegId::RET] = 1;
    expected[RegId::RETL] = 0;
    expected[RegId::PC] += 4;
    assert_eq!(registers, expected);
    assert_eq!(
        *receipts.as_ref().last().unwrap(),
        Receipt::ret(
            ContractId::default(),
            1,
            expected[RegId::PC] - 4,
            expected[RegId::IS]
        )
    );

    let r = input(
        &mut frames,
        &mut registers,
        &mut receipts,
        &mut memory,
        &mut context,
    )
    .ret_data(Word::MAX, Word::MAX);
    assert_eq!(r, Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)));

    let r = input(
        &mut frames,
        &mut registers,
        &mut receipts,
        &mut memory,
        &mut context,
    )
    .ret_data(VM_MAX_RAM, 1);
    assert_eq!(r, Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)));

    let r = input(
        &mut frames,
        &mut registers,
        &mut receipts,
        &mut memory,
        &mut context,
    )
    .ret_data(0, VM_MAX_RAM + 1);
    assert_eq!(r, Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)));

    let r = input(
        &mut frames,
        &mut registers,
        &mut receipts,
        &mut memory,
        &mut context,
    )
    .ret_data(20, 22)
    .unwrap();

    expected[RegId::RET] = 20;
    expected[RegId::RETL] = 22;
    expected[RegId::PC] += 4;
    assert_eq!(
        *input(
            &mut frames,
            &mut registers,
            &mut receipts,
            &mut memory,
            &mut context
        )
        .registers,
        expected
    );

    assert_eq!(
        *receipts.as_ref().last().unwrap(),
        Receipt::return_data_with_len(
            ContractId::default(),
            20,
            22,
            r,
            expected[RegId::PC] - 4,
            expected[RegId::IS],
            Some(vec![0u8; 22]),
        )
    );
}

fn input<'a>(
    frames: &'a mut Vec<CallFrame>,
    registers: &'a mut [Word; VM_REGISTER_COUNT],
    receipts: &'a mut ReceiptsCtx,
    memory: &'a mut MemoryInstance,
    context: &'a mut Context,
) -> RetCtx<'a> {
    RetCtx {
        frames,
        registers,
        receipts,
        memory,
        context,
        current_contract: Default::default(),
    }
}

#[test]
fn test_revert() {
    let mut receipts = Default::default();
    let pc = 10;
    let is = 20;
    revert(&mut receipts, None, Reg::new(&pc), Reg::new(&is), 99).expect("should be ok");
    assert_eq!(
        *receipts.as_ref().last().unwrap(),
        Receipt::revert(ContractId::default(), 99, pc, is)
    );
}
