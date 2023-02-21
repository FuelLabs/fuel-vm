//! Bench mark multiple mutable array borrows.
use std::mem::{self, MaybeUninit};

use criterion::{black_box, Criterion};

use super::*;

/// Benchmark different strategies for splitting registers.
pub fn bench_split_reg(c: &mut Criterion) {
    let mut registers = [0; VM_REGISTER_COUNT];
    c.bench_function("split_registers_half", |b| {
        b.iter(|| {
            let (
                ReadRegisters {
                    zero,
                    one,
                    of,
                    pc,
                    err,
                    ggas,
                    cgas,
                    flag,
                    ..
                },
                _,
            ) = split_registers(&mut registers);
            black_box(zero);
            black_box(one);
            black_box(of);
            black_box(pc);
            black_box(err);
            black_box(ggas);
            black_box(cgas);
            black_box(flag);
        })
    });
    c.bench_function("split_registers_two", |b| {
        b.iter(|| {
            let (ReadRegisters { one, flag, .. }, _) = split_registers(&mut registers);
            black_box(one);
            black_box(flag);
        })
    });
    let indices = [15, 12, 5, 4, 3, 2, 1, 0];
    c.bench_function("split_at_mut_half", |b| {
        b.iter(|| {
            let mut out: [MaybeUninit<&mut u64>; 8] = unsafe { MaybeUninit::uninit().assume_init() };
            let mut rest = &mut registers[..];
            for (index, out) in indices.iter().zip(out.iter_mut()) {
                let (r, i) = rest.split_at_mut(*index);
                out.write(&mut i[0]);
                rest = r;
            }

            unsafe { mem::transmute::<_, [&mut u64; 8]>(out) }
        })
    });
    let indices = [15, 3];
    c.bench_function("split_at_mut_2", |b| {
        b.iter(|| {
            let mut out: [MaybeUninit<&mut u64>; 2] = unsafe { MaybeUninit::uninit().assume_init() };
            let mut rest = &mut registers[..];
            for (index, out) in indices.iter().zip(out.iter_mut()) {
                let (r, i) = rest.split_at_mut(*index);
                out.write(&mut i[0]);
                rest = r;
            }

            unsafe { mem::transmute::<_, [&mut u64; 2]>(out) }
        })
    });
}
