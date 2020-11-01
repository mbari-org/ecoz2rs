extern crate serde;
extern crate walkdir;

use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
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

/// General "resolution" for a file listing including case with a single given
/// `.csv` indicating such list plus some filtering (tt, class_name).
///
/// * `filenames` - given list of files
/// * `tt` - TRAIN or TEST if under `.csv` case
/// * `class_name_opt` - desired class name if under `.csv` case
/// * `subdir` - to compose names in returned list
/// * `file_ext` - to compose names in returned list or for filtering
///
pub fn resolve_files(
    filenames: Vec<PathBuf>,
    tt: &str,
    class_name_opt: Option<String>,
    subdir: String,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let subdir = subdir.as_str();
    let is_tt_list = filenames.len() == 1 && filenames[0].to_str().unwrap().ends_with(".csv");

    let filenames = if is_tt_list {
        get_files_from_csv(&filenames[0], tt, &class_name_opt, subdir, file_ext)?
    } else {
        resolve_filenames(filenames, file_ext, subdir)?
    };

    Ok(filenames)
}

// TODO some "unification" as variations of some methods were added rather hastily

pub fn resolve_files2(
    filenames: &Vec<PathBuf>,
    tt: &str,
    class_name_opt: &Option<String>,
    subdir: String,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let subdir = subdir.as_str();
    let is_tt_list = filenames.len() == 1 && filenames[0].to_str().unwrap().ends_with(".csv");

    let filenames = if is_tt_list {
        get_files_from_csv(&filenames[0], tt, class_name_opt, subdir, file_ext)?
    } else {
        resolve_filenames2(filenames, file_ext, subdir)?
    };

    Ok(filenames)
}

/// A train/Test row
#[derive(Debug, serde::Deserialize)]
struct TTRow {
    pub tt: String,
    pub class: String,
    pub selection: String,
}

/// Returns the `tt` (TRAIN or TEST) category filenames from the given csv.
pub fn get_files_from_csv(
    filename: &PathBuf,
    tt: &str,
    class_name_opt: &Option<String>,
    subdir: &str,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let br = BufReader::new(file);
    let mut rdr = csv::ReaderBuilder::new().delimiter(b',').from_reader(br);

    let rows: Vec<TTRow> = rdr
        .deserialize()
        .map(|result| result.unwrap())
        .collect::<Vec<_>>();

    let mut list: Vec<PathBuf> = Vec::new();

    let stuff = &"".to_string();
    let class_string = class_name_opt.as_ref().unwrap_or(stuff);
    let class: &str = class_string.as_str();
    for row in rows {
        if tt != row.tt {
            continue;
        }
        if !class.is_empty() && class != row.class {
            continue;
        }

        let filename = format!(
            "data/{}/{}/{}{}",
            subdir, row.class, row.selection, file_ext
        );
        list.push(PathBuf::from(filename));
    }
    if list.is_empty() {
        return Err(format!("No {} given in given file", subdir).into());
    }
    Ok(list)
}

// TODO unify the following with resolve_filenames2
// so use only one with more flexible parameter: `filenames: &Vec<PathBuf>`

/// Returns the list of files resulting from "resolving" the given list.
/// This will contain the same regular files in the list (but having the
/// given extension) plus files under any given directories.
pub fn resolve_filenames(
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

pub fn resolve_filenames2(
    filenames: &Vec<PathBuf>,
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

pub fn save_ser<T: serde::Serialize>(model: &T, filename: &str) -> Result<(), Box<dyn Error>> {
    let f = File::create(filename)?;
    let bw = BufWriter::new(f);
    serde_cbor::to_writer(bw, &model)?;
    Ok(())
}

pub fn save_json<T: serde::Serialize>(model: &T, filename: &str) -> Result<(), Box<dyn Error>> {
    let f = File::create(filename)?;
    let bw = BufWriter::new(f);
    serde_json::to_writer_pretty(bw, &model)?;
    Ok(())
}

pub fn to_pickle<T: serde::Serialize>(obj: &T, filename: &PathBuf) -> Result<(), Box<dyn Error>> {
    let serialized = serde_pickle::to_vec(&obj, true).unwrap();
    let f = File::create(filename)?;
    let mut bw = BufWriter::new(f);
    bw.write_all(&serialized[..])?;
    Ok(())
}
