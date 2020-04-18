extern crate criterion;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "../src/lpc/lpca.rs"]
mod lpca;

use lpca::{lpca, lpca_load_input};

fn criterion_benchmark(c: &mut Criterion) {
    let input = lpca_load_input("signal_frame.inputs").unwrap();
    let frame = black_box(input.x);
    let prediction_order = black_box(input.p);

    let mut vector = vec![0f64; prediction_order + 1];
    let mut reflex = vec![0f64; prediction_order + 1];
    let mut pred = vec![0f64; prediction_order + 1];

    c.bench_function("lpca", |b| {
        b.iter(|| {
            lpca(
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
