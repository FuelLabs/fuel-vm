use crate::context::Context;

use super::*;

use criterion::Criterion;

/// Benchmark the sha256 calculation.
pub fn bench_sha256(c: &mut Criterion) {
    let mut memory: Box<[u8; VM_MEMORY_SIZE]> = vec![0u8; VM_MEMORY_SIZE].try_into().unwrap();
    let mut pc = 0;
    c.bench_function("sha256", |b| {
        b.iter(|| {
            let owner = OwnershipRegisters {
                sp: 700,
                ssp: 1,
                hp: 900,
                prev_hp: 1000,
                context: Context::Script { block_height: 0 },
            };
            sha256(&mut memory, owner, RegMut::new(&mut pc), 10, 400, 100).unwrap();
            pc = 0;
        })
    });
}
