use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use frost_snake_lib::{Ledger, Transaction, TransactionExecutor, UCurrency};

pub fn execution_bench(c: &mut Criterion) {
    let amount: UCurrency = UCurrency::from_num(123);

    c.bench_function("deposits all 1 client", |b| {
        b.iter_custom(|iters| {
            let mut ledger = Ledger::default();
            let start = Instant::now();
            for i in 1..iters {
                ledger = ledger
                    .clone()
                    .execute(Transaction::new_deposit(i as u32, 1, amount))
                    .unwrap()
            }
            start.elapsed()
        })
    });
    c.bench_function("withdrawals all 1 client", |b| {
        b.iter_custom(|iters| {
            let mut ledger = Ledger::default();
            for i in 1..iters {
                ledger = ledger
                    .clone()
                    .execute(Transaction::new_deposit(i as u32, 1, amount))
                    .unwrap()
            }
            let start = Instant::now();

            for i in 1..iters {
                ledger = ledger
                    .clone()
                    .execute(Transaction::new_withdrawal(i as u32, 1, amount))
                    .unwrap()
            }
            start.elapsed()
        })
    });

    c.bench_function("deposits 1 per client", |b| {
        b.iter_custom(|iters| {
            let mut ledger = Ledger::default();
            let start = Instant::now();
            for i in 1..iters {
                ledger = ledger
                    .clone()
                    .execute(Transaction::new_deposit(i as u32, i as u16, amount))
                    .unwrap()
            }
            start.elapsed()
        })
    });
}

criterion_group!(benches, execution_bench);
criterion_main!(benches);
