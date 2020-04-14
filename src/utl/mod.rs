extern crate walkdir;

use std::error::Error;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use self::walkdir::WalkDir;

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
        return Err(format!("No {} given", subjects_msg_if_empty))?;
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
