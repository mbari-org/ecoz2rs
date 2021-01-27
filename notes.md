## Implementation in Rust

Some preliminary notes/exercises toward a possible
implementation of the ECOZ2 programs in Rust.

### To keep in mind

- for python binding
    - https://github.com/PyO3/pyo3
    - https://github.com/PyO3/maturin
    - https://www.reddit.com/r/rust/comments/fxe99l/cffi_vs_cpython_vs_pyo3_what_should_i_use/fmvgonr/
    - http://jakegoulding.com/rust-ffi-omnibus/

- `long double` was used since the origins of the C implementation to represent
   probabilities in HMM models.
   Although this has been changed to `double` more recently, worth noting the
   following:

    - Not yet a `f128` type in Rust - https://github.com/rust-lang/rfcs/issues/2629
    - "long double becomes u128" - https://github.com/rust-lang/rust-bindgen/issues/1549
    - https://users.rust-lang.org/t/are-there-any-floating-types-with-precision-beyond-that-of-f64/50601/2
    - https://github.com/jkarns275/f128/ (but "in maintenance mode")
    
- https://jvns.ca/blog/2017/12/21/bindgen-is-awesome/

#### fast-math

(From 2020-04 notes in changelog)

Lack of "fast-math" in Rust is what explains the big performance discrepancy
between the C and Rust impls.

- first, I added "-ffast-math" to the C build here upon noting `ecoz lpc ...`
  surprisingly slow compared to the direct execution of the C generated binary
  (where `-ffast-math` has been used since the origins of the ECOZ software).
  Now `lpc ...` (the binary from the C project) and `ecoz2 lpc ...`
  (the binary from this Rust project, but using the C impl) are similarly
  performant as expected.
  `cargo bench` results now also make more sense: the mean execution time
  of lpca for the C impl is ~3.7 times faster than before.
   
- from this I then of course realized that this fast-math feature is not
  enabled for Rust, thus explaining the difference in performance: 
    - https://internals.rust-lang.org/t/pre-rfc-whats-the-best-way-to-implement-ffast-math/5740
    - https://github.com/rust-lang/rust/issues/21690
    - https://www.reidatcheson.com/hpc/architecture/performance/rust/c++/2019/10/19/measure-cache.html
    - https://stackoverflow.com/questions/7420665/what-does-gccs-ffast-math-actually-do

- for now, for the Rust impl of the lpc subcommand I'm enabling the C impl of the lpca operation.
  
- and perhaps that could be a general approach with the Rust impl while "fast-math" is not available:
  general code in Rust and critical operations in C.
 

#### Initial exploration

Note: the following ran as shown but may not be readily buildable/runnable
as I moved things around a bit and focused on other stuff later on. 

- [src/lpc/lpc_rs.rs](src/lpc/lpc_rs.rs) is `lpc` implemented in rust.
   Generates predictor file serialized with
  `serde_cbor`, that is, not compatible with traditional format in ecoz2.
  
- [src/prd/lib_rs.rs](src/prd/lib_rs.rs)
  displays predictor file generated with `lpc` above:

        $ cargo run lpc -P 36 -W 45 -O 15 -f ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
        Signal loaded: ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
        num_samples: 18368474  sample_rate: 32000  bits_per_sample: 16  sample_format = Int
        lpa_on_signal: p=36 numSamples=18368474 sampleRate=32000 winSize=1440 offset=480 t=38265
          15000 frames processed
          30000 frames processed
          38265 frames processed
        predictor.prd saved.  Class: '_':  38265 vector sequences
        
        $ cargo run -- prd-show -f predictor.prd
            Finished dev [optimized + debuginfo] target(s) in 0.03s
             Running `target/debug/ecoz2 prd-show -f predictor.prd`
        Predictor loaded: predictor.prd
         class_name = '_' prediction_order: 36 sequences: 38265

- `sgn-show` implemented in rust:

        $ cargo run -- sgn-show -f ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
            Finished dev [optimized + debuginfo] target(s) in 0.12s
             Running `target/debug/ecoz2 sgn-show -f ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav`
        Signal loaded: ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
        num_samples: 18368474  sample_rate: 32000  bits_per_sample: 16  sample_format = Int
    
- `vq-learn` links with C code. Note in particular that the predictor files
  for input need to come from the traditional `lpc` program:
  
        $ cargo run -- vq-learn data/predictors/HBSe_20161221T010133/HBSe_20161221T010133.prd 
        Codebook generation:
        
        38265 training vectors (ε=0.05)
        Report: data/codebooks/_/eps_0.05.rpt
        data/codebooks/_/eps_0.05_M_0002.cbook
        (4)	DP=0.330561	DDprv=12784.2e+3DD=12648.9=751200.0106969      e+303
        data/codebooks/_/eps_0.05_M_0004.cbook
        (2)	DP=0.285617	DDprv=11173.5	DD=10929.1	0.022362
        data/codebooks/_/eps_0.05_M_0008.cbook
        (3)	DP=0.232681	DDprv=9153.62	DD=8903.56	0.0280858
        data/codebooks/_/eps_0.05_M_0016.cbook
        (4)	DP=0.166335	DDprv=6567.81	DD=6364.82	0.0318917
        data/codebooks/_/eps_0.05_M_0032.cbook
        (2)	DP=0.141507	DDprv=5644.19	DD=5414.75	0.0423718
        data/codebooks/_/eps_0.05_M_0064.cbook
        (3)	DP=0.114893	DDprv=4489.99	DD=4396.36	0.0212976
        data/codebooks/_/eps_0.05_M_0128.cbook
        (2)	DP=0.100817	DDprv=3993.95	DD=3857.76	0.0353041
        data/codebooks/_/eps_0.05_M_0256.cbook
        (2)	DP=0.0888497	DDprv=3495.66	DD=3399.83	0.0281852
        data/codebooks/_/eps_0.05_M_0512.cbook
        (2)	DP=0.0781936	DDprv=3093.34	DD=2992.08	0.0338451
        data/codebooks/_/eps_0.05_M_1024.cbook
        (2)	DP=0.0684865	DDprv=2709.19	DD=2620.64	0.0337901
        data/codebooks/_/eps_0.05_M_2048.cbook
        (0)	DP=0.0664449	DDprv=2620.64	DD=2542.51	0.0307272
        WARN: review_cells: 18 empty cell(s) for codebook size 2048)
        (1)	DP=0.0607904	DDprv=2542.51	DD=2326.14	0.0930165
        WARN: review_cells: 17 empty cell(s) for codebook size 2048)
        (2)	DP=0.0587476	DDprv=2326.14	DD=2247.98	0.0347719
        WARN: review_cells: 17 empty cell(s) for codebook size 2048)
    
## Performance    

With the `lpc` program re-implemented in Rust, here's a basic performance comparison: 

rust:

    $ cargo build --release
    $ time target/release/ecoz2 lpc --file ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
    Signal loaded: ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
    num_samples: 18368474  sample_rate: 32000  bits_per_sample: 16  sample_format = Int
    lpa_on_signal: p=36 numSamples=18368474 sampleRate=32000 winSize=1440 offset=480 t=38265
      15000 frames processed
      30000 frames processed
      38265 frames processed
    predictor.prd saved.  Class: '_':  38265 vector sequences
    target/release/ecoz2 lpc --file   2.78s user 0.08s system 98% cpu 2.897 total

c:
    
    $ time lpc -P 36 -W 45 -O 15  ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
    Number of classes: 1
    class 'HBSe_20161221T010133': 1
      ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
    lpa_on_signal: P=36 numSamples=18368474 sampleRate=32000 winSize=1440 offset=480 T=38265
    data/predictors/HBSe_20161221T010133/HBSe_20161221T010133.prd: 'HBSe_20161221T010133': predictor saved
    
    lpc -P 36 -W 45 -O 15   0.78s user 0.12s system 98% cpu 0.912 total
    
So, 2.9secs vs. < ~1sec.

----
[src/lpc/libpar.rs](src/lpc/libpar.rs): initial attempt to parallelize the LP analysis 
