#[macro_use]
extern crate criterion;

use criterion::{black_box, Bencher, Criterion};
use futures::channel::oneshot as futures_oneshot;

fn create_futures(b: &mut Bencher) {
    b.iter(|| futures_oneshot::channel::<u32>());
}

fn create_catty(b: &mut Bencher) {
    b.iter(|| catty::oneshot::<u32>());
}

fn oneshot_futures(b: &mut Bencher) {
    b.iter(|| {
        let (tx, rx) = futures_oneshot::channel();
        tx.send(black_box(10u32)).unwrap();
        assert_eq!(pollster::block_on(rx), Ok(10));
    });
}

fn oneshot_catty(b: &mut Bencher) {
    b.iter(|| {
        let (tx, rx) = catty::oneshot();
        tx.send(black_box(10u32)).unwrap();
        assert_eq!(pollster::block_on(rx), Ok(10));
    });
}

fn create(c: &mut Criterion) {
    c.bench_function("create-futures", |b| create_futures(b));
    c.bench_function("create-catty", |b| create_catty(b));
}

fn oneshot(c: &mut Criterion) {
    c.bench_function("oneshot-futures", |b| oneshot_futures(b));
    c.bench_function("oneshot-catty", |b| oneshot_catty(b));
}

criterion_group!(compare, create, oneshot);
criterion_main!(compare);
