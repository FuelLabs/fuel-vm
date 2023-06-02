use criterion::{criterion_group, criterion_main, Criterion};
use fuel_asm::{op, RegId};
use fuel_tx::{ConsensusParameters, Finalizable, Input, TransactionBuilder, UtxoId};
use fuel_vm::checked_transaction::{EstimatePredicates, IntoChecked};
use fuel_vm::gas::GasCosts;
use fuel_vm::interpreter::Interpreter;
use fuel_vm::prelude::PredicateStorage;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn predicate(c: &mut Criterion) {
    let predicate = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();
    let owner = Input::predicate_owner(&predicate, &ConsensusParameters::DEFAULT.chain_id);

    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();
    let mut builder = TransactionBuilder::script(vec![], vec![]);
    builder.gas_limit(10000000);

    for i in 0..100 {
        let coin_predicate = Input::coin_predicate(
            UtxoId::new([0; 32].into(), i),
            owner,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            predicate.clone(),
            Default::default(),
        );

        builder.add_input(coin_predicate);
    }

    let mut tx = builder.finalize();

    tx.estimate_predicates(&params, &gas_costs)
        .expect("Estimation is valid");

    let checked = tx
        .into_checked_basic(Default::default(), &params)
        .expect("The test transaction is valid");

    let checked = criterion::black_box(checked);

    c.bench_function("Check predicate", |b| {
        b.iter(|| {
            Interpreter::<PredicateStorage>::check_predicates(&checked, params, gas_costs.clone())
                .expect("Should be valid");
        })
    });
}

criterion_group!(benches, predicate);
criterion_main!(benches);
