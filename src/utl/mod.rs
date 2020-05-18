extern crate walkdir;

use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use byteorder::*;

use self::walkdir::WalkDir;

// first few defs to deal with files generated from C version

pub const FILE_IDENT_LEN: usize = 16;
pub const MAX_CLASS_NAME_LEN: usize = 96;

pub fn read_file_ident(br: &mut BufReader<File>) -> Result<String, Box<dyn Error>> {
    read_fixed_size_string(br, FILE_IDENT_LEN)
}

pub fn read_class_name(br: &mut BufReader<File>) -> Result<String, Box<dyn Error>> {
    read_fixed_size_string(br, MAX_CLASS_NAME_LEN)
}

fn read_fixed_size_string(
    br: &mut BufReader<File>,
    fixed_len: usize,
) -> Result<String, Box<dyn Error>> {
    let mut s = vec![0_u8; fixed_len];
    br.read(&mut s)?;
    let eol_pos = s.iter().position(|v| *v == 0_u8).unwrap_or(fixed_len - 1);
    // note: excluding the \0 byte itself:
    s.resize(eol_pos, 0);
    let s = String::from_utf8(s)?;
    Ok(s)
}

pub fn read_u32(br: &mut BufReader<File>) -> Result<u32, Box<dyn Error>> {
    match br.read_u32::<LittleEndian>() {
        Ok(v) => Ok(v),
        Err(e) => Err(e.into()),
    }
}

pub fn read_u16(br: &mut BufReader<File>) -> Result<u16, Box<dyn Error>> {
    match br.read_u16::<LittleEndian>() {
        Ok(v) => Ok(v),
        Err(e) => Err(e.into()),
    }
}

/// Returns the given list of files but expanding
/// any directories.
///
pub fn get_actual_filenames(
    filenames: Vec<PathBuf>,
    file_ext: &str,
    subjects_msg_if_empty: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut list: Vec<PathBuf> = Vec::new();
    for filename in filenames {
        let path = Path::new(&filename);
        if path.is_dir() {
            let dir_files = list_files(path, file_ext)?;
            list.extend(dir_files);
        } else if path.is_file() && path.to_str().unwrap().ends_with(file_ext) {
            list.push(path.to_path_buf());
        }
    }
    if !list.is_empty() {
        list.sort_by(|a, b| a.cmp(b));
    } else if !subjects_msg_if_empty.is_empty() {
        return Err(format!("No {} given", subjects_msg_if_empty).into());
    }
    Ok(list)
}

/// List all files under the given directory and having the given extension.
///
pub fn list_files(directory: &Path, file_ext: &str) -> io::Result<Vec<PathBuf>> {
    let mut list: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(directory) {
        let entry = entry.unwrap();
        let path = entry.path().to_path_buf();
        if path.is_file() && path.to_str().unwrap().ends_with(file_ext) {
            //println!("list_files: {}", entry.path().display());
            list.push(path);
        }
    }
    Ok(list)
}
