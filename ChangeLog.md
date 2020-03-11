2020-03

- implement `sgn extract` to extract segments from a given signal

       cargo run -- sgn extract \
            --segments MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels.csv \
            --wav ~/Downloads/MARS_20161221_000046_SongSession_16kHz_HPF5Hz/MARS_20161221_000046_SongSession_16kHz_HPF5Hz.wav \
            --out-prefix /tmp/
            
    TODO organize output files by given "type"

- prepare sgn module as subcommand with subcommands
- add very preliminary csvutil helper 
  to read raven segmentation+labels file:
  
        cargo run -- csv-show --file MARS_20161221_000046_SongSession_16kHz_HPF5HzNorm_labels.csv

2019-09

- initial version