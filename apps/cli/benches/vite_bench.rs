use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tok0::compressors::js::vite_cmd::{filter_vite_pub, ViteArgs};

const RAW_BUILD: &str = include_str!("../tests/fixtures/js/vite_build_raw.txt");
const RAW_DEV: &str = include_str!("../tests/fixtures/js/vite_dev_raw.txt");

fn bench_vite_build(c: &mut Criterion) {
    let args = ViteArgs::from_argv(vec!["build".into()]);
    c.bench_function("vite_build", |b| {
        b.iter(|| filter_vite_pub(black_box(&args), black_box(RAW_BUILD)))
    });
}

fn bench_vite_dev(c: &mut Criterion) {
    let args = ViteArgs::from_argv(vec![]);
    c.bench_function("vite_dev", |b| {
        b.iter(|| filter_vite_pub(black_box(&args), black_box(RAW_DEV)))
    });
}

criterion_group!(benches, bench_vite_build, bench_vite_dev);
criterion_main!(benches);
