extern crate criterion;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "../src/ecoz2_lib/lpca_c.rs"]
mod lpca_c;

#[path = "../src/lpc/lpca_rs.rs"]
mod lpca_rs;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn lpca_load_input(filename: &str) -> Result<lpca_rs::LpcaInput, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let inputs = serde_cbor::from_reader(br)?;
    Ok(inputs)
}

fn criterion_benchmark(c: &mut Criterion) {
    let input = lpca_load_input("signal_frame.inputs").unwrap();
    let frame = black_box(input.x);
    let prediction_order = black_box(input.p);

    let mut vector = vec![0f64; prediction_order + 1];
    let mut reflex = vec![0f64; prediction_order + 1];
    let mut pred = vec![0f64; prediction_order + 1];

    c.bench_function("lpca_rs", |b| {
        b.iter(|| {
            lpca_rs::lpca(
                &frame,
                prediction_order,
                &mut vector,
                &mut reflex,
                &mut pred,
            )
        })
    });

    c.bench_function("lpca_c", |b| {
        b.iter(|| {
            lpca_c::lpca(
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
