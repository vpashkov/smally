use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::Path;

// Note: These benchmarks require the model to be downloaded.
// Run `make model` first if not already done.

fn bench_tokenizer_encode(c: &mut Criterion) {
    let model_path = Path::new("models/all-MiniLM-L6-v2-onnx");

    // Check if model exists, skip benchmark if not
    if !model_path.exists() {
        eprintln!(
            "Model not found at {:?}. Run `make model` to download it.",
            model_path
        );
        return;
    }

    let tokenizer = match api::inference::tokenizer::Tokenizer::new(model_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to load tokenizer: {}. Skipping benchmark.", e);
            return;
        }
    };

    let mut group = c.benchmark_group("tokenizer_encode");

    // Short text (5 tokens)
    let short_text = "how to reset password";
    group.bench_with_input(BenchmarkId::new("short", 5), &short_text, |b, text| {
        b.iter(|| tokenizer.encode(black_box(text), true))
    });

    // Medium text (20 tokens)
    let medium_text =
        "how to reset my password and recover my account if I forgot my email address";
    group.bench_with_input(BenchmarkId::new("medium", 20), &medium_text, |b, text| {
        b.iter(|| tokenizer.encode(black_box(text), true))
    });

    // Long text (50 tokens)
    let long_text = "how to reset my password and recover my account if I forgot my email address and phone number. I need help accessing my account because I can't remember any of my security information and the recovery process is not working for me";
    group.bench_with_input(BenchmarkId::new("long", 50), &long_text, |b, text| {
        b.iter(|| tokenizer.encode(black_box(text), true))
    });

    group.finish();
}

fn bench_tokenizer_tokenize(c: &mut Criterion) {
    let model_path = Path::new("models/all-MiniLM-L6-v2-onnx");

    if !model_path.exists() {
        eprintln!("Model not found. Skipping benchmark.");
        return;
    }

    let tokenizer = match api::inference::tokenizer::Tokenizer::new(model_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to load tokenizer: {}. Skipping benchmark.", e);
            return;
        }
    };

    let mut group = c.benchmark_group("tokenizer_tokenize");

    let test_cases = vec![
        ("simple", "hello world"),
        ("punctuation", "Hello, world! How are you?"),
        ("mixed", "The quick brown fox jumps over the lazy dog."),
    ];

    for (name, text) in test_cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                // This tests the tokenization step
                tokenizer.encode(black_box(text), true)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_tokenizer_encode, bench_tokenizer_tokenize);
criterion_main!(benches);
