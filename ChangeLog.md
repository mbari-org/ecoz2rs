2020-04

- `lpc --zrsp` now uses all the available logical cores
  as reported with the num_cpus crate.
  BTW, with the 4.5hr file, processing on my mac (8 logical cores) 
  only takes now 6.81s (vs. 35.13s with the serialized version `--zrs`). 

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