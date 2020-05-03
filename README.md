# ECOZ2 in Rust

This project is mainly a "front-end" to the original
[ecoz2](https://github.com/ecoz2/ecoz2) implementation in C,
with some functionality implemented in Rust.

## Installing and running

You will need a gcc compiler and
[`Rust`](https://www.rust-lang.org/tools/install)
on your machine:

    $ cargo install ecoz2
    
This may take some time to complete (example of output
[here](https://gist.github.com/carueda/0b4ede3e0152d3d670b0a0f2fc7098ce)).

Running:

    $ ecoz2 help
    ecoz2 0.3.0
    ECOZ2 System
    
    USAGE:
        ecoz2 <SUBCOMMAND>
    
    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information
    
    SUBCOMMANDS:
        csv-show    Basic csv selection info
        cversion    Show version of C code
        help        Prints this message or the help of the given subcommand(s)
        hmm         HMM operations
        lpc         Linear prediction coding
        prd         Predictor file operations
        seq         Sequence file operations
        sgn         Signal operations
        vq          VQ operations

Starting with a set of acoustic signals (WAV format) on your machine,
the typical use of the system will involve the following main subcommands
in temporal order:

- `ecoz2 lpc`:         takes `*.wav` and generates `*.prd`
- `ecoz2 vq learn`     takes `*.prd` and generates `*.cb`
- `ecoz2 vq quantize`  takes `*.cb` and `*.prd` and generates `*.seq`
- `ecoz2 hmm learn`    takes `*.seq` and generates `*.hmm`
- `ecoz2 hmm classify` takes `*.hmm` and `*.seq` and reports classification
  of the sequences


## Development

[ecoz2](https://github.com/ecoz2/ecoz2) is included as a submodule,
with selected functionality exposed via
https://doc.rust-lang.org/cargo/reference/build-scripts.html.

    $ cargo build [--release]
