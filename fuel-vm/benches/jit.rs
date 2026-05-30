//! Microbenchmark: JIT vs interpreter on straight-line ALU workloads.
//!
//! Runs a full script through `Interpreter::transact` (which drives the
//! `run_program` loop where the JIT hook lives), toggling the JIT on/off in-process
//! via `set_jit_enabled` for a clean A/B comparison. Requires `--features jit`.
//!
//! Run with: `cargo bench -p fuel-vm --features jit --bench jit`

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use fuel_asm::{Instruction, RegId, op};
use fuel_tx::{Finalizable, GasCosts, Script, TransactionBuilder};
use fuel_vm::{
    checked_transaction::IntoChecked,
    interpreter::{InterpreterParams, MemoryInstance},
    prelude::{Interpreter, MemoryStorage},
};

/// A long straight-line block of mixed ALU ops (no control flow, no overflow) so the
/// JIT can compile it as a single native function. Ends with RET.
fn alu_script(n_ops: usize) -> Vec<Instruction> {
    let mut ops = Vec::with_capacity(n_ops + 8);
    // Seed a few writable registers with non-trivial values.
    let r0 = 0x10;
    let r1 = 0x11;
    let r2 = 0x12;
    let r3 = 0x13;
    ops.push(op::movi(r0, 0x1234));
    ops.push(op::movi(r1, 0x5678));
    ops.push(op::movi(r2, 0x9abc));
    ops.push(op::movi(r3, 0x0def));
    // Straight-line ALU body. Small immediates avoid u64 overflow (which would bail).
    for i in 0..n_ops {
        match i % 8 {
            0 => ops.push(op::addi(r0, r0, 1)),
            1 => ops.push(op::xori(r1, r1, 0x55)),
            2 => ops.push(op::ori(r2, r2, 0x0f)),
            3 => ops.push(op::andi(r3, r3, 0x07ff)),
            4 => ops.push(op::add(r0, r0, r1)),
            5 => ops.push(op::slli(r2, r2, 1)),
            6 => ops.push(op::srli(r1, r1, 1)),
            _ => ops.push(op::move_(r3, r0)),
        }
        // Periodically reset r0 to keep `add` from overflowing into a bail.
        if i % 64 == 63 {
            ops.push(op::movi(r0, 0x1234));
        }
    }
    ops.push(op::ret(RegId::ONE));
    ops
}

fn build_interpreter() -> Interpreter<MemoryInstance, MemoryStorage, Script> {
    Interpreter::<_, _, Script>::with_storage(
        MemoryInstance::new(),
        MemoryStorage::default(),
        InterpreterParams {
            gas_costs: GasCosts::free(),
            ..Default::default()
        },
    )
}

fn ready_script(ops: Vec<Instruction>) -> fuel_vm::checked_transaction::Ready<Script> {
    let script = TransactionBuilder::script(ops.into_iter().collect(), vec![])
        .max_fee_limit(0)
        .add_fee_input()
        .finalize();
    script
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap()
        .test_into_ready()
}

fn jit_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit_vs_interp");
    for &n in &[64usize, 256, 1024, 4096] {
        let ops = alu_script(n);
        let ready = ready_script(ops);
        let mut interp = build_interpreter();

        interp.set_jit_enabled(false);
        group.bench_with_input(BenchmarkId::new("interp", n), &n, |b, _| {
            b.iter(|| {
                interp.transact(ready.clone()).unwrap();
            });
        });

        interp.set_jit_enabled(true);
        // Warm the block cache once.
        interp.transact(ready.clone()).unwrap();
        group.bench_with_input(BenchmarkId::new("jit", n), &n, |b, _| {
            b.iter(|| {
                interp.transact(ready.clone()).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, jit_bench);
criterion_main!(benches);
