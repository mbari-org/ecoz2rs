# ECOZ2 in Rust

This project is mainly a "front-end" to the original
[ecoz2](https://github.com/ecoz2/ecoz2) implementation in C,
with some functionality implemented in Rust.

## Installing and running

You will need a C compiler as well as
[`Rust`](https://www.rust-lang.org/tools/install)
on your machine.

    $ cargo install ecoz2
    
> If getting C compile errors, try `$ CC=c99 cargo install ecoz2`.

Running:

    $ ecoz2 --help
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


## Development

[ecoz2](https://github.com/ecoz2/ecoz2) is included as a submodule,
with selected functionality exposed via
https://doc.rust-lang.org/cargo/reference/build-scripts.html.

    $ cargo build
