extern crate walkdir;
use std::error::Error;
use std::ffi::CString;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn get_actual_filenames(
    predictor_filenames: Vec<PathBuf>,
    file_ext: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let list = if predictor_filenames.len() == 1 {
        let path = Path::new(&predictor_filenames[0]);
        if path.is_dir() {
            list_files(path, file_ext)?
        } else {
            predictor_filenames
        }
    } else {
        predictor_filenames
    };
    Ok(list)
}

pub fn to_cstrings(paths: Vec<PathBuf>) -> Vec<CString> {
    paths
        .into_iter()
        .map(|predictor_filename| {
            let str = predictor_filename.to_str().unwrap();
            let c_string = CString::new(str).unwrap();
            //println!("c_string = {:?}", c_string);
            c_string
        })
        .collect()
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
