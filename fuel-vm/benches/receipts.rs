use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn receipts(c: &mut Criterion) {
    c.bench_function("markle receipt root", |b| b.iter(|| black_box(())));
    c.bench_function("accumulated receipt root", |b| b.iter(|| ()));
}

criterion_group!(benches, receipts);
criterion_main!(benches);
