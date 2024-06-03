use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    Finalizable,
    GasCosts,
    Script,
    TransactionBuilder,
};
use fuel_vm::{
    checked_transaction::IntoChecked,
    interpreter::{
        InterpreterParams,
        MemoryInstance,
    },
    prelude::{
        Interpreter,
        MemoryStorage,
    },
};

fn execution(c: &mut Criterion) {
    let mut interpreter = Interpreter::<_, _, Script>::with_storage(
        MemoryInstance::new(),
        MemoryStorage::default(),
        InterpreterParams {
            gas_costs: GasCosts::free(),
            ..Default::default()
        },
    );
    let script = TransactionBuilder::script(
        vec![
            op::meq(RegId::new(0x10), RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::jmpb(RegId::ZERO, 0),
        ]
        .into_iter()
        .collect(),
        vec![],
    )
    .max_fee_limit(0)
    .add_random_fee_input()
    .finalize();
    let script = script
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap();
    let script = script.test_into_ready();
    black_box(interpreter.init_script(script)).unwrap();

    let mut group_execution = c.benchmark_group("execution");

    group_execution.bench_function("Infinite `meq` loop", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let _ = interpreter.execute().unwrap();
            }
        })
    });

    group_execution.bench_function("Infinite `meq` loop black box", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(interpreter.execute()).unwrap();
            }
        })
    });

    group_execution.bench_function("Infinite `meq` loop unsafe", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                unsafe {
                    let dummy = interpreter.execute().unwrap();
                    let ret = std::ptr::read_volatile(&dummy);
                    ret
                };
            }
        })
    });

    let script = TransactionBuilder::script(
        vec![
            op::add(RegId::new(0x10), RegId::ZERO, RegId::ONE),
            op::jmpb(RegId::ZERO, 0),
        ]
        .into_iter()
        .collect(),
        vec![],
    )
    .max_fee_limit(0)
    .add_random_fee_input()
    .finalize();
    let script = script
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap();
    let script = script.test_into_ready();
    black_box(interpreter.init_script(script)).unwrap();

    group_execution.bench_function("Infinite `add` loop", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let _ = interpreter.execute().unwrap();
            }
        })
    });

    group_execution.bench_function("Infinite `add` loop black box", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(interpreter.execute()).unwrap();
            }
        })
    });

    group_execution.bench_function("Infinite `add` loop unsafe", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                unsafe {
                    let dummy = interpreter.execute().unwrap();
                    let ret = std::ptr::read_volatile(&dummy);
                    ret
                };
            }
        })
    });

    let script = TransactionBuilder::script(
        vec![
            op::not(RegId::new(0x10), RegId::ZERO),
            op::jmpb(RegId::ZERO, 0),
        ]
        .into_iter()
        .collect(),
        vec![],
    )
    .max_fee_limit(0)
    .add_random_fee_input()
    .finalize();
    let script = script
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap();
    let script = script.test_into_ready();
    black_box(interpreter.init_script(script)).unwrap();

    group_execution.bench_function("Infinite `not` loop", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let _ = interpreter.execute().unwrap();
            }
        })
    });

    group_execution.bench_function("Infinite `not` loop black box", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(interpreter.execute()).unwrap();
            }
        })
    });

    group_execution.bench_function("Infinite `not` loop unsafe", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                unsafe {
                    let dummy = interpreter.execute().unwrap();
                    let ret = std::ptr::read_volatile(&dummy);
                    ret
                };
            }
        })
    });

    group_execution.finish();
}

criterion_group!(benches, execution);
criterion_main!(benches);
