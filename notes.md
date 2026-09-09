# ECOZ2 in Rust — porting notes

## 2026-09: porting the C implementation to Rust

Branch: `2026-09_port_to_rust`.

Until now this crate has been a front-end to the C
[ecoz2](https://github.com/ecoz2/ecoz2), with a few operations reimplemented in
Rust. The plan is to finish the job and retire the C submodule.

### Why now

The reason for keeping the C was performance, specifically `-ffast-math`, which
Rust had no equivalent for (see the older "fast-math" notes below). Rust 1.98
stabilized the algebraic floating-point methods, and `lpca3` now runs the
dominant autocorrelation loop ~3.4x faster than the C on aarch64
(7.30 µs vs 24.82 µs; see CHANGELOG 2026-08).

That removes the one technical reason for keeping the C. This is a performance
argument only: the C's use of `-ffast-math` has never been a correctness
problem here, and the code is structurally safe under it — see "Validation
basis" below, where that safety turns out to be what makes the old build usable
as an oracle at all.

What Rust adds is per-expression control rather than a whole-program flag,
which is how `lpca3` is written: algebraic ops in the O(n·p) autocorrelation,
strict IEEE in the O(p²) Levinson-Durbin recursion. Useful discipline going
forward, but not a defect being fixed; the criterion is under "Decisions
taken".

### Scope

Only 40 C files are actually compiled (see `build.rs`); `ecoz2/src/x/*` and
`sgn/endpoint.c` are not.

| area | C LOC | state on the Rust side |
|---|---:|---|
| `ecoz2/ecoz2.c` (FFI shim, RNG seed) | 303 | deleted, not ported |
| `utl/` (list, memutil, fileutil, utl) | 454 | ~90% deleted (`Vec`, `Box`, `walkdir`); `src/utl` already covers the rest |
| `sgn/sgn.c` + `dr_wav.h` (3727) | 104 | **done** — `hound` in `src/sgn` |
| `lpc/` (lpca, lpa_on_signal, lpc_signals, prd, ref2raas, prd_show) | 663 | largely done: `lpca3`, `lpca_r_rs`, `lpca_cepstrum_rs`, `lpc_rs`, `libpar` |
| `vq/` (+ `vq_learn.i`) | 1694 | not started |
| `hmm/` (+ `hmm_refinement.c`) | 2347 | not started |
| **total compiled** | **5565** | |

So roughly 1500 lines evaporate or are already written, and ~4000 lines are the
real port — about 1700 VQ and 2300 HMM, a good fraction of which is `printf`
reporting that maps onto the existing `src/c12n`.

### Beyond the port itself

The FFI layer and the C support code simply go away; no need to enumerate that.
Two consequences are worth planning for, though, because they are decisions
rather than deletions:

- **The fixed caps become choices.** `MAX_SEQS 4096`, `MAX_MODELS 256`,
  `MAX_CODEBOOK_SIZE 4096`, `MAX_PREDICTION_ORDER 200`, and the ~6.5 MB of
  static arrays in `vq_learn.i` sized for the maxima regardless of need. In
  Rust these are dynamic; decide per case whether to keep a limit and where.
- **OpenMP becomes `rayon`.** Only four sites: `lpa_on_signal:119`,
  `vq_learn_par`, `hmm_classify:441/457`, and the `hmm_learn:81` reduction.

Dropping `openmp-sys`, the gcc requirement, the submodule and `build.rs` is
what removes the C toolchain from building and installing this crate, and what
makes the PyO3 binding sketched in the older notes stop being a two-language
build problem.

### Validation basis (phase 0 — done)

The harness lives in the sibling repo, `ecoz2-whale/exerc07-port-validation`.
It reproduces exerc06 as a differential test: `run.sh --impl c|rs` into separate
trees, `compare.sh` between them or against the stored 2020 artifacts.

`exerc06/data/` is gitignored but ~978 MB survives locally — all codebooks, all
quantized sequences, 1540 HMM combinations with their `.rpt` seeds, and 1540
classification CSVs. Only signals and predictors are gone, which is exactly what
`run.sh` regenerates. So the port has a six-year-old oracle built by a different
compiler on a different architecture, for free.

Re-running the whole C pipeline against it (full tier, 4539 instances, 2m19s):

| stage | scale | agreement with 2020 |
|---|---:|---|
| sequences | 36,312 files / 2,978,976 symbols | **100.000000%** — zero differences |
| hmm models | 8 | 1.1e-15 relative, with the per-class `.rpt` seeds |
| codebooks | 108 | 5.9e-9 relative, worst at large M |
| c12n TEST | 910 | **100%** full `r1…r8` agreement |

That the C reproduces this tightly is not luck, and it is worth naming as an
asset rather than a curiosity: the code is structurally safe under
`-ffast-math`. It never tests for or produces NaN/Inf, its min-search sentinels
are `DBL_MAX` rather than `INFINITY`, `hmm_log_prob` controls underflow by
scaling rather than by relying on IEEE special values, and `hmm_epsilon` floors
every `B` entry so `logl` never sees zero. Relaxed arithmetic therefore has
little to act on beyond rounding, which is why a 2020 build and a 2026 build on
a different ISA still agree to 1e-15. The port depends on exactly that
stability — an oracle that drifted with the compiler would not be usable as
one — so the same discipline should carry over to the Rust code rather than
being treated as an artifact of the old implementation.

Two things this settles:

- **Tolerances are measured, not guessed.** 1e-8 is the honest floor for
  codebooks; anything tighter fails C-vs-C. And the codebooks differing at
  5.9e-9 moved **no symbol at all**, which is why `vq quantize` carries the
  pass/fail weight: its output is discrete, so drift either flips a symbol at a
  cell boundary or it does not. When `lpca3` reaches this pipeline, the symbol
  count and the resulting accuracy delta are the answer to "does the port change
  the science".
- **Seed everything, per class.** `hmm learn` initializes from `rand()`. The
  2020 run trained classes in parallel with no `-s`, so each recorded its own
  seed. One global seed reproduces only the class whose seed you picked and
  leaves the other seven at O(1e2) relative — while their `.rpt` files still
  show matching sequence and refinement counts, so the output looks fine.
  Unseeded, the same hyperparameters differ by 14/910 top-1 labels and 0.65 pp
  accuracy. The TRAIN/TEST split matters the same way: `util split` shuffles.

HMM models must never be compared parameter-wise: two trainings at different
seeds differ by max rel 2.6e3 while classifying within 1.5% of each other.
Judge that stage by its classification output.

### Phases

| phase | work | status |
|---|---|---|
| 0 | golden corpus + differential harness; `utl::cfmt` readers; `util cmp` | **done** |
| 1 | LPC all-Rust: `libpar`/`lpc_rs` onto `lpca3`, port `lpc_signals`, **write the C-compatible `.prd`** | next |
| 2 | VQ: LBG/Juang, quantize, classify, report; `rayon` for `vq_learn_par` | |
| 3 | HMM: Baum-Welch, scaled forward-backward, Viterbi, `estimateB`, B-epsilon | |
| 4 | delete FFI, `build.rs`, submodule, `openmp-sys`; revisit packaging | |

Order follows the data flow, so a hybrid pipeline always runs and each phase
diffs against the C. Phase 3 is the delicate one: the scaling and log-domain
arithmetic is where `-ffast-math` has been silently doing us favours.

Phase 1 **must** write the traditional `.prd` format. Today `lpc --zrs` writes
`serde_cbor` via `utl::save_ser`, so the Rust LPC path cannot feed the C VQ
path — closing that fork is a prerequisite for incremental migration, not a
nicety.

After phase 4, keep the C behind an off-by-default `c-oracle` Cargo feature:
`build.rs` stays for differential tests, but nobody building or installing the
tool needs gcc.

### Decisions taken

- **Where algebraic float ops are allowed.** Relax arithmetic in reductions over
  independent data that dominate the runtime; keep recurrences, convergence
  tests, and anything feeding a discrete decision on strict IEEE. This is what
  `lpca3` does — the autocorrelation is ~105k flops and reassociates freely,
  while the Levinson-Durbin recursion is ~2.5% of the work, has no
  reassociation freedom anyway (it is a dependency chain), and ends in the
  `pe <= 0.0` breakdown guard whose output is a discrete status code. The point
  is predictability, not accuracy: contracting `1.0 - akk*akk` into an FMA is
  actually *more* accurate, but the algebraic methods are non-deterministic by
  design and may contract differently across rustc versions and targets, so a
  guard could trip on a different frame between builds.

  Applying the rule to what is left: in VQ, the `distortion()` inner loop is the
  hot reduction and relaxes, but the `if (dd < ddmin)` argmin must not — it *is*
  the emitted symbol, i.e. the very quantity the harness measures the port with
  — and neither must the `(DDprv - DD)/DD < eps` convergence test. In HMM,
  relax essentially nothing: N is 3-10 so the forward-backward inner sums have
  no headroom, the stage is memory-bound over B rather than flop-bound, and the
  scaling factors are the whole basis of the Shen-2008 formulation's stability.
  Spend the effort there on `rayon` across sequences instead.
- **Determinism.** Port RNG use to `rand`'s `ChaCha`/`StdRng`. `rand()`/`srand()`
  are libc-specific, so seeded runs are *already* not reproducible across
  platforms; this is a fix, not a risk.
- **File formats: later, deliberately.** Keep reading the traditional formats
  at least for time being.
  Once there is a single writer, add an opt-in modern one — `.prd` and
  `.cbook` are plain 2-D f64 matrices, i.e. exactly `.npy`/`.npz`, whose header
  carries dtype and shape. Doing this after the port means changing one
  implementation instead of two. Note the current formats have no version field,
  no endianness marker, and no record of the `prob_t` width the writer used, so
  a mismatched build misreads silently.
- **`target-cpu` when benchmarking.** `build.rs` passes `-march=native` to the
  C while a plain `cargo build --release` leaves Rust at the baseline, so use
  `just release-native`, or set `-C target-cpu` explicitly, for every comparison.
  (What the *released* binaries should be built with is a separate
  question; leave it until the port works locally.)

### Backlog (small, found while building the harness)

- `vq learn --max-codebook-size`: the ladder always doubles to
  `MAX_CODEBOOK_SIZE`. Today you kill it and resume with `-B`, which is lossless
  — each size is saved on convergence and resuming from M=256 reproduces
  512…4096 byte-identically. But `prepare_report` opens with `"w"`, so a resumed
  run truncates `eps_<ε>.rpt`/`.rpt.csv` to only the sizes it produced. Add the
  stop flag; make the report append on resume.
- `-m` means two different things: instances in the label file for
  `sgn extract`, signal files present for `lpc`. Unify.
- Fixed: `sgn extract --time-ranges` containment test was inverted, and giving
  both range filters discarded the selection verdict. This is already-pure-Rust
  code that the C oracle never covered — the Rust-only parts need their own
  tests, not just differential ones.

---

## Earlier notes: initial exploration (2020)

What follows is the exploration that led here, kept for context.

Some preliminary notes/exercises toward a possible
implementation of the ECOZ2 programs in Rust.

### To keep in mind

- https://rust-lang.github.io/rust-bindgen/

- https://nnethercote.github.io/perf-book/

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

        $ export CC=gcc-10
        
    **NOTE** Let's first do regular runs (no rust impl):
    
        $ cargo run lpc -P 36 -W 45 -O 15 --signals ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
        lpc_signals: number of classes: 1
        class 'HBSe_20161221T010133': 1
        
        $ cargo run prd show --from 0 --to 10 data/predictors/HBSe_20161221T010133/HBSe_20161221T010133.prd
        # data/predictors/HBSe_20161221T010133/HBSe_20161221T010133.prd:
        # className='HBSe_20161221T010133', T=38265, P=36
        r0,r1,r2,r3,r4,r5,r6,r7,r8,r9,r10
        2.35985,0.32431,-0.69968,0.00268,-0.32025,0.03768,0.00825,-0.05656,-0.00750,-0.17555,0.03140
        2.39831,0.24925,-0.73353,0.11278,-0.35405,0.00366,0.07970,-0.13980,-0.00649,-0.03235,0.04081
        ...
        2.65369,0.71388,-0.66782,-0.36442,-0.59910,-0.04919,0.34036,0.26143,-0.00044,-0.13380,-0.01206
        2.89329,0.94659,-0.47174,-0.37060,-0.76588,-0.15328,0.22067,0.18169,0.02488,-0.25885,-0.23922
        
    **Now with Rust impl** (`--zrs` option):
    
        $ cargo run lpc --zrs -P 36 -W 45 -O 15 --signals ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
        Loading: ../ecoz2-whale-cb/HBSe_20161221T010133/HBSe_20161221T010133.wav
        num_samples: 18368474  sample_rate: 32000  bits_per_sample: 16  sample_format = Int
        lpa_on_signal: p=36 numSamples=18368474 sampleRate=32000 winSize=1440 offset=480 num_frames=38265
        saving lpca inputs, frame=21
          38265 total frames processed
          SER lpa_on_signal complete: 38265 vectors
        predictor.prd saved.  Class: '_':  38265 vectors
        
        $ cargo run prd show --zrs --from 0 --to 10 predictor.prd
        # predictor.prd
        # class_name='_', T=38265 P=36
        r0,r1,r2,r3,r4,r5,r6,r7,r8,r9,r10
        2.35985,0.32431,-0.69968,0.00268,-0.32025,0.03768,0.00825,-0.05656,-0.00750,-0.17555,0.03140
        2.39831,0.24925,-0.73353,0.11278,-0.35405,0.00366,0.07970,-0.13980,-0.00649,-0.03235,0.04081
        ...
        2.65369,0.71388,-0.66782,-0.36442,-0.59910,-0.04919,0.34036,0.26143,-0.00044,-0.13380,-0.01206
        2.89329,0.94659,-0.47174,-0.37060,-0.76588,-0.15328,0.22067,0.18169,0.02488,-0.25885,-0.23922

    **Note**: same output.


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
    
### Performance (2020)

Note these predate `lpca3`; the ratio is now reversed. With the `lpc` program
re-implemented in Rust, here's a basic performance comparison: 

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
