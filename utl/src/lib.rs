extern crate walkdir;

use std::error::Error;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use walkdir::WalkDir;

pub fn get_actual_filenames(
    filenames: Vec<PathBuf>,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut list = if filenames.len() == 1 {
        let path = Path::new(&filenames[0]);
        if path.is_dir() {
            list_files(path, file_ext)?
        } else {
            filenames
        }
    } else {
        filenames
    };
    list.sort_by(|a, b| a.cmp(b));
    Ok(list)
}

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
