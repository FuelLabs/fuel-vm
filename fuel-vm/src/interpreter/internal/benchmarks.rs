use super::*;

use criterion::BenchmarkId;
use criterion::Criterion;

/// Benchmark the receipt root calculation.
pub fn bench_set_receipt_root(c: &mut Criterion) {
    let mut memory: Box<[u8; VM_MEMORY_SIZE]> = vec![0u8; VM_MEMORY_SIZE].try_into().unwrap();
    let receipt = Receipt::call(
        Default::default(),
        Default::default(),
        0,
        Default::default(),
        0,
        0,
        0,
        0,
        0,
    );
    let receipts = vec![
        vec![receipt.clone(); 1],
        vec![receipt.clone(); 10],
        vec![receipt.clone(); 50],
        vec![receipt.clone(); 100],
        vec![receipt; 1000],
    ];
    let mut script = Script::default();
    for input in receipts {
        c.bench_with_input(BenchmarkId::new("set_receipt_root", input.len()), &input, |b, input| {
            b.iter(|| {
                set_receipt_root(0, &mut memory, input, &mut script);
            })
        });
    }
}
