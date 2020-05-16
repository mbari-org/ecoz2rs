# ECOZ2 in Rust

This project is mainly a "front-end" to the original
[ecoz2](https://github.com/ecoz2/ecoz2) implementation in C,
with some functionality implemented in Rust.

## Installing and running

The `ecoz2` executable is currently being built and released for
Linux and MacOS, which you can find under
[releases](https://github.com/ecoz2/ecoz2rs/releases).

Alternatively, you can also install the executable using
[`Rust`](https://www.rust-lang.org/tools/install).
For this you will also need a GNU gcc compiler on your machine.
On Linux you can run:

    $ CC=gcc cargo install ecoz2

and on a MacOS, something like:

    $ CC=gcc-9 cargo install ecoz2

This may take some time to complete (example of output
[here](https://gist.github.com/carueda/0b4ede3e0152d3d670b0a0f2fc7098ce)).

Running:

    $ ecoz2
    ecoz2 0.3.35
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
in this general order:

- `ecoz2 lpc`:         takes `*.wav` and generates perdictor files `*.prd`
- `ecoz2 vq learn`     takes `*.prd` and generates codebooks `*.cb`
- `ecoz2 vq quantize`  takes `*.cb` and `*.prd` and generates observation sequences `*.seq`
- `ecoz2 hmm learn`    takes `*.seq` and generates an HMM model `*.hmm`
- `ecoz2 hmm classify` takes `*.hmm` and `*.seq` and reports classification
  of the sequences


## Development

[ecoz2](https://github.com/ecoz2/ecoz2) is included as a submodule,
with selected functionality exposed via
https://doc.rust-lang.org/cargo/reference/build-scripts.html.

    $ cargo build [--release]
