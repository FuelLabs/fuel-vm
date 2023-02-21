use super::*;

use criterion::{black_box, Criterion};

/// Benchmark the alu clear.
pub fn bench_alu_clear(c: &mut Criterion) {
    let mut of = 1;
    let mut err = 1;
    let mut pc = 4;
    let mut dest = 1;
    c.bench_function("alu_clear", |b| {
        b.iter(|| {
            let common = AluCommonReg {
                of: RegMut::new(&mut of),
                err: RegMut::new(&mut err),
                pc: RegMut::new(&mut pc),
            };
            alu_set(&mut dest, common, 10).unwrap();
            black_box(err);
            black_box(of);
        })
    });
}
