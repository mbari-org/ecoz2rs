2023-03

- fixed issue while trying to build on my Mac: `fatal error: "_stdio.h" No such file or directory`,
  caused by brew/apple idiosyncrasies. Anyway, all OK again after:
  ```shell
  brew upgrade
  xcode-select --install
  sudo xcode-select -switch /Library/Developer/CommandLineTools
  ```
  This really helped: <https://stackoverflow.com/a/74715717/830737> 

2022-11

- update criterion to 0.4.0 and use cargo-criterion:
  ```
  cargo install cargo-criterion
  cargo criterion
  open target/criterion/reports/index.html
  ```

2022-09

- fix #1 to address critical dependabot notification
- remove unneeded, large html files (bench generated)

2022-06

- ran `cargo update`, upon getting some dependabot notifs.
  All continues to work ok: `cargo test`, `cargo run -- --help`, `cargo bench` 

2022-04

- clippy fixes
- `sgn show`: only use the spec

2021-02

- comet
    - more generic parameter and metric logging to comet
    - use a lighter http client as reqwest is bringing in some heavy dependencies
      (even with only the json and blocking features) and taking the most significant
      build time (per `cargo +nightly build -Ztimings`), which went from
      3m 46.8s down to 
      1m 54.1s
      NOTE: change not tested yet -- comet hasn't been used for along while.

- gain of the system is sqrt(prediction_error)

- add `--pickle` option for `prd show` to put the extracted data into 
  a file in pickle format.  (only for the rust impl)
  
        $ cargo run prd show --zrs --cepstrum 40 --from 0 --to 40 --pickle cepstra.pickle predictor.prd
        # predictor.prd
        38265 vectors(s) saved to "cepstra.pickle"
  
        $ ipython3 -c 'import pickle; cepstra = pickle.load(open("cepstra.pickle", "rb")); len(cepstra)'
        Out[1]: 38265
  
- initial `--cepstrum` option for `prd show`

        $ cargo run prd show --zrs --cepstrum 40 --from 0 --to 40 predictor.prd
        # predictor.prd
        # class_name='_', T=38265 P=36
        c0,c1,c2,c3,c4,c5,c6,c7,c8,c9,c10,c11,c12,c13,c14,c15,c16,c17,c18,c19,c20,c21,c22,c23,c24,c25,c26,c27,c28,c29,c30,c31,c32,c33,c34,c35,c36,c37,c38,c39
        0.85860,0.85134,-0.32285,-0.04447,-0.00153,-0.04979,0.02819,-0.01774,0.00016,-0.00713,-0.01010,0.00890,-0.00433,-0.00081,-0.00387,0.00528,0.00390,-0.00818,-0.00068,-0.00024,0.00138,0.00326,-0.00123,0.00152,0.0039
        ...

- add `--predictors` option in Rust implementation of `prd show` to
  show the predictor coefficient vectors instead of the default
  auto-correlations (if `--reflections` is not given)
  
          cargo run prd show --zrs --predictors --from 0 --to 10 predictor.prd |head

- implement `--reflections` for Rust implementation of `prd show`.

    Comparison OK:
    
          cargo run prd show --zrs --reflections --from 0 --to 10 predictor.prd |head
          cargo run prd show       --reflections --from 0 --to 10 data/predictors/HBSe_20161221T010133/HBSe_20161221T010133.prd |head
      
    This is also preparation to generate cepstral vector.
  
- update notes re `lpc` and `prd show` Rust implementations

2021-01

- re-enabled build on my mojave mac as `CC=gcc-10 cargo build` failing with C compile problems
  (I recently removed xcode due to low space)
    - per https://developpaper.com/the-latest-version-of-xcode-that-macos-mojave-can-install/
      installed xcode 11.3.1 from https://developer.apple.com/download/more/?=xcode
    - also installed Command Line Tools for xcode 11.3.1
      (as `xcode-select --install` was unable to find the software)
    - now `CC=gcc-10 cargo build` finally completes!
    
    - Other things tried while coming up with the above:
        - `brew gist-logs gcc`
        - `brew doctor`
        - `rm -rf /Library/Developer/CommandLineTools`
        - `xcode-select --install`

2020-11

- add `--class-name` to `vq quantize`
- add `--predictors-dir-template` to `hmm classify`
- 0.6.1 add hmm_classify_predictors

- set `#` as comment character when reading in a csv file
- `seq show`: `--codebook-size` and `--tt` required only when `--pickle` given
- 0.5.6: `hmm learn`: fix description of `--class-name` parameter.
  This is actually for selection when `.csv` given.
  (the name of the training model is taken from the first training sequence.)

- 0.5.5: per C impl, `prob_t` type is now `double` by default,
  so, just building/installing with:
 
        CC=gcc-10 cargo install --path .

- 0.5.5: increased maximum prediction order (200)
 
- 0.5.4: 
    - `lpc`: add options for "tt-list" handling as in other commands.
      The general idea is train/test splitting should be done on signals.
      
      The new options include `--signals-dir-template`, which could be
      elaborated more in general in a future version.
      
        TODO: not have to specify `--tt`
        
    - verbose for lpc analysis in C impl

- 0.5.3: C impl increase of MAX_PREDICTION_ORDER to 70

NOTE:

Was able to build on `pam` (x86_64 GNU/Linux; gcc 4.8.5 20150623 (Red Hat 4.8.5-39)) 
by simply removing the "static" feature for `openmp-sys` in `Cargo.toml`.
 
NOTE: the conditioning added in `Cargo.toml` to accommodate both linux and macos
is not currently effective for linux per https://github.com/rust-lang/cargo/issues/7914.
The conditioning is because "Static linking is recommended on macOS."

This is the current situation:

Per https://github.com/rust-lang/cargo/issues/7914#issuecomment-599147541,
building on linux as follows:
  
    $ rustup override set nightly
        
Then any of the typical builds, eg:
    
    $ cargo +nightly build   -Z features=itarget
    $ cargo +nightly install -Z features=itarget --path .
        
Note: The updated `Cargo.toml` with the conditioning works just fine on MacOs
without having to set any of the "nightly" stuff, I suppose because the "static"
feature is always taking effect according to the issue.

- 0.5.2: update openmp-sys to 1.0.0 (from 0.1.7)

2020-10

- c12n: export y_true and y_pred to facilitate use of external tools
  for confusion matrix and the like
 
- `seq show`: add option `--pickle` to save sequence in pickle format.
  If given, also `--codebook-size`, `--tt` to be given, and
  optionally `--class-name`.

    Example: export all 'A' test instances with associated M=32:

        ecoz2 seq show --pickle M32_TEST_sequences_A.pickle \
                       --codebook-size=32 --tt=TEST \
                       --class-name=A  tt-list.csv

     TODO some cleanup in `utl` module.

- change mm probability type to `f32`

- 0.5.1: per C impl, can now override `prob_t` type, eg:
 
        CC=gcc-10 PROB_T=double cargo build --release

- 0.5.0: add `-m` option to `sgn extract` to indicate minimum
  number of instances to extract a class.

2020-08

- adjustments in `vq classify`

- exploring some iterator use in lpca
  - `cargo bench` includes lpca1 (as before) and lpca2 (with some iterators)
  - `cargo test test_lpca` to verify lpca1's and lpca2's outputs are the same

2020-07-31

- Reset my local dev environment since usual build was not working upon
  some various OS upgrades.

    In short:

    - Re-install XCode (11.6) and then `xcode-select --install`
    - Upgrade gcc `brew upgrade gcc`
    - update path: `export PATH=/usr/local/Cellar/gcc/10.1.0/bin:$PATH`
    - back to a successful complete build: `CC=gcc-10 cargo build [--release]`

2020-05

- `hmm classify` with option `--c12n` to generate classification results file.

        # num_models=18  M=256  num_seqs=408
        seq_filename,seq_class_name,correct,rank
        data/sequences/M256/A/00004.seq,A,*,1
        data/sequences/M256/Bd/00119.seq,Bd,!,2
        data/sequences/M256/Bu/00250.seq,Bu,!,3
        data/sequences/M256/E1/00823.seq,E1,!,13
        data/sequences/M256/E1/01000.seq,E1,*,1
        ...

    Note: rank now shown starting from 1 (not 0).

- rename nbayes command to nb
- 0.4.3 to align with c
- update ecoz2 pointer (which generates csv with hmm training measure)

- `sgn extract` now also with optional `--time-ranges`

- `util split` now generates `tt,class,selection` rows in csv output

        ecoz2 util split --train-fraction 0.8
                         --file-ext .prd
                         --files data/predictors/B 

    output something like:

        tt,class,selection
        TRAIN,A,00003
        TRAIN,A,00011
        TEST,A,00012
        ...

    This file, along with other relevant parameters depending on the command,
    is then used as basis to name derived files.
    TODO already functional per new exerc02 exercise in ecoz2-whale but
    need further testing.

- `sgn extract` simplify generated filename to only indicate selection number

- `vq learn` now accepts a .csv to indicate the .prd files to be
  used for training

        ecoz2 vq learn --prediction-order 36 --epsilon 0.0005 --predictors tt-list.csv

- `sgn extract` now accepts `--selection-ranges range ...` to indicate desired
  selection ranges for the extraction:

         ecoz2 sgn extract --segments MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels.csv \
                    --wav ${SOURCE_WAV} --selection-ranges 0-10 1100-1105 \
                    --out-dir data/signals
        ...
              Bm    1 instances
              Bu    1 instances
              I3    3 instances
               C    3 instances
              I4    2 instances
               A    4 instances
                   14 total extracted instances


- some adjs and cleanup.
  Quickly tried pickle as the output format for MM models, but it takes
  significantly more space wrt CBOR; also there's python loaders for CBOR.

- enable serde for ndarray
- use 2018 edition with the help of `cargo fix --edition`

- some minor adjs.

- add basic markov model learning and classification.

       $ ecoz2 mm learn data/sequences/TRAIN/M2048/B
       $ ecoz2 mm classify --models data/mms/M${M} --sequences data/sequences/TRAIN/M${M}

- add c12n module as helper for classification results/confusion matrix

- adding NBayes learning and classification.

       $ ecoz2 nbayes learn data/sequences/TRAIN/M2048/B
       $ ecoz2 nbayes classify --models data/nbs/M${M} --sequences data/sequences/TRAIN/M${M}

    As part of this, new `sequence` module to load and display sequences
    generated from C version.

- 0.3.63 - align with C
- release using github actions (linux and macos binaries)

- `vq learn`: initial optional logging to comet
    - if the COMET_API_KEY env var is defined and the `-exp-key` option
      is given (with the experiment id), then some parameters and metrics
      are reported

- 0.3.3 - `vq learn` can now accept a base codebook for "resuming" the
  training, that is, starting with the next power-of-2 codebook size.
  Note that for the additional codebook size (4096) the C impl currently
  needs a bigger stack, which is given from Rust via a thread.

- 0.3.1 aligned with C

- show versions (proper from cargo.toml;  and c code  via cversion cmd

- use openmp-sys to compile and use c version, which now uses openmp.
  Fortunately, found https://docs.rs/crate/openmp-sys/0.1.7 for this purpose.
  Build successful on:
    - macos with  gcc (Homebrew GCC 9.3.0_1) 9.3.0: `CC=gcc-9 cargo build [...]`
    - centos with gcc (GCC) 8.3.1 20190507 (Red Hat 8.3.1-4)


2020-04

- use `-march=native`
- minor adjustments per clippy suggestions 

- ok, lack of "fast-math" in Rust is what explains the big performance
  discrepancy between the C and Rust impls.
  (Notes here moved to notes.md)
    
 
- load signal samples into `Vec<f64>` and other adjustments
 
- benchmark exposed lpca (C impl) and do comparison with rust impl:

      cargo bench
      open target/criterion/report/index.html
      
- put lpca (rust impl) in a module and do some benchmarking
- some preps for benchmarking

- `lpc --zrsp` now uses all the available logical cores
  as reported with the num_cpus crate.
  
    With the 4.5hr file, the multi-threaded processing on my mac (8 logical cores) 
    takes now 6.63s vs. 35.53s with the single-threaded version `--zrs`.
    However, the C version (single-threaded) takes 10.65s, meaning
    that (unsurprisingly given the preliminary attempts) there's lots
    to be improved in the Rust version regarding performance.
    
    (note: I verified that all versions generate the same output.) 

- `prd show --zrs` more similar to the c impl
- lpc: --zrs and --zrsp now generating exactly same output
- reincorporate some of the rust implementations.
  Note: not fully implemented yet.
  Special `--zrs` options added to exercise them.

- have get_actual_filenames fail if resulting list is empty
- use callback in `vq learn`
- add `--seed` option for `hmm learn`
- 0.2.0: same version as in C version
- use the `ecoz2_` prefixed functions exported from C

- 0.1.2: callback in ecoz2_hmm_learn
 
2020-03

- get_actual_filenames: expand all given directories

- reorganize project

- add `vq classify`

- fix in `vq learn` to properly pass class name to C impl
 
- add `seq show`

      cargo run seq show ../ecoz2-whale/exerc01/data/sequences/TRAIN/M2048/B/

- `ecoz2 prd` now uses the C implementation
  (only `prd show` at this point)

- `ecoz2 lpc` now uses the C implementation

        $ cargo build --release
        $ cd MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels
        $ for t in `ls signals`; do 
            ecoz2 lpc -P 36 -W 45 -O 15 -m 10 -s 0.8 signals/$t &
        done
      
    rust implementation moved to `lpc_rs.rs`.

- add `vq show`

      cargo run -- vq show --codebook data/codebooks/_/eps_0.0005_M_0002.cbook

- add `hmm show`

      cargo run -- hmm show --hmm data/hmms/N64__M4_t3__a0.1_I10/A.hmm

- add `hmm classify`

      cargo run -- hmm classify \
            --models data/hmms/N64__M4_t3__a0.1_I10 \
            --sequences data/sequences/TRAIN/M4

- add ecoz2_lib module to link with the c library

- add `hmm learn`

      cargo run -- hmm learn -a 0.1 -I 10 -N 64 data/sequences/TRAIN/M2/B/*.seq

- add `vq quantize`
  TODO: remove some C warnings

       cargo run -- vq quantize --codebook  data/codebooks/_/eps_0.0005_M_0002.cbook \
            MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels/data/predictors/TRAIN
       
       
- adjust `vq` also as command with subcommands
- adjust `vq-learn` to scan all `.prd` files under a given directory

       cargo run -- vq learn -P 36 -e 0.0005 MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels/data/predictors/TRAIN

- implement `sgn extract` to extract segments from a given signal

       cargo run -- sgn extract \ 
       --wav ~/Downloads/MARS_20161221_000046_SongSession_16kHz_HPF5Hz/MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav \
       --segments MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels/MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels.csv \
       --out-dir MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels/signals
            
    Output files organized by given "type", eg:
    
        $ ls MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels/signals/B
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__10079.092_10080.35.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__10540.822_10543.197.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__1068.8552_1069.205.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__1089.723_1090.6355.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__1102.666_1103.1608.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__12907.783_12909.293.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__15415.037_15417.307.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__2378.6963_2380.534.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__2926.575_2929.6223.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__5067.5444_5070.2764.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__588.77454_591.3191.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__680.14154_680.8046.wav
        from_MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav__7145.495_7147.0107.wav

- sgn show now run as follows:

        cargo run -- sgn show --file ~/Downloads/MARS_20161221_000046_SongSession_16kHz_HPF5Hz/MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav
            Finished dev [optimized + debuginfo] target(s) in 0.05s
             Running `target/debug/ecoz2 sgn show --file /Users/carueda/Downloads/MARS_20161221_000046_SongSession_16kHz_HPF5Hz/MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav`
        Signal loaded: /Users/carueda/Downloads/MARS_20161221_000046_SongSession_16kHz_HPF5Hz/MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav
        num_samples: 266117287  sample_rate: 16000  bits_per_sample: 16  sample_format = Int

- prepare sgn module as subcommand with subcommands
- add very preliminary csvutil helper 
  to read raven segmentation+labels file:
  
        cargo run -- csv-show --file MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels/MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels.csv

2019-09

- initial version
