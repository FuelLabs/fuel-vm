use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};
use fuel_asm::{
    op,
    Instruction,
    RegId,
};
use fuel_tx::{
    Finalizable,
    GasCosts,
    Script,
    TransactionBuilder,
};
use fuel_types::{
    Immediate12,
    Word,
};
use fuel_vm::{
    interpreter::{
        Interpreter,
        InterpreterParams,
    },
    prelude::{
        IntoChecked,
        MemoryInstance,
        MemoryStorage,
    },
};

/// from; fuel-vm/src/tests/test_helpers.rs
/// Set a register `r` to a Word-sized number value using left-shifts
pub fn set_full_word(r: RegId, v: Word) -> Vec<Instruction> {
    let r = r.to_u8();
    let mut ops = vec![op::movi(r, 0)];
    for byte in v.to_be_bytes() {
        ops.push(op::ori(r, r, byte as Immediate12));
        ops.push(op::slli(r, r, 8));
    }
    ops.pop().unwrap(); // Remove last shift
    ops
}

fn meq_performance(c: &mut Criterion) {
    let benchmark_matrix = [
        1, 10, 100, 1000, 10_000, 50_000, 100_000, 500_000, 1_000_000, 2_000_000,
        2_500_000, 5_000_000, 10_000_000, 15_000_000, 20_000_000,
        // some exact multiples of 8 to verify alignment perf
        8, 16, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072,
        262144, 524288, 1048576, 2097152, 4194304, 8388608,
    ];

    for size in benchmark_matrix.iter() {
        let mut interpreter = Interpreter::<_, _, Script>::with_storage(
            MemoryInstance::new(),
            MemoryStorage::default(),
            InterpreterParams {
                gas_costs: GasCosts::free(),
                ..Default::default()
            },
        );

        let reg_len = RegId::new_checked(0x13).unwrap();

        let mut script = set_full_word(reg_len, *size as Word);
        script.extend(vec![
            op::cfe(0x13),
            op::meq(RegId::WRITABLE, RegId::ZERO, RegId::ZERO, reg_len),
            op::jmpb(RegId::ZERO, 0),
        ]);

        let tx_builder_script =
            TransactionBuilder::script(script.into_iter().collect(), vec![])
                .max_fee_limit(0)
                .add_fee_input()
                .finalize();
        let script = tx_builder_script
            .into_checked_basic(Default::default(), &Default::default())
            .unwrap();
        let script = script.test_into_ready();

        interpreter.init_script(script).unwrap();

        c.bench_function(&format!("meq_performance_{}", size), |b| {
            b.iter(|| {
                interpreter.execute().unwrap();
            });
        });
    }
}

criterion_group!(benches, meq_performance);
criterion_main!(benches);
