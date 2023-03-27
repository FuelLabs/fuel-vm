use fuel_tx::Script;

use super::*;

#[test]
fn test_metadata() {
    let context = Context::Predicate {
        program: Default::default(),
    };
    let frames = vec![];
    let mut pc = 4;
    let mut result = 1;
    let imm = 0x03;
    metadata(&context, &frames, RegMut::new(&mut pc), &mut result, imm).unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, 0);
}

#[test]
fn test_get_transaction_field() {
    let mut pc = 4;
    let input = GTFInput {
        tx: &Script::default(),
        tx_offset: 0,
        pc: RegMut::new(&mut pc),
    };
    let mut result = 1;
    let imm = 2;
    let b = 0;
    input.get_transaction_field(&mut result, b, imm).unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, 0);
}
