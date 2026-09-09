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

pub mod cfmt;

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
    br.read_exact(&mut s)?;
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
        get_files_from_csv(&filenames[0], tt, &class_name_opt, subdir, file_ext, &None)?
    } else {
        resolve_filenames(filenames, file_ext, subdir)?
    };

    Ok(filenames)
}

// TODO some "unification" as variations of some methods were added rather hastily

pub fn resolve_files2(
    filenames: &[PathBuf],
    tt: &str,
    class_name_opt: &Option<String>,
    subdir: String,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let subdir = subdir.as_str();
    let is_tt_list = filenames.len() == 1 && filenames[0].to_str().unwrap().ends_with(".csv");

    let filenames = if is_tt_list {
        get_files_from_csv(&filenames[0], tt, class_name_opt, subdir, file_ext, &None)?
    } else {
        resolve_filenames2(filenames, file_ext, subdir)?
    };

    Ok(filenames)
}

pub fn resolve_files3(
    filenames: &[PathBuf],
    tt: &str,
    class_name_opt: &Option<String>,
    subdir: String,
    subdir_template: String,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let subdir = subdir.as_str();
    let is_tt_list = filenames.len() == 1 && filenames[0].to_str().unwrap().ends_with(".csv");

    let filenames = if is_tt_list {
        let subdir_template = Some(subdir_template);
        get_files_from_csv(
            &filenames[0],
            tt,
            class_name_opt,
            subdir,
            file_ext,
            &subdir_template,
        )?
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
    filename: &Path,
    tt: &str,
    class_name_opt: &Option<String>,
    subdir: &str,
    file_ext: &str,
    subdir_template_opt: &Option<String>,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let br = BufReader::new(file);
    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .delimiter(b',')
        .from_reader(br);

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

        let filename = match subdir_template_opt {
            Some(subdir_template) => subdir_template
                .replace("{class}", &row.class)
                .replace("{selection}", &row.selection),

            None => format!(
                "data/{}/{}/{}{}",
                subdir, row.class, row.selection, file_ext
            ),
        };

        list.push(PathBuf::from(filename));
    }
    if list.is_empty() {
        return Err(format!("No {} given in given file", subdir).into());
    }
    Ok(list)
}

// TODO unify the following with resolve_filenames2
// so use only one with more flexible parameter: `filenames: &[PathBuf]`

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
        list.sort();
    } else if !subjects_msg_if_empty.is_empty() {
        return Err(format!("No {} given", subjects_msg_if_empty).into());
    }
    Ok(list)
}

pub fn resolve_filenames2(
    filenames: &[PathBuf],
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
        list.sort();
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

/// Replaces a filename's extension, following `camext` in the C.
///
/// Its quirks are preserved deliberately, so derived names match: a name with
/// no `.` is returned unchanged, and a trailing `.` or one-character extension
/// gets the new extension *appended* rather than substituted.
pub fn camext(name: &str, new_ext: &str) -> String {
    match name.rfind('.') {
        None => name.to_string(),
        Some(i) => {
            if name.len() - i < 3 {
                format!("{}{}", name, new_ext)
            } else if new_ext.is_empty() || new_ext == "." {
                name[..i].to_string()
            } else if let Some(stripped) = new_ext.strip_prefix('.') {
                format!("{}.{}", &name[..i], stripped)
            } else {
                format!("{}.{}", &name[..i], new_ext)
            }
        }
    }
}

/// The class a signal or predictor path implies: its second-to-last component,
/// as `get_class_name` in the C does. Empty when the path has fewer than two
/// separators, e.g. `foo.wav` or `dir/foo.wav`.
pub fn class_name_of(path: &Path) -> String {
    let s = path.to_string_lossy();
    let head = match s.rfind('/') {
        Some(i) => &s[..i],
        None => return String::new(),
    };
    match head.rfind('/') {
        Some(i) => head[i + 1..].to_string(),
        None => String::new(),
    }
}

/// Where a derived artifact goes, as `get_output_filename` in the C computes
/// it: `data/<base_dir>/<class>/<stem><ext>`.
///
/// Note the class defaults to `_` here while [`class_name_of`] yields an empty
/// string for the same path; the C has the same asymmetry, and the two agree
/// for any path with at least two separators, which is the normal case.
pub fn output_filename(from: &Path, base_dir: &str, ext: &str) -> PathBuf {
    let s = from.to_string_lossy();
    let renamed = camext(&s, ext);
    let (simple, class_name) = match renamed.rfind('/') {
        Some(i) => {
            let simple = renamed[i + 1..].to_string();
            let head = &renamed[..i];
            let class = match head.rfind('/') {
                Some(j) => head[j + 1..].to_string(),
                None => "_".to_string(),
            };
            (simple, class)
        }
        None => (renamed.clone(), "_".to_string()),
    };
    PathBuf::from(format!("data/{}/{}/{}", base_dir, class_name, simple))
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

pub fn to_pickle<T: serde::Serialize>(obj: &T, filename: &Path) -> Result<(), Box<dyn Error>> {
    let serialized = serde_pickle::to_vec(&obj, serde_pickle::SerOptions::new())?;
    let f = File::create(filename)?;
    let mut bw = BufWriter::new(f);
    bw.write_all(&serialized[..])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camext_replaces_a_normal_extension() {
        assert_eq!(camext("00003.wav", ".prd"), "00003.prd");
        assert_eq!(
            camext("data/signals/A/00003.wav", ".prd"),
            "data/signals/A/00003.prd"
        );
        assert_eq!(camext("x.wav", "prd"), "x.prd");
    }

    /// Quirks kept from the C so derived names stay identical.
    #[test]
    fn camext_quirks_match_the_c() {
        assert_eq!(camext("noext", ".prd"), "noext"); // no '.': unchanged
        assert_eq!(camext("x.", ".prd"), "x..prd"); // len-i < 3: appended
        assert_eq!(camext("x.a", ".prd"), "x.a.prd"); // len-i < 3: appended
        assert_eq!(camext("x.wav", ""), "x"); // empty ext drops it
    }

    #[test]
    fn class_name_is_the_second_to_last_component() {
        assert_eq!(class_name_of(Path::new("data/signals/A/00003.wav")), "A");
        assert_eq!(class_name_of(Path::new("A/00003.wav")), "");
        assert_eq!(class_name_of(Path::new("00003.wav")), "");
    }

    #[test]
    fn output_filename_matches_the_c_layout() {
        assert_eq!(
            output_filename(Path::new("data/signals/A/00003.wav"), "predictors", ".prd"),
            PathBuf::from("data/predictors/A/00003.prd")
        );
        // fewer than two separators: the C defaults the class to "_"
        assert_eq!(
            output_filename(Path::new("00003.wav"), "predictors", ".prd"),
            PathBuf::from("data/predictors/_/00003.prd")
        );
    }
}
