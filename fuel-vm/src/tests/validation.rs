use fuel_asm::op;
use fuel_tx::TransactionBuilder;
use fuel_types::BlockHeight;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn transaction_can_be_executed_after_maturity() {
    const MATURITY: BlockHeight = BlockHeight::new(1);
    const BLOCK_HEIGHT: BlockHeight = BlockHeight::new(2);

    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let rng = &mut StdRng::seed_from_u64(2322u64);
    let tx = TransactionBuilder::script(Some(op::ret(1)).into_iter().collect(), Default::default())
        .add_unsigned_coin_input(
            rng.gen(),
            rng.gen(),
            1,
            Default::default(),
            rng.gen(),
            Default::default(),
        )
        .gas_limit(100)
        .maturity(MATURITY)
        .finalize_checked(BLOCK_HEIGHT, &params, &gas_costs);

    let result = TestBuilder::new(2322u64).block_height(BLOCK_HEIGHT).execute_tx(tx);
    assert!(result.is_ok());
}
