use itertools::Itertools;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use crate::seq;
use crate::utl;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Sequence {
    pub class_name: String,
    pub codebook_size: u32,
    pub symbols: Vec<u16>,
}

impl Sequence {
    pub fn show(&mut self, opts: &seq::SeqShowOpts) {
        if let Some(pickle_filename) = &opts.pickle {
            self.to_pickle(pickle_filename);
        }
        if opts.no_sequence {
            return;
        }
        if opts.only_length {
            println!("{}", self.symbols.len());
            return;
        }

        let symbols_to_show = if opts.full || self.symbols.len() <= 30 {
            self.symbols.iter().join(", ")
        } else {
            let v = self.symbols[..10].to_vec();
            let w = self.symbols[self.symbols.len() - 10..].to_vec();
            format!(
                "{}{}{}",
                v.iter().join(", "),
                ", ..., ",
                w.iter().join(", ")
            )
        };
        println!(
            "<{}(M={},L={}): {}>",
            self.class_name,
            self.codebook_size,
            self.symbols.len(),
            symbols_to_show,
        );
    }

    fn to_pickle(&self, filename: &PathBuf) {
        let serialized = serde_pickle::to_vec(&self.symbols, true).unwrap();
        let f = File::create(filename).unwrap();
        let mut bw = BufWriter::new(f);
        bw.write_all(&serialized[..]).unwrap();
        println!(" Sequence saved to: {:?}", filename);
    }
}

pub fn load(filename: &str) -> Result<Sequence, Box<dyn Error>> {
    let f = File::open(filename)?;
    let mut br = BufReader::new(f);

    let ident = utl::read_file_ident(&mut br)?;
    if !ident.starts_with("<sequence>") {
        return Err(format!("{}: Not a sequence", filename)).unwrap();
    }

    let class_name: String = utl::read_class_name(&mut br)?;

    let len = utl::read_u32(&mut br)?;

    let codebook_size = utl::read_u32(&mut br)?;

    let mut symbols: Vec<u16> = Vec::new();

    for _ in 0..len {
        symbols.push(utl::read_u16(&mut br)?);
    }

    Ok(Sequence {
        class_name,
        codebook_size,
        symbols,
    })
}
