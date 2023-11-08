use alloc::vec;

use fuel_tx::Script;
use fuel_types::BlockHeight;
use test_case::test_case;

use super::*;

#[test]
fn test_metadata() {
    let context = Context::PredicateVerification {
        program: Default::default(),
    };
    let frames = vec![];
    let mut pc = 4;
    let mut result = 1;
    let imm = 0x03;
    metadata(
        &context,
        &frames,
        RegMut::new(&mut pc),
        &mut result,
        imm,
        ChainId::default(),
    )
    .unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, 0);
}

#[test]
fn test_get_transaction_field() {
    let mut pc = 4;
    let tx = Script::default();
    let input = GTFInput {
        tx: &tx,
        tx_offset: 0,
        pc: RegMut::new(&mut pc),
    };
    let mut result = 1;
    let b = 0;
    input
        .get_transaction_field(&mut result, b, GTFArgs::ScriptGasLimit as Immediate12)
        .unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, *tx.script_gas_limit());
}

#[test_case(Context::PredicateEstimation { program: Default::default() }, 2 => (); "can fetch inside predicate estimation")]
#[test_case(Context::PredicateVerification { program: Default::default() }, 2 => (); "can fetch inside predicate verification")]
#[test_case(Context::Script { block_height: BlockHeight::default() }, 3 => (); "can fetch inside script")]
#[test_case(Context::Call { block_height: BlockHeight::default() }, 4 => (); "can fetch inside call")]
fn get_chain_id(context: Context, chain_id: u64) {
    let mut frames = vec![];
    let mut pc = 4;
    let mut result = 1;
    let imm = GMArgs::GetChainId as Immediate18;

    if context.is_internal() {
        frames.push(CallFrame::default());
    }
    metadata(
        &context,
        &frames,
        RegMut::new(&mut pc),
        &mut result,
        imm,
        chain_id.into(),
    )
    .unwrap();

    assert_eq!(result, chain_id);
}
