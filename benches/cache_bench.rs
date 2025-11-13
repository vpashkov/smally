use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use api::cache::lru::LruCache;

fn bench_lru_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_put");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut cache = LruCache::new(size);
            let mut counter = 0;

            b.iter(|| {
                cache.put(black_box(counter), black_box(vec![0.1f32; 384]));
                counter += 1;
            });
        });
    }
    group.finish();
}

fn bench_lru_get_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_get_hit");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut cache = LruCache::new(size);

            // Pre-populate cache
            for i in 0..size {
                cache.put(i, vec![0.1f32; 384]);
            }

            b.iter(|| {
                let key = black_box(size / 2);
                cache.get(&key)
            });
        });
    }
    group.finish();
}

fn bench_lru_get_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_get_miss");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut cache = LruCache::new(size);

            // Pre-populate cache
            for i in 0..size {
                cache.put(i, vec![0.1f32; 384]);
            }

            b.iter(|| {
                let key = black_box(size + 1000);
                cache.get(&key)
            });
        });
    }
    group.finish();
}

fn bench_lru_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_mixed_workload");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut cache = LruCache::new(size);

            // Pre-populate cache
            for i in 0..size {
                cache.put(i, vec![0.1f32; 384]);
            }

            let mut counter = size;
            b.iter(|| {
                // 70% reads (50% hit, 20% miss), 30% writes
                let op = counter % 10;
                if op < 5 {
                    // Hit
                    let key = black_box(counter % size);
                    cache.get(&key)
                } else if op < 7 {
                    // Miss
                    let key = black_box(counter);
                    cache.get(&key)
                } else {
                    // Write
                    cache.put(black_box(counter), black_box(vec![0.1f32; 384]));
                    None
                };
                counter += 1;
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_lru_put,
    bench_lru_get_hit,
    bench_lru_get_miss,
    bench_lru_mixed_workload
);
criterion_main!(benches);
