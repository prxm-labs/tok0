use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tok0::compressors::js::prisma_cmd::{filter_prisma, PrismaArgs};

const RAW: &str = include_str!("../tests/fixtures/js/prisma_migrate_raw.txt");

fn bench_prisma_migrate(c: &mut Criterion) {
    let args = PrismaArgs::from_argv(vec!["prisma".into(), "migrate".into()]);
    c.bench_function("prisma_migrate", |b| {
        b.iter(|| filter_prisma(black_box(&args), black_box(RAW)))
    });
}

criterion_group!(benches, bench_prisma_migrate);
criterion_main!(benches);
