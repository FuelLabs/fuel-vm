use alloc::vec;

use fuel_tx::{
    Script,
    TxParameters,
};
use fuel_types::BlockHeight;
use test_case::test_case;

use crate::prelude::RuntimePredicate;

use super::*;

#[test]
fn test_metadata() {
    let context = Context::PredicateVerification {
        program: RuntimePredicate::empty(),
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
        TxParameters::default().tx_offset() as Word,
        0,
    )
    .unwrap();
    assert_eq!(pc, 8);
    assert_eq!(result, 0);
}

#[test]
fn test_get_transaction_field() {
    let mut pc = 4;
    let tx = Script::default();
    let input_contracts_index_to_output_index = Default::default();
    let input = GTFInput {
        tx: &tx,
        input_contracts_index_to_output_index: &input_contracts_index_to_output_index,
        tx_offset: 0,
        tx_size: fuel_tx::TxParameters::DEFAULT.tx_offset() as Word,
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

#[test_case(Context::PredicateEstimation { program: RuntimePredicate::empty() }, 2 => (); "can fetch inside predicate estimation")]
#[test_case(Context::PredicateVerification { program: RuntimePredicate::empty() }, 2 => (); "can fetch inside predicate verification")]
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
        TxParameters::default().tx_offset() as Word,
        0,
    )
    .unwrap();

    assert_eq!(result, chain_id);
}

// should fail in predicate estimation and predicate verification, but pass in other
// contexts
#[test_case(Context::PredicateEstimation { program: RuntimePredicate::empty() }, 2, Err(PanicReason::CanNotGetGasPriceInPredicate.into()); "can not fetch inside predicate estimation")]
#[test_case(Context::PredicateVerification { program: RuntimePredicate::empty() }, 2, Err(PanicReason::CanNotGetGasPriceInPredicate.into()); "can not fetch inside predicate verification")]
#[test_case(Context::Script { block_height: BlockHeight::default() }, 2, Ok(()); "can fetch inside script")]
#[test_case(Context::Call { block_height: BlockHeight::default() }, 2, Ok(()); "can fetch inside call")]
fn get_gas_price(context: Context, gas_price: u64, expected_res: SimpleResult<()>) {
    let mut frames = vec![];
    let mut pc = 4;
    let mut result = 0;
    let imm = GMArgs::GetGasPrice as Immediate18;

    if context.is_internal() {
        frames.push(CallFrame::default());
    }

    let actual_res = metadata(
        &context,
        &frames,
        RegMut::new(&mut pc),
        &mut result,
        imm,
        ChainId::default(),
        TxParameters::default().tx_offset() as Word,
        gas_price,
    );

    match expected_res {
        Ok(_) => {
            assert_eq!(actual_res, Ok(()));
            assert_eq!(result, gas_price);
        }
        Err(e) => {
            assert_eq!(actual_res, Err(e));
            assert_eq!(result, 0);
        }
    }
}
