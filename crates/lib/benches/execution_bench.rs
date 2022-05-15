use std::{env, fs::File, io::BufReader, time::Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use frost_snake_lib::{execute, Ledger, Transaction, TransactionExecutor, UCurrency};
use glob::glob;

pub fn execution_bench(c: &mut Criterion) {
    let cases = &[(100_0000, "tests/test-cases/100k-complex.input.csv")];

    let mut group = c.benchmark_group("execute");
    for (size, path) in cases {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), path, |b, path| {
            b.iter(|| {
                let reader = BufReader::new(File::open(path).unwrap());
                let writer = Vec::with_capacity(1024 * 1024 * 1024);
                execute(reader, writer)
            });
        });
    }
}

criterion_group!(benches, execution_bench);
criterion_main!(benches);
