extern crate cc;

fn main() {
    let flags = &["-g", "-O3", "-Wall"];

    let headers = &[
        "../ecoz2/src/include",
        "../ecoz2/src/sgn",
        "../ecoz2/src/hmm",
    ];

    let files = &[
        "../ecoz2/src/utl/fileutil.c",
        "../ecoz2/src/utl/list.c",
        "../ecoz2/src/utl/memutil.c",
        "../ecoz2/src/lpc/lpca.c",
        "../ecoz2/src/lpc/prd.c",
        "../ecoz2/src/lpc/ref2raas.c",
        "../ecoz2/src/vq/vq_learn.c",
        "../ecoz2/src/vq/vq_quantize.c",
        "../ecoz2/src/vq/vq_show.c",
        "../ecoz2/src/vq/vq.c",
        "../ecoz2/src/vq/distortion.c",
        "../ecoz2/src/vq/report.c",
        "../ecoz2/src/vq/sigma.c",
        "../ecoz2/src/vq/inertia.c",
        "../ecoz2/src/vq/quantize.c",
        "../ecoz2/src/hmm/hmm.c",
        "../ecoz2/src/hmm/hmm_learn.c",
        "../ecoz2/src/hmm/hmm_classify.c",
        "../ecoz2/src/hmm/hmm_show.c",
        "../ecoz2/src/hmm/hmm_adjustb.c",
        "../ecoz2/src/hmm/hmm_file.c",
        "../ecoz2/src/hmm/hmm_refinement.c",
        "../ecoz2/src/hmm/hmm_log_prob.c",
        "../ecoz2/src/hmm/hmm_genQopt.c",
        "../ecoz2/src/hmm/hmm_estimateB.c",
        "../ecoz2/src/hmm/hmm_gen.c",
        "../ecoz2/src/hmm/distr.c",
        "../ecoz2/src/hmm/symbol.c",
    ];

    // https://doc.rust-lang.org/cargo/reference/build-scripts.html#rerun-if-changed
    for f in headers {
        println!("cargo:rerun-if-changed={}", f)
    }
    for f in files {
        println!("cargo:rerun-if-changed={}", f)
    }

    let mut build = cc::Build::new();

    for f in flags {
        build = build.flag(f).to_owned();
    }

    for f in headers {
        build = build.include(f).to_owned();
    }

    build.files(files).compile("ecoz2_lib");
}
