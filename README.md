# ECOZ2 in Rust

A pure Rust implementation of the ECOZ2 system for pattern recognition on
acoustic signals: linear predictive coding, vector quantization, and hidden
Markov modeling.

Through v0.7.x this project was a front-end to the original
[ecoz2](https://github.com/ecoz2/ecoz2) implementation in C, which it linked in
as a submodule. As of v0.8.0 every stage of the pipeline is implemented in
Rust and the C is gone; see [CHANGELOG.md](CHANGELOG.md). The file formats are
unchanged, so artifacts produced by either implementation remain interchangeable.

## Installing and running

The `ecoz2` executable is built and released for Linux and macOS, which you can
find under [releases](https://github.com/ecoz2/ecoz2rs/releases).

Alternatively, install it with [Rust](https://www.rust-lang.org/tools/install).
No C compiler is needed:

    $ cargo install ecoz2

Running:

    $ ecoz2
    ...
    USAGE:
        ecoz2 <SUBCOMMAND>

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    SUBCOMMANDS:
        csv-show    Basic csv selection info
        help        Prints this message or the help of the given subcommand(s)
        hmm         HMM operations
        lpc         Linear prediction coding
        mm          MM operations
        nb          NBayes operations
        prd         Predictor file operations
        seq         Sequence file operations
        sgn         Signal operations
        util        Utilities
        vq          VQ operations

Starting with a set of acoustic signals (WAV format) on your machine,
the typical use of the system will involve the following main subcommands
in this general order:

- `ecoz2 lpc`:         takes `*.wav` and generates predictor files `*.prd`
- `ecoz2 vq learn`     takes `*.prd` and generates codebook files `*.cbook`
- `ecoz2 vq quantize`  takes `*.cbook` and `*.prd` and generates observation sequences `*.seq`
- `ecoz2 hmm learn`    takes `*.seq` and generates an HMM model `*.hmm`
- `ecoz2 hmm classify` takes `*.hmm` and `*.seq`, performs classification
  of the sequences and reports the results

`ecoz2 util cmp` compares two artifacts, or two trees of them, which is how the
Rust implementation was validated against the C's output stage by stage.

## Development

    $ cargo build [--release]
    $ cargo install --path .

See the [justfile](justfile) for the test, format, clippy and benchmark recipes.
