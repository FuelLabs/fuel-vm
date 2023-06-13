use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};

#[cfg(feature = "unsafe")]
use criterion::black_box;
#[cfg(feature = "unsafe")]
use fuel_types::bytes::from_slice_unchecked;

#[cfg(not(feature = "unsafe"))]
pub fn criterion_benchmark(_: &mut Criterion) {}

#[cfg(feature = "unsafe")]
pub fn criterion_benchmark(c: &mut Criterion) {
    use criterion::BenchmarkId;
    use fuel_types::Bytes32;

    let mem = vec![0u8; 1_000_000];
    c.bench_function("from_slice_unchecked_1", |b| {
        b.iter(|| {
            let slice = unsafe { from_slice_unchecked::<1>(&mem) };
            black_box(slice);
        })
    });
    c.bench_function("from_slice_unchecked_10", |b| {
        b.iter(|| {
            let slice = unsafe { from_slice_unchecked::<10>(&mem) };
            black_box(slice);
        })
    });
    c.bench_function("from_slice_unchecked_100", |b| {
        b.iter(|| {
            let slice = unsafe { from_slice_unchecked::<100>(&mem) };
            black_box(slice);
        })
    });
    c.bench_function("from_slice_unchecked_1_000", |b| {
        b.iter(|| {
            let slice = unsafe { from_slice_unchecked::<1_000>(&mem) };
            black_box(slice);
        })
    });

    c.bench_function("Bytes32_from_bytes_ref", |b| {
        b.iter(|| {
            let mem: &[u8; 32] = (&mem[..32]).try_into().unwrap();
            let bytes: &Bytes32 = Bytes32::from_bytes_ref(mem);
            black_box(bytes);
        })
    });
    c.bench_function("raw_try", |b| {
        b.iter(|| {
            let mem: &[u8; 32] = (&mem[..32]).try_into().unwrap();
            black_box(mem);
        })
    });
    c.bench_function("raw_unsafe", |b| {
        b.iter(|| {
            let ptr = mem.as_ptr() as *const [u8; 32];
            black_box(unsafe { &*ptr });
        })
    });
    c.bench_function("raw_slice", |b| {
        b.iter(|| {
            let mem: &[u8] = &mem[..32];
            black_box(mem);
        })
    });
    let size = 64 * 1024 * 1024;
    let mut memory = vec![0u8; size];

    for i in [
        1, 4, 16, 64, 512, 8192, 32768, 131072, 1048576, 16777216, 33554431,
    ] {
        c.bench_with_input(BenchmarkId::new("copy_nonover", i), &i, |b, i| {
            b.iter(|| {
                let src = &memory[0_usize] as *const u8;
                let dst = &mut memory[size / 2_usize] as *mut u8;

                unsafe {
                    std::ptr::copy_nonoverlapping(src, dst, *i);
                }
            })
        });
    }
    for i in [
        1, 4, 16, 64, 512, 8192, 32768, 131072, 1048576, 16777216, 33554431,
    ] {
        c.bench_with_input(BenchmarkId::new("copy", i), &i, |b, i| {
            b.iter(|| {
                memory.copy_within(0..*i, size / 2_usize);
            })
        });
    }
    for i in [
        1, 4, 16, 64, 512, 8192, 32768, 131072, 1048576, 16777216, 33554431,
    ] {
        c.bench_with_input(BenchmarkId::new("copy_split", i), &i, |b, i| {
            b.iter(|| {
                let (a, b) = memory.split_at_mut(size / 2_usize);
                a[0..*i].copy_from_slice(&b[0..*i]);
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
