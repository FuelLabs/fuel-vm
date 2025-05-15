use criterion::{
    Criterion,
    black_box,
    criterion_group,
    criterion_main,
};
use fuel_asm::{
    GTFArgs,
    RegId,
    op,
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

fn epar(c: &mut Criterion) {
    let mut interpreter = Interpreter::<_, _, Script>::with_storage(
        MemoryInstance::new(),
        MemoryStorage::default(),
        InterpreterParams {
            gas_costs: GasCosts::free(),
            ..Default::default()
        },
    );

    #[rustfmt::skip]
    let script = vec![
        // Get the point, scalar and expected result from the script data
        op::gtf_args(0x10, 0x00, GTFArgs::ScriptData),
        // Store the number of batchs in 0x11
        op::movi(0x11, 0x02),
        // Perform multiplication of the two points
        op::epar(0x12, RegId::ZERO, 0x11, 0x10),
        // keep running epar
        op::jmpb(RegId::ZERO, 1),
    ].into_iter().collect();

    // Batch 1 + Batch 2
    let mut script_data = Vec::new();
    // Batch 1
    script_data.extend(
        hex::decode(
            "\
        1c76476f4def4bb94541d57ebba1193381ffa7aa76ada664dd31c16024c43f59\
        3034dd2920f673e204fee2811c678745fc819b55d3e9d294e45c9b03a76aef41\
        209dd15ebff5d46c4bd888e51a93cf99a7329636c63514396b4a452003a35bf7\
        04bf11ca01483bfa8b34b43561848d28905960114c8ac04049af4b6315a41678\
        2bb8324af6cfc93537a2ad1a445cfd0ca2a71acd7ac41fadbf933c2a51be344d\
        120a2a4cf30c1bf9845f20c6fe39e07ea2cce61f0c9bb048165fe5e4de877550",
        )
        .unwrap(),
    );
    // Batch 2
    script_data.extend(
        hex::decode(
            "\
        111e129f1cf1097710d41c4ac70fcdfa5ba2023c6ff1cbeac322de49d1b6df7c\
        2032c61a830e3c17286de9462bf242fca2883585b93870a73853face6a6bf411\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa",
        )
        .unwrap(),
    );

    let script = TransactionBuilder::script(script, script_data)
        .max_fee_limit(0)
        .add_fee_input()
        .finalize();

    let script = script
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap();
    let script = script.test_into_ready();
    black_box(interpreter.init_script(script)).unwrap();

    let mut group_execution = c.benchmark_group("epar");
    group_execution.measurement_time(std::time::Duration::from_secs(100));

    group_execution.bench_function("test_vector", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let _ = interpreter.execute::<false>().unwrap();
            }
        })
    });

    group_execution.finish();
}

criterion_group!(benches, epar);
criterion_main!(benches);
