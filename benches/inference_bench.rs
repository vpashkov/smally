use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// Note: These benchmarks require:
// 1. Model files downloaded (`make model`)
// 2. Environment variables set (MODEL_PATH, etc.)
//
// The EmbeddingModel::new() reads from config/environment,
// so we can't easily instantiate it without full setup.
//
// For now, these benchmarks are designed to work when the
// full application environment is configured.

fn bench_embedding_generation(c: &mut Criterion) {
    // Check if model directory exists
    let model_path = std::path::Path::new("models/all-MiniLM-L6-v2-onnx");
    if !model_path.exists() {
        eprintln!(
            "Model not found at {:?}. Run `make model` to download it.",
            model_path
        );
        eprintln!("Skipping inference benchmarks.");
        return;
    }

    // Try to create model - requires environment setup
    let mut model = match api::inference::EmbeddingModel::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load model: {}.", e);
            eprintln!("Make sure environment variables are set (MODEL_PATH, etc.)");
            eprintln!("Skipping inference benchmarks.");
            return;
        }
    };

    let mut group = c.benchmark_group("embedding_generation");

    // Test with different text lengths
    let test_cases = vec![
        ("short_5tok", "how to reset password"),
        ("medium_20tok", "how to reset my password and recover my account if I forgot my email address"),
        ("long_50tok", "how to reset my password and recover my account if I forgot my email address and phone number. I need help accessing my account because I can't remember any of my security information"),
    ];

    for (name, text) in test_cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| model.encode(black_box(text), black_box(true)))
        });
    }

    group.finish();
}

fn bench_normalize_impact(c: &mut Criterion) {
    let model_path = std::path::Path::new("models/all-MiniLM-L6-v2-onnx");
    if !model_path.exists() {
        eprintln!("Model not found. Skipping benchmark.");
        return;
    }

    let mut model = match api::inference::EmbeddingModel::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load model: {}. Skipping benchmark.", e);
            return;
        }
    };

    let mut group = c.benchmark_group("normalize_impact");

    let text = "how to reset my password and recover my account";

    group.bench_function("without_normalize", |b| {
        b.iter(|| model.encode(black_box(text), black_box(false)))
    });

    group.bench_function("with_normalize", |b| {
        b.iter(|| model.encode(black_box(text), black_box(true)))
    });

    group.finish();
}

criterion_group!(benches, bench_embedding_generation, bench_normalize_impact);
criterion_main!(benches);
