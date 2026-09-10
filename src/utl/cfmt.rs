//! Readers for the traditional ECOZ2 binary file formats written by the C code.
//!
//! Each file starts with a 16-byte NUL-padded identifier and a 96-byte class
//! name (name, NUL, then `_` padding), followed by native-endian scalars and
//! `double` arrays.  See `ecoz2/src/utl/fileutil.c` and the corresponding
//! `*_save` / `*_load` functions.
//!
//! Note that the on-disk layout carries no version, no endianness marker, and
//! no record of the `prob_t` width the writer was built with; these readers
//! assume the defaults (`double`, little-endian) that every released build has
//! used.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};

use super::{FILE_IDENT_LEN, MAX_CLASS_NAME_LEN};

/// A block of values within an artifact, named so that differences can be
/// attributed to a particular part of the model.
#[derive(Debug)]
pub enum Section {
    Floats(Vec<f64>),
    Symbols(Vec<u16>),
}

impl Section {
    pub fn len(&self) -> usize {
        match self {
            Section::Floats(v) => v.len(),
            Section::Symbols(v) => v.len(),
        }
    }
}

/// An artifact loaded from one of the traditional binary formats.
#[derive(Debug)]
pub struct Artifact {
    /// `predictor`, `codebook`, `sequence`, or `hmm`
    pub kind: &'static str,
    pub class_name: String,
    /// Human-readable shape, e.g. `T=38265 P=20`
    pub dims: String,
    /// Values per row, where the artifact is a stack of equal-length vectors
    /// (`1 + P` for predictors and codebooks). Lets a comparison reason about
    /// whole codewords rather than a flat array.
    pub row_len: Option<usize>,
    pub sections: Vec<(&'static str, Section)>,
}

impl fmt::Display for Artifact {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<{}> class='{}' {}",
            self.kind, self.class_name, self.dims
        )
    }
}

fn read_fixed_string<R: Read>(r: &mut R, fixed_len: usize) -> Result<String, Box<dyn Error>> {
    let mut buf = vec![0u8; fixed_len];
    r.read_exact(&mut buf)?;
    let end = buf.iter().position(|b| *b == 0).unwrap_or(fixed_len);
    buf.truncate(end);
    Ok(String::from_utf8(buf)?)
}

/// Reads the leading identifier without insisting it be valid UTF-8: a file
/// that is not an ECOZ2 artifact at all should be reported as such, rather
/// than as a decoding error from somewhere inside the header.
fn read_ident<R: Read>(r: &mut R) -> Result<String, Box<dyn Error>> {
    let mut buf = vec![0u8; FILE_IDENT_LEN];
    r.read_exact(&mut buf)?;
    let end = buf.iter().position(|b| *b == 0).unwrap_or(FILE_IDENT_LEN);
    buf.truncate(end);
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn read_i32<R: Read>(r: &mut R) -> Result<i32, Box<dyn Error>> {
    Ok(r.read_i32::<LittleEndian>()?)
}

fn read_u32<R: Read>(r: &mut R) -> Result<u32, Box<dyn Error>> {
    Ok(r.read_u32::<LittleEndian>()?)
}

fn read_f64_vec<R: Read>(r: &mut R, n: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut v = vec![0f64; n];
    r.read_f64_into::<LittleEndian>(&mut v)?;
    Ok(v)
}

fn read_u16_vec<R: Read>(r: &mut R, n: usize) -> Result<Vec<u16>, Box<dyn Error>> {
    let mut v = vec![0u16; n];
    r.read_u16_into::<LittleEndian>(&mut v)?;
    Ok(v)
}

/// Checks a count read from a file before it is used to size an allocation.
/// These files carry no length field, so a corrupt or foreign file would
/// otherwise turn into a huge `vec![0; n]`.
fn checked_count(label: &str, value: i32) -> Result<usize, Box<dyn Error>> {
    if value <= 0 {
        return Err(format!("invalid {}: {}", label, value).into());
    }
    Ok(value as usize)
}

/// Loads any of the traditional artifacts, dispatching on the file identifier.
pub fn load(path: &Path) -> Result<Artifact, Box<dyn Error>> {
    let f = File::open(path)?;
    let mut br = BufReader::new(f);
    read_artifact(&mut br, &path.display().to_string())
}

/// As [`load`], but from any reader; `source` only names the input in errors.
pub fn read_artifact<R: Read>(r: &mut R, source: &str) -> Result<Artifact, Box<dyn Error>> {
    let ident = read_ident(r)?;
    match ident.as_str() {
        "<predictor>" => parse_predictor(r),
        "<codebook>" => parse_codebook(r),
        "<sequence>" => parse_sequence(r),
        "<hmm>" => parse_hmm(r),

        // A `.prd` written by the current Rust `lpc --zrs` is serde_cbor, which
        // has no such header; say so plainly rather than reporting garbage.
        other => Err(format!(
            "{}: unrecognized file identifier {:?} \
             (a Rust-written .prd is CBOR, not the traditional format)",
            source, other
        )
        .into()),
    }
}

fn parse_predictor<R: Read>(r: &mut R) -> Result<Artifact, Box<dyn Error>> {
    let class_name = read_fixed_string(r, MAX_CLASS_NAME_LEN)?;
    let p = checked_count("P", read_i32(r)?)?;
    let t = checked_count("T", read_i32(r)?)?;
    let values = read_f64_vec(r, t * (1 + p))?;
    Ok(Artifact {
        kind: "predictor",
        class_name,
        dims: format!("T={} P={}", t, p),
        row_len: Some(1 + p),
        sections: vec![("vectors", Section::Floats(values))],
    })
}

fn parse_codebook<R: Read>(r: &mut R) -> Result<Artifact, Box<dyn Error>> {
    let class_name = read_fixed_string(r, MAX_CLASS_NAME_LEN)?;
    let p = checked_count("P", read_i32(r)?)?;
    let num_vecs = checked_count("num_vecs", read_i32(r)?)?;
    let values = read_f64_vec(r, num_vecs * (1 + p))?;
    Ok(Artifact {
        kind: "codebook",
        class_name,
        dims: format!("M={} P={}", num_vecs, p),
        row_len: Some(1 + p),
        sections: vec![("vectors", Section::Floats(values))],
    })
}

fn parse_sequence<R: Read>(r: &mut R) -> Result<Artifact, Box<dyn Error>> {
    let class_name = read_fixed_string(r, MAX_CLASS_NAME_LEN)?;
    let len = read_u32(r)? as usize;
    let codebook_size = read_u32(r)?;
    let symbols = read_u16_vec(r, len)?;
    Ok(Artifact {
        kind: "sequence",
        class_name,
        dims: format!("T={} M={}", len, codebook_size),
        row_len: None,
        sections: vec![("symbols", Section::Symbols(symbols))],
    })
}

fn parse_hmm<R: Read>(r: &mut R) -> Result<Artifact, Box<dyn Error>> {
    let class_name = read_fixed_string(r, MAX_CLASS_NAME_LEN)?;
    let n = checked_count("N", read_i32(r)?)?;
    let m = checked_count("M", read_i32(r)?)?;
    let pi = read_f64_vec(r, n)?;
    let a = read_f64_vec(r, n * n)?;
    let b = read_f64_vec(r, n * m)?;
    Ok(Artifact {
        kind: "hmm",
        class_name,
        dims: format!("N={} M={}", n, m),
        row_len: None,
        sections: vec![
            ("pi", Section::Floats(pi)),
            ("A", Section::Floats(a)),
            ("B", Section::Floats(b)),
        ],
    })
}

/// File extensions understood by [`load`].
pub const EXTENSIONS: &[&str] = &["prd", "cbook", "seq", "hmm"];

// --- writing ---------------------------------------------------------------
//
// Byte-for-byte compatible with `write_file_ident` / `write_fixed_size_string`
// in `ecoz2/src/utl/fileutil.c`: the identifier is NUL-padded, while a class
// name is NUL-terminated and then padded with '_'.

fn write_ident<W: Write>(w: &mut W, ident: &str) -> Result<(), Box<dyn Error>> {
    let mut buf = vec![0u8; FILE_IDENT_LEN];
    let b = ident.as_bytes();
    let n = b.len().min(FILE_IDENT_LEN - 1);
    buf[..n].copy_from_slice(&b[..n]);
    w.write_all(&buf)?;
    Ok(())
}

fn write_class_name<W: Write>(w: &mut W, name: &str) -> Result<(), Box<dyn Error>> {
    let mut buf = vec![b'_'; MAX_CLASS_NAME_LEN];
    let b = name.as_bytes();
    let n = b.len().min(MAX_CLASS_NAME_LEN - 1);
    buf[..n].copy_from_slice(&b[..n]);
    buf[n] = 0;
    w.write_all(&buf)?;
    Ok(())
}

fn write_i32<W: Write>(w: &mut W, v: i32) -> Result<(), Box<dyn Error>> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

/// A stack of equal-length f64 vectors with a class name: the shape shared by
/// `<predictor>` (T rows of normalized autocorrelation) and `<codebook>`
/// (M rows of reflection coefficients), each row `1 + prediction_order` long.
pub struct VectorSet {
    pub class_name: String,
    pub prediction_order: usize,
    pub vectors: Vec<Vec<f64>>,
}

/// Writes a `<predictor>` file in the traditional format, as `prd_save` does.
pub fn save_predictor(path: &Path, prd: &VectorSet) -> Result<(), Box<dyn Error>> {
    let p = prd.prediction_order;
    let t = prd.vectors.len();
    if t == 0 {
        return Err(format!("{}: refusing to write an empty predictor", path.display()).into());
    }
    if let Some(bad) = prd.vectors.iter().position(|v| v.len() != 1 + p) {
        return Err(format!(
            "{}: vector {} has length {}, expected {}",
            path.display(),
            bad,
            prd.vectors[bad].len(),
            1 + p
        )
        .into());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let f = File::create(path)?;
    let mut bw = BufWriter::new(f);
    write_ident(&mut bw, "<predictor>")?;
    write_class_name(&mut bw, &prd.class_name)?;
    write_i32(&mut bw, p as i32)?;
    write_i32(&mut bw, t as i32)?;
    for v in &prd.vectors {
        for x in v {
            bw.write_all(&x.to_le_bytes())?;
        }
    }
    bw.flush()?;
    Ok(())
}

/// Reads a `<predictor>` file written by either implementation.
pub fn load_predictor(path: &Path) -> Result<VectorSet, Box<dyn Error>> {
    let a = load(path)?;
    if a.kind != "predictor" {
        return Err(format!("{}: not a predictor, but a <{}>", path.display(), a.kind).into());
    }
    // `dims` is "T=<t> P=<p>"; take P from it rather than re-parsing the file.
    let p: usize = a
        .dims
        .split_whitespace()
        .find_map(|kv| kv.strip_prefix("P="))
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| format!("{}: cannot determine P", path.display()))?;

    let flat = match a.sections.into_iter().next() {
        Some((_, Section::Floats(v))) => v,
        _ => return Err(format!("{}: unexpected predictor content", path.display()).into()),
    };
    let vectors = flat.chunks_exact(1 + p).map(|c| c.to_vec()).collect();
    Ok(VectorSet {
        class_name: a.class_name,
        prediction_order: p,
        vectors,
    })
}

/// Writes a `<codebook>` in the traditional format, as `cb_save` does.
pub fn save_codebook(path: &Path, cb: &VectorSet) -> Result<(), Box<dyn Error>> {
    let p = cb.prediction_order;
    if let Some(bad) = cb.vectors.iter().position(|v| v.len() != 1 + p) {
        return Err(format!(
            "{}: entry {} has length {}, expected {}",
            path.display(),
            bad,
            cb.vectors[bad].len(),
            1 + p
        )
        .into());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let f = File::create(path)?;
    let mut bw = BufWriter::new(f);
    write_ident(&mut bw, "<codebook>")?;
    write_class_name(&mut bw, &cb.class_name)?;
    write_i32(&mut bw, p as i32)?;
    write_i32(&mut bw, cb.vectors.len() as i32)?;
    for v in &cb.vectors {
        for x in v {
            bw.write_all(&x.to_le_bytes())?;
        }
    }
    bw.flush()?;
    Ok(())
}

/// Reads a `<codebook>`: M reflection vectors of `1 + P` values, as `cb_load`.
pub fn load_codebook(path: &Path) -> Result<VectorSet, Box<dyn Error>> {
    let a = load(path)?;
    if a.kind != "codebook" {
        return Err(format!("{}: not a codebook, but a <{}>", path.display(), a.kind).into());
    }
    let w = a
        .row_len
        .ok_or_else(|| format!("{}: cannot determine P", path.display()))?;
    let flat = match a.sections.into_iter().next() {
        Some((_, Section::Floats(v))) => v,
        _ => return Err(format!("{}: unexpected codebook content", path.display()).into()),
    };
    Ok(VectorSet {
        class_name: a.class_name,
        prediction_order: w - 1,
        vectors: flat.chunks_exact(w).map(|c| c.to_vec()).collect(),
    })
}

/// Writes a `<sequence>` file as `seq_save` does. `Symbol` is `unsigned short`.
pub fn save_sequence(
    path: &Path,
    class_name: &str,
    codebook_size: usize,
    symbols: &[u16],
) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let f = File::create(path)?;
    let mut bw = BufWriter::new(f);
    write_ident(&mut bw, "<sequence>")?;
    write_class_name(&mut bw, class_name)?;
    write_i32(&mut bw, symbols.len() as i32)?;
    write_i32(&mut bw, codebook_size as i32)?;
    for s in symbols {
        bw.write_all(&s.to_le_bytes())?;
    }
    bw.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Builds the 16-byte identifier and 96-byte class name headers exactly as
    /// `write_file_ident` / `write_fixed_size_string` in the C do: the ident is
    /// NUL-padded, the class name is NUL-terminated then padded with '_'.
    fn header(ident: &str, class_name: &str) -> Vec<u8> {
        let mut v = vec![0u8; FILE_IDENT_LEN];
        v[..ident.len()].copy_from_slice(ident.as_bytes());
        let mut cn = vec![b'_'; MAX_CLASS_NAME_LEN];
        cn[..class_name.len()].copy_from_slice(class_name.as_bytes());
        cn[class_name.len()] = 0;
        v.extend(cn);
        v
    }

    fn floats(vals: &[f64]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn reads_codebook() {
        let mut buf = header("<codebook>", "Bm");
        buf.extend(2i32.to_le_bytes()); // P
        buf.extend(3i32.to_le_bytes()); // num_vecs
        buf.extend(floats(&[1., 2., 3., 4., 5., 6., 7., 8., 9.])); // 3 * (1+P)

        let a = read_artifact(&mut Cursor::new(buf), "test").unwrap();
        assert_eq!(a.kind, "codebook");
        assert_eq!(a.class_name, "Bm");
        assert_eq!(a.dims, "M=3 P=2");
        match &a.sections[0].1 {
            Section::Floats(v) => assert_eq!(v.len(), 9),
            _ => panic!("expected floats"),
        }
    }

    #[test]
    fn reads_sequence() {
        let mut buf = header("<sequence>", "II");
        buf.extend(4u32.to_le_bytes()); // len
        buf.extend(2048u32.to_le_bytes()); // codebook size
        for s in [7u16, 1000, 2047, 0] {
            buf.extend(s.to_le_bytes());
        }

        let a = read_artifact(&mut Cursor::new(buf), "test").unwrap();
        assert_eq!(a.kind, "sequence");
        assert_eq!(a.class_name, "II");
        assert_eq!(a.dims, "T=4 M=2048");
        match &a.sections[0].1 {
            Section::Symbols(v) => assert_eq!(v, &[7, 1000, 2047, 0]),
            _ => panic!("expected symbols"),
        }
    }

    #[test]
    fn reads_hmm_sections() {
        let (n, m) = (2usize, 3usize);
        let mut buf = header("<hmm>", "A");
        buf.extend((n as i32).to_le_bytes());
        buf.extend((m as i32).to_le_bytes());
        buf.extend(floats(&vec![0.5; n])); // pi
        buf.extend(floats(&vec![0.25; n * n])); // A
        buf.extend(floats(&vec![0.125; n * m])); // B

        let a = read_artifact(&mut Cursor::new(buf), "test").unwrap();
        assert_eq!(a.dims, "N=2 M=3");
        let names: Vec<_> = a.sections.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, vec!["pi", "A", "B"]);
        let lens: Vec<_> = a.sections.iter().map(|(_, s)| s.len()).collect();
        assert_eq!(lens, vec![n, n * n, n * m]);
    }

    /// A `.prd` written by the current Rust `lpc --zrs` is CBOR; the reader must
    /// say so rather than interpreting the bytes as a traditional predictor.
    #[test]
    fn rejects_unknown_ident() {
        let buf = vec![0xA3u8; 64]; // CBOR-ish, certainly not an ECOZ2 header
        let err = read_artifact(&mut Cursor::new(buf), "x.prd").unwrap_err();
        assert!(err.to_string().contains("unrecognized file identifier"));
    }

    /// The formats carry no length field, so a bogus count must be rejected
    /// before it becomes an allocation.
    #[test]
    fn rejects_negative_count() {
        let mut buf = header("<predictor>", "A");
        buf.extend((-1i32).to_le_bytes()); // P
        buf.extend(10i32.to_le_bytes()); // T
        let err = read_artifact(&mut Cursor::new(buf), "test").unwrap_err();
        assert!(err.to_string().contains("invalid P"));
    }
}
