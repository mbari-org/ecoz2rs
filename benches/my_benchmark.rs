extern crate criterion;

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

#[path = "../src/lpc/lpca_rs.rs"]
mod lpca_rs;

/*
   input length=1440, prediction_order=36
   lpca/lpca1_rs           time:   [49.762 µs 50.067 µs 50.391 µs]
   lpca/lpca2_rs           time:   [49.010 µs 49.342 µs 49.693 µs]

   These predate lpca3, which uses algebraic float ops and is the one in use.
   The former `lpca_c` arm compared against the C's -ffast-math build; it went
   away with the C submodule in v0.8.0.
*/
fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("lpca");
    let input = lpca_rs::lpca_load_input("signal_frame.inputs").unwrap();
    let frame = black_box(input.x);
    let prediction_order = black_box(input.p);

    println!(
        "input length={}, prediction_order={}",
        frame.len(),
        prediction_order
    );

    let mut vector = vec![0f64; prediction_order + 1];
    let mut reflex = vec![0f64; prediction_order + 1];
    let mut pred = vec![0f64; prediction_order + 1];

    group.bench_function("lpca1_rs", |b| {
        b.iter(|| {
            lpca_rs::lpca1(
                &frame,
                prediction_order,
                &mut vector,
                &mut reflex,
                &mut pred,
            )
        })
    });

    group.bench_function("lpca2_rs", |b| {
        b.iter(|| {
            lpca_rs::lpca2(
                &frame,
                prediction_order,
                &mut vector,
                &mut reflex,
                &mut pred,
            )
        })
    });

    group.bench_function("lpca3_rs", |b| {
        b.iter(|| {
            lpca_rs::lpca3(
                &frame,
                prediction_order,
                &mut vector,
                &mut reflex,
                &mut pred,
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
