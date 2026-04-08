use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tok0::compressors::js::next_cmd::{filter_next_pub, NextArgs};

const RAW_BUILD: &str = include_str!("../tests/fixtures/js/next_build_raw.txt");
const RAW_DEV: &str = include_str!("../tests/fixtures/js/next_dev_raw.txt");

fn bench_next_build(c: &mut Criterion) {
    let args = NextArgs::from_argv(vec!["build".into()]);
    c.bench_function("next_build", |b| {
        b.iter(|| filter_next_pub(black_box(&args), black_box(RAW_BUILD)))
    });
}

fn bench_next_dev(c: &mut Criterion) {
    let args = NextArgs::from_argv(vec![]);
    c.bench_function("next_dev", |b| {
        b.iter(|| filter_next_pub(black_box(&args), black_box(RAW_DEV)))
    });
}

criterion_group!(benches, bench_next_build, bench_next_dev);
criterion_main!(benches);
