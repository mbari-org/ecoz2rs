# ECOZ2 in Rust

This project is mainly a "front-end" to the original
[ecoz2](https://github.com/ecoz2/ecoz2) implementation in C,
with some functionality implemented in Rust.

## Linking with C

[ecoz2](https://github.com/ecoz2/ecoz2) is included as a submodule,
with selected functionality exposed via
https://doc.rust-lang.org/cargo/reference/build-scripts.html.

## Build

    $ cargo build --release
    
## Run

    $ target/release/ecoz2 --help
    ecoz2 0.1.0
    ECOZ2 System
    
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
        prd         Predictor file operations
        seq         Sequence file operations
        sgn         Signal operations
        vq          VQ operations
