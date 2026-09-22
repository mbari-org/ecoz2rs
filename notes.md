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
| 1 | LPC all-Rust: `libpar`/`lpc_rs` onto `lpca3`, port `lpc_signals`, write the C-compatible `.prd` | **done** |
| 2 | VQ: LBG/Juang, quantize, classify, report; `rayon` for `vq_learn_par` | **done** |
| 3 | HMM: Baum-Welch, scaled forward-backward, Viterbi, `estimateB`, B-epsilon | **done** |
| 4 | port `hmm classify --predictors`; delete FFI, `build.rs`, submodule, `openmp-sys` | **done** — see "Phase 4 result" |

Order follows the data flow, so a hybrid pipeline always runs and each phase
diffs against the C. Phase 3 is the delicate one: the scaling and log-domain
arithmetic is where `-ffast-math` has been silently doing us favors.

Phase 1 closed the format fork: `lpc --zrs` used to write `serde_cbor`, so the
Rust LPC path could not feed the C VQ path. It now writes the traditional
`<predictor>` format, and `prd::load` reads it, so the two implementations are
interchangeable at that boundary.

`--impl rs` in the harness runs the C for anything not yet ported rather than
skipping it, so a complete classification result is available at every phase and
each one can be judged on what it does to the final numbers.

The `c-oracle` plan below — keeping the C behind an off-by-default Cargo
feature — was **dropped** at phase 4. With the differential harness having
validated every stage, a retained oracle bought little against the cost of
keeping `build.rs`, the submodule and gcc alive in the tree. The C remains
available where it always was: [ecoz2](https://github.com/ecoz2/ecoz2), and in
this crate's own history up to v0.7.5.

### Phase 1 result

`lpc --zrs` now runs `lpca3` and writes the traditional format. Measured on the
quick tier (910 signals, 8 classes), Rust LPC against C LPC with everything
downstream held identical:

| stage | scale | difference |
|---|---:|---|
| predictors | 910 files | max rel **5.5e-10** |
| sequences, M=32…256 | 3640 files / 341,916 symbols | **zero symbol flips** |
| hmm models | 8 | exactly 0 |
| classification, TRAIN and TEST | 910 | 100% full-ranking agreement, **+0.00 pp** |

So the algebraic-float change perturbs the predictors at the 1e-10 level and
changes nothing downstream at all.

**One qualitative finding, which matters for phase 2.** The codebooks at M≥2048
differ in a few dozen positions across 108 files (56 on one split, 80 on
another — it depends on which cells end up near-tied) — and every one of them is
a *pairwise transposition*: both codewords are present in both runs, swapped,
matching to 3.5e-14, with their cardinalities exchanged. These are near-tied
sibling cells from `grow_codebook`, each holding a single training vector, and a
5e-10 nudge flips which one wins it.

The codebook is therefore the same model with two labels exchanged. But the
symbol indices it emits are not the same, so this is not automatically benign:
it is harmless within a self-consistent pipeline and breaks any comparison that
crosses runs. `ecoz2 util cmp` now distinguishes "rows reordered" from "rows
changed" and reports the counts; `--allow-permutation` (or `ALLOW_PERM=1` in
`compare.sh`) accepts it, off by default. Phase 2 should expect this whenever it
touches VQ, and not read a raw symbol-agreement drop as a modeling difference
without checking for it first.

### Phase 2 result (VQ)

All of VQ is ported: `vq learn`, `vq quantize`, `vq classify` and `vq show`,
each behind `--zrs`.

`vq classify` reproduces the C's confusion matrix cell for cell, with identical
per-class accuracy and candidate-order counts; only the column padding differs,
which is the existing `c12n` formatting that `nb` and `mm` already share.
`vq show` output is byte-identical, `%g` formatting included.

**Agreement.** `vq learn`'s report table matches the C on all twelve codebook
sizes — passes, DDprm, σ and inertia, every row. The codebooks themselves agree
to 3.6e-10, and the full Rust VQ path (learn then quantize) is symbol-identical
to the full C path:

| test | scale | result |
|---|---:|---|
| Rust quantize vs C quantize, M=32…256 | 4 × 85,479 symbols | zero flips |
| Rust quantize vs the **stored 2020 sequences**, using the stored 2020 codebooks | 372,372 symbols at M=2048 and M=4096 | zero flips |
| Rust learn + Rust quantize vs C learn + C quantize | 4 × 85,479 symbols | zero flips |

The middle row is the sharper one: it exercises the quantizer in isolation
against a six-year-old C oracle, with the codebook held fixed.

No codeword transpositions show up when both `vq learn` runs are given the
*same* predictors — no near-tied cell has anything to tip it. They reappear as
soon as the input changes: running the whole pipeline on Rust through quantize
(Rust predictors into Rust learn) gives 72 transpositions at M≥2048, and still
zero symbol flips at the sizes used downstream. So the phase 1 finding holds,
and `ALLOW_PERM=1` is the right setting for any end-to-end comparison that
retrains a codebook.

End to end, C versus Rust-through-quantize on the quick tier: sequences
identical over 341,916 symbols, HMM models bit-identical, classification
identical on TRAIN and TEST, +0.00 pp.

**Performance**, quick tier, 68,219 training vectors, 16 cores:

| | C (OpenMP) | Rust (rayon) | |
|---|---:|---:|---|
| `vq learn`, ladder to M=4096 | 6.87 s real / 46.7 s user | **3.44 s** / 35.1 s | 2.0x wall, 1.33x less CPU |
| `vq quantize`, M=4096 | 1.67 s / 1.55 s | **1.14 s** / 1.05 s | 1.5x, single-threaded both |
| `lpc`, full tier | 2 s | 1 s | |

Building with `-C target-cpu=native` changes none of this here (3.93 s vs
3.44 s real on `vq learn`, identical user time): baseline aarch64 already has
NEON and FMA, so there is nothing for it to unlock. See the revised note under
"Decisions taken".

**The C's `vq learn` depends on the core count.** It partitions the training
vectors round-robin across `omp_get_max_threads()` and sums the per-thread
partials in thread order, so the reduction order — and the codebook — changes
with the machine. Measured on the quick tier: 8.4e-11 relative between 1 and 4
threads, 9.3e-11 between 1 and 16, growing with M. No RNG is involved; this is
purely reduction order, and it is a better explanation of the 5.9e-9 codebook
gap against the 2020 oracle than "LBG float noise" was.

The port fixes this rather than reproducing it: the Rust version splits the
vectors into a *fixed* number of chunks, independent of available parallelism,
and merges them in chunk order. `RAYON_NUM_THREADS=1` and `=16` give
byte-identical codebooks. That is also why it is not bit-identical to the C, and
should not be.

**Two details that had to be right.** `pert0`/`pert1` in `grow_codebook` are
`float` literals multiplied into a `double`, so the values actually applied are
`0.99f32 as f64` and `1.01f32 as f64`; using the f64 literals would be 9.5e-9
off and would send the whole ladder down a different path. And the codebook in
the autocorrelation domain is derived from the reflections only at init and at
each growth — inside the loop `calculate_reflections` writes each new entry
straight from the predictor and never round-trips back through `lpca_rc`.

Also landed: `vq learn --max-codebook-size`, the backlog item, since the ladder
was being written fresh anyway.

### Phase 3 result (HMM)

All of HMM is ported: `hmm learn`, `hmm classify` and `hmm show`. Every stage of
the pipeline now runs on Rust.

`hmm classify` reproduces the **stored 2020 c12n CSV exactly** — all 910
sequences, full `r1…r8` ranking, identical accuracy — which validates the scaled
forward recursion against a six-year-old oracle. `hmm show` output is identical
to the C's on every model tried.

Two notes on fidelity. The C accumulates the log-likelihood with `logl`, which
is `long double`: 80-bit on x86, but plain `double` on aarch64, so the C itself
is not consistent across targets there. The port uses `f64::ln` everywhere.

This is a leftover rather than a design choice. `prob_t` was changed to `double`
long ago, but the long-double *math functions* the type change did not touch
stayed: `logl` at `hmm_prob.c:127`, `hmm_log_prob.c:74`, `hmm_genQopt.c:16,28,71`
and `hmm_learn.c:245`, and `fabsl` at eight more sites. The `fabsl` calls have no
effect at all — absolute value is exact at any precision — and once the scaling
factors were introduced there was nothing left for extended precision to protect
in the `logl` ones either. The remaining `(long double)` casts are printf
arguments required by the `%Lg`/`%Le` conversions and compute nothing.

So this is the C's *second* residual platform dependency, alongside `vq learn`
varying with the core count: its HMM log-probabilities differ by ULPs between
x86 and aarch64. Not worth fixing in code that is being retired — changing
`logl` to `log` now would perturb the oracle mid-port for no gain — but worth
knowing when comparing runs made on different machines. And
`hmm_show_model` hands its `--format` argument straight to `printf` alongside a
`long double`, which lets the caller inject an arbitrary conversion; the port
parses the subset in use (`utl::pf`) and rejects the rest.

Two bugs found, both pre-existing and unrelated to the port:

- `ecoz2 hmm show` **panicked on every invocation**. `--hmm` derived a short
  `-h`, which clap rejects as conflicting with help. Confirmed at HEAD before
  any phase 3 change; fixed by dropping the short form.
- The FFI wrappers for `vq show` and `hmm show` print a `codebook_filename = …`
  / `hmm_show: …` line before calling into C. That is shim chatter, not part of
  the C's own output, and the ported versions drop it. It also means an earlier
  "byte-identical" claim about `vq show` was measured before `--zrs` was wired;
  the correct statement is that the port matches `vq_show.c`'s output, and the
  C *path* additionally emits that one wrapper line.

#### hmm learn, and how it was validated

Training is the one stage that cannot be compared bit for bit, because the C
seeds `pi`, `A` and its fallback rows from `rand()`. So it was split in two.

**Model type 1 (uniform) uses no randomness at all**, which makes the whole
algorithm directly comparable. Across twelve combinations — N ∈ {2,3,5},
M ∈ {128,256}, up to 12 refinements, four classes — the Rust and C models agree
to **1e-14 relative**, and the Σ log(P) trace is identical at every iteration.
That covers initialization, `estimateB` (Viterbi plus frequency counting),
`adjust_B_epsilon`, the alpha/beta/gamma passes, both re-estimations, the
convergence test and the writer. Only the RNG is left uncovered.

**For type 3 (what the exercises use) the comparison is behavioral**, and the
right yardstick is the C's own variance under reseeding, already measured at
14/910 top-1 labels and 0.65 pp. Full Rust pipeline against full C:

| | difference |
|---|---|
| top-1 labels | 11 of 910 (1.21%) |
| accuracy | +0.54 pp on TEST, −0.28 pp on TRAIN |
| C vs C, reseeded (for scale) | 14 of 910 (1.54%), 0.65 pp |

So the port differs from the C by *less* than the C differs from itself when
reseeded. Two independent Rust runs with the same seeds are bit-identical.

One consequence for the harness: with `hmm learn` on Rust, the per-class seeds
in the stored `.rpt` files no longer reproduce the stored models — they are
`rand()` seeds. They still give reproducibility, just to a different stream.
Comparing a Rust run against the 2020 oracle is therefore only exact up to the
sequences; past that it has to be read against the reseeding band above.

A bug in the C worth noting, since the trace files differ visibly: `hmm_learn.c`
logs `csv_add_line(num_refinements + 1, ...)` *after* incrementing, so index 1
is never emitted and a one-refinement run writes rows 0 and 2. The stored 2020
traces show this. The port emits 0…n and does not reproduce the off-by-one.

### Phase 4 result (the C is gone)

Decided against the `c-oracle` feature: with every stage validated the retained
oracle was not worth keeping `build.rs`, the submodule and gcc alive. Straight
cutover to pure Rust instead.

Deleted: `src/ecoz2_lib` (593 lines), `src/comet_client`, `build.rs`, the
`ecoz2` submodule and `.gitmodules`, and the `cc`, `openmp-sys`, `libc`, `attohttpc` and `lazy_static`
dependencies. `cargo build` no longer needs a C compiler and no longer sets
`CC`; CI and the release workflow drop `submodules: recursive` along with it.

One gap had to be closed first: `hmm classify --predictors` had no Rust
implementation. It is now `hmm_classify_predictors_rs`, which mirrors the C's
`seq_provider` predictor path — each `.prd` quantized with every model's own
codebook (codebook `r` belongs to model `r`, positional, as the C asserts by
class name), each resulting sequence scored by that model. The class comes from
the predictor's recorded `className`, not from its path, which is where this
differs from `vq quantize`; and an unmodelled class is reported once, as the C's
`not_loaded_models` list does. The per-instance c12n CSV writer is now shared
with the sequences path (`C12nCsv`).

`seq show -P` / `-Q` stay TODO. They were never a regression here: the
`ecoz2_seq_show_files` call had been commented out for years, so those flags
were already no-ops before the C left.

Also removed, being C-only: the `cversion` subcommand, `set_random_seed`,
`hmm learn --ser` (it toggled the C's OpenMP), and `lpc --split` (deprecated,
and superseded by `util split`). `--zrs` is deleted everywhere. `lpc --zrsp`
becomes `lpc --par`, which is what it always meant — the multi-threaded LP
analysis, not an implementation choice.

The `lpca_c` benchmark arm went with the submodule, so there is no longer an
in-tree reference for the ~4x the C's `-ffast-math` build showed against
`lpca1`/`lpca2`. `lpca3` closed that gap and is what ships; the historical
numbers stay in the bench file's comment.

Still open from the phase 4 plan: item 4, revisiting the inherited caps
(`MAX_SEQS 4096`, `MAX_MODELS 256`, `MAX_CODEBOOK_SIZE 4096`,
`MAX_PREDICTION_ORDER 200`) as deliberate choices rather than inherited ones.

### Phase 4 plan

The goal is to delete the C: `src/ecoz2_lib` (593 lines, and **every `unsafe`
block in the crate**), `build.rs`, the `ecoz2` submodule and `openmp-sys`.

**Two gaps have to be closed first.** Twelve `use crate::ecoz2_lib::…` sites
remain, and all but two are just the non-`--zrs` branch of something already
ported. The exceptions have no Rust implementation at all yet:

- `hmm classify --predictors`. The port covers only `--sequences`. The predictor
  path quantizes each predictor against every class codebook and scores the
  resulting sequences, so it is mostly wiring over pieces that already exist
  (`vq_rs::Codebook`, `vq_rs::quantize`, `hmm_rs::log_prob`) — but it is not
  written.
- `seq show -P` / `-Q`, still marked TODO on the Rust side. Needs
  `hmm_log_prob` (have it) and Viterbi (have it, inside `hmm_learn_rs` — it
  would move somewhere shared).

**Then, in order:**

1. Flip the flag polarity, as decided above: `--zrs` is deleted, Rust becomes
   the default, and a `-c` opt-in exists only under `c-oracle`. `--zrsp` does
   not survive as an implementation flag either; parallelism belongs in a
   `--jobs`-style option or is left to rayon. In the harness this is one line in
   `stage_flags`.
2. Validate the whole harness with Rust as the default, C still present.
3. Only then remove the submodule, `openmp-sys` and the default `build.rs`.
4. Consider `#![forbid(unsafe_code)]`, and revisit the inherited caps
   (`MAX_SEQS 4096`, `MAX_MODELS 256`, `MAX_CODEBOOK_SIZE 4096`,
   `MAX_PREDICTION_ORDER 200`) as deliberate choices.

**Sequencing matters in two places.** Do not flip the default and delete the
submodule in the same step — a regression then has no oracle to be diagnosed
against. And leave the C-shaped-kernel cleanup in the backlog until after the
deletion: while the C exists, the one-for-one correspondence is what makes a
discrepancy findable.

### Decisions taken

- **Where algebraic float ops are allowed.** Relax arithmetic in *long*
  reductions over independent data that dominate the runtime; keep recurrences,
  convergence tests, and anything feeding a discrete decision on strict IEEE.
  The distinguishing property is the dependency chain, not the loop count: a
  short dependent recursion gains nothing from reassociation and can lose, since
  splitting into partial sums plus a reduction tree costs setup the loop is too
  short to amortize. This is what
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
  relax nothing, and this one was measured rather than argued: switching the
  forward-backward loops to the algebraic methods gives no speedup, and relaxing
  all of them is ~4% *slower* at N=20. The accumulations over T in `accumulate`
  are the one genuine long-reduction candidate there, and they gain nothing
  either — `gamma1[t][i]` strides across separate `Vec` allocations, so the
  reduction chases pointers and never vectorizes. Spend the effort on `rayon`
  across sequences, and on the layout, instead.
- **The `--zrs` flag is temporary, and its polarity will flip.** Today `--zrs`
  opts *into* the Rust implementation and the C is the default, which is the
  convention `lpc` and `prd show` already used. That is right while the port is
  in progress: the default stays the known-good C, and the harness opts in
  explicitly.

  Flip it once, at the end of phase 3, rather than per stage — mixed polarity
  across subcommands would be a standing source of mistakes. At that point every
  stage has a Rust implementation and the C exists only as a differential
  oracle, so the natural form is: no flag at all in a normal build, and a `-c`
  opt-in that exists only under the `c-oracle` feature. `--zrs` is then deleted
  rather than renamed; it never becomes part of the shipped interface. Phase 4
  removes the oracle and the flag with it.

  In the harness this is a one-line change: `stage_flags` in `run.sh` emits
  `--zrs` for Rust and `""` for C today, and would emit `""` for Rust and `-c`
  for C afterwards.

  **Superseded at phase 4**: there is no `-c` and no `c-oracle`. `--zrs` was
  deleted outright, with no deprecation window — it only ever existed for the
  port — and `stage_flags` emits `""` for every stage.

  Note `--zrsp` (the threaded LPC variant) is a different axis and should not
  survive as an implementation flag; parallelism belongs in a `--jobs`-style
  option or is simply left to rayon.

- **How to benchmark.** Interleave the variants and repeat; never compare two
  builds from separate sequential passes. Measured here: the *same* binary timed
  8.57 ms per HMM refinement in one pass and 11.68 ms in another, a 26% swing
  from thermal state or background load — larger than most effects worth
  chasing, and it silently favors whichever build ran second. An earlier
  "algebraic ops are 23% faster" conclusion in these notes was exactly that
  artifact, and reversed once the runs were interleaved. Isolate the loop of
  interest by differencing two iteration counts (`-I=2` against `-I=102`), since
  a whole run is dominated by process start and file loading.

- **Determinism.** Port RNG use to `rand`'s `ChaCha`/`StdRng`. `rand()`/`srand()`
  are libc-specific, so seeded runs are *already* not reproducible across
  platforms; this is a fix, not a risk. Every source of randomness should take
  an explicit seed and report the one it used. `hmm learn` already had `-s`;
  `util split` now has one too, and also sorts its input, since the shuffled
  markers are zipped positionally against a filesystem walk whose order is not
  guaranteed.

  Randomness is not the only source: **parallel reductions need a fixed order
  too**. The C's `vq learn` partitions across `omp_get_max_threads()` and sums
  in thread order, so its codebooks depend on the core count. Every ported
  parallel reduction should chunk by a fixed size rather than by the thread
  count and merge in chunk order, so the result does not depend on how the work
  is scheduled.
- **File formats: later, deliberately.** Keep reading the traditional formats
  at least for time being.
  Once there is a single writer, add an opt-in modern one — `.prd` and
  `.cbook` are plain 2-D f64 matrices, i.e. exactly `.npy`/`.npz`, whose header
  carries dtype and shape. Doing this after the port means changing one
  implementation instead of two. Note the current formats have no version field,
  no endianness marker, and no record of the `prob_t` width the writer used, so
  a mismatched build misreads silently.
- **`target-cpu` when benchmarking.** `build.rs` passes `-march=native` to the
  C while a plain `cargo build --release` leaves Rust at the baseline. Measured
  on aarch64 this makes no difference — `-C target-cpu=native` moved `vq learn`
  from 3.44 s to 3.93 s real with identical user time, i.e. noise, since
  baseline aarch64 already has NEON and FMA. It would matter on x86_64, whose
  baseline has no FMA at all. So state the target when quoting a Rust-vs-C
  number, and set `-C target-cpu` explicitly before comparing there. (What the
  *released* binaries should be built with is a separate question; leave it
  until the port works locally.)

### Backlog

**After phase 4 — undo the C-shaped kernels.** The numeric modules (`vq_rs`,
`vq_learn_rs`, `hmm_rs`, `hmm_learn_rs`) deliberately mirror the C: flat
`Vec<f64>` with manual stride arithmetic (`i * w..(i + 1) * w`,
`state * m + symbol`), index loops, and structs like `Partial` and `Refiner`
that exist only because the C used file-scope statics. That was the right call
while the two implementations had to be reviewed side by side — a discrepancy
has to be findable — but the justification expires with the C, and then it is
just debt. Note `ndarray` is already a dependency and is used by nothing but
`mm/markov.rs`. Rough order: flat arrays with manual striding → `ndarray`;
`Refiner`'s eight parallel scratch fields; `Partial`'s hand-written `merge`.

This is also the prerequisite for the one concrete performance lead: the
reductions over T in `hmm_learn_rs::accumulate` cannot vectorize while
`gamma1`/`gamma2` are `Vec<Vec<..>>`, because the stride crosses allocations.
Flatten those and the algebraic methods become worth re-testing there.

**Smaller items, found while building the harness**

- `vq learn --max-codebook-size` — **added in phase 2**. The C ladder always
  doubles to `MAX_CODEBOOK_SIZE`; you kill it and resume with `-B`, which is
  lossless (each size is saved on convergence, and resuming from M=256
  reproduces 512…4096 byte-identically), so the flag was ergonomics rather than
  a capability gap. Still open: `prepare_report` opens with `"w"`, so a resumed
  run truncates `eps_<ε>.rpt`/`.rpt.csv` to only the sizes it produced. Make the
  report append on resume.
- `-m` means two different things: instances in the label file for
  `sgn extract`, signal files present for `lpc`. Unify.
- `hmm classify --predictors-dir-template` defaults to `data/predictors`, which
  has no `{class}`/`{selection}` placeholders. With a `tt-list.csv` every row
  then resolves to that one literal directory and the command dies with
  `Is a directory (os error 21)`. The usable value is
  `data/predictors/{class}/{selection}.prd`; either make that the default or
  reject a template with no placeholders. Predates the port — the C received
  the same already-resolved list — and went unnoticed because the exerc07
  harness only ever exercises `hmm classify --sequences`. Found in phase 4 while
  validating `hmm_classify_predictors_rs`, which is the first Rust code to run
  that path.
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
