use criterion::{
    Criterion,
    black_box,
    criterion_group,
    criterion_main,
};
use fuel_types::{
    bytes::Bytes,
    canonical::{
        Deserialize,
        Serialize,
    },
};

#[derive(serde::Serialize, serde::Deserialize, Serialize, Deserialize)]
enum ReceiptWithVector {
    Call { data: Vec<u8> },
}

impl ReceiptWithVector {
    fn new(data: Vec<u8>) -> Self {
        Self::Call { data }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Serialize, Deserialize)]
enum ReceiptWithBytes {
    Call { data: Bytes },
}

impl ReceiptWithBytes {
    fn new(data: Vec<u8>) -> Self {
        Self::Call {
            data: Bytes::new(data),
        }
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    const SIZE: usize = 100000;

    let vec: Vec<u8> = (0..=127u8).cycle().take(SIZE).collect();

    let vec = black_box(vec);

    c.bench_function("canonical_vector_serialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(original.to_bytes());
        });
    });

    c.bench_function("canonical_bytes_serialize", |b| {
        let input = ReceiptWithBytes::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(original.to_bytes());
        });
    });

    c.bench_function("canonical_vector_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = original.to_bytes();
        b.iter(|| {
            let _: ReceiptWithVector =
                black_box(Deserialize::from_bytes(&serialized)).unwrap();
        });
    });

    c.bench_function("canonical_bytes_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = original.to_bytes();
        b.iter(|| {
            let _: ReceiptWithBytes =
                black_box(Deserialize::from_bytes(&serialized)).unwrap();
        });
    });

    c.bench_function("serde_bincode_vector_serialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(bincode::serialize(original)).unwrap();
        });
    });

    c.bench_function("serde_bincode_bytes_serialize", |b| {
        let input = ReceiptWithBytes::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(bincode::serialize(original)).unwrap();
        });
    });

    c.bench_function("serde_bincode_vector_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = bincode::serialize(original).unwrap();
        b.iter(|| {
            let _: ReceiptWithVector =
                black_box(bincode::deserialize(&serialized)).unwrap();
        });
    });

    c.bench_function("serde_bincode_bytes_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = bincode::serialize(original).unwrap();
        b.iter(|| {
            let _: ReceiptWithBytes =
                black_box(bincode::deserialize(&serialized)).unwrap();
        });
    });

    c.bench_function("serde_postcard_vector_serialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(postcard::to_stdvec(original)).unwrap();
        });
    });

    c.bench_function("serde_postcard_bytes_serialize", |b| {
        let input = ReceiptWithBytes::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(postcard::to_stdvec(original)).unwrap();
        });
    });

    c.bench_function("serde_postcard_vector_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = postcard::to_stdvec(original).unwrap();
        b.iter(|| {
            let _: ReceiptWithVector =
                black_box(postcard::from_bytes(&serialized)).unwrap();
        });
    });

    c.bench_function("serde_postcard_bytes_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = postcard::to_stdvec(original).unwrap();
        b.iter(|| {
            let _: ReceiptWithBytes =
                black_box(postcard::from_bytes(&serialized)).unwrap();
        });
    });

    c.bench_function("serde_json_vector_serialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(serde_json::to_vec(original)).unwrap();
        });
    });

    c.bench_function("serde_json_bytes_serialize", |b| {
        let input = ReceiptWithBytes::new(vec.clone());
        let original = black_box(&input);
        b.iter(|| {
            let _ = black_box(serde_json::to_vec(original)).unwrap();
        });
    });

    c.bench_function("serde_json_vector_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = serde_json::to_vec(original).unwrap();
        b.iter(|| {
            let _: ReceiptWithVector =
                black_box(serde_json::from_slice(&serialized)).unwrap();
        });
    });

    c.bench_function("serde_json_bytes_deserialize", |b| {
        let input = ReceiptWithVector::new(vec.clone());
        let original = black_box(&input);
        let serialized = serde_json::to_vec(original).unwrap();
        b.iter(|| {
            let _: ReceiptWithBytes =
                black_box(serde_json::from_slice(&serialized)).unwrap();
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
