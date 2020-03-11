extern crate cc;

fn main() {
    cc::Build::new()
        .flag("-g")
        .flag("-O3")
        .flag("-Wall")
        .include("../ecoz2/src/include")
        .include("../ecoz2/src/sgn")
        .files(&[
            "../ecoz2/src/utl/fileutil.c",
            "../ecoz2/src/utl/list.c",
            "../ecoz2/src/utl/memutil.c",
            "../ecoz2/src/lpc/lpca.c",
            "../ecoz2/src/lpc/prd.c",
            "../ecoz2/src/lpc/ref2raas.c",
            "../ecoz2/src/vq/vq_learn.c",
            "../ecoz2/src/vq/vq_quantize.c",
            "../ecoz2/src/hmm/symbol.c",
            "../ecoz2/src/vq/vq.c",
            "../ecoz2/src/vq/distortion.c",
            "../ecoz2/src/vq/report.c",
            "../ecoz2/src/vq/sigma.c",
            "../ecoz2/src/vq/inertia.c",
            "../ecoz2/src/vq/quantize.c",
        ])
        .compile("ecoz_lib");
}
