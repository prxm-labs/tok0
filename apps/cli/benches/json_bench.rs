use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tok0::compressors::system::json_cmd::filter_json;

const RAW: &str = include_str!("../tests/fixtures/system/json_huge_array_raw.txt");

fn bench(c: &mut Criterion) {
    c.bench_function("json_huge", |b| b.iter(|| filter_json(black_box(RAW))));
}

criterion_group!(benches, bench);
criterion_main!(benches);
