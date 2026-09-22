//! A small, safe stand-in for the `printf` formats the C accepts.
//!
//! Two places need C-compatible number formatting: the `%g` in codebook
//! filenames and reports, and the `--format` option of `hmm show`, which the C
//! passes straight to `printf` with a `long double` argument. That is a
//! format-string injection hole — the caller controls the format — so rather
//! than reproduce it, this parses the subset actually in use and renders it in
//! Rust, rejecting anything it does not understand.
//!
//! Supported: an optional literal prefix and suffix around one conversion of
//! the form `%[-][0][width][.precision][L|l]{g,G,f,F,e,E}`.

use std::error::Error;

/// C's `%g` with the default precision of six significant digits: whichever of
/// `%e` and `%f` is shorter, with trailing zeros removed.
pub fn g(v: f64) -> String {
    g_prec(v, 6)
}

/// C's `%g` with an explicit precision (significant digits).
pub fn g_prec(v: f64, precision: usize) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let p = precision.max(1);
    let exp = v.abs().log10().floor() as i32;
    if exp < -4 || exp >= p as i32 {
        // %e style, trailing zeros trimmed from the mantissa
        let s = format!("{:.*e}", p - 1, v);
        let (mant, e) = s.split_once('e').unwrap();
        let mant = trim_zeros(mant);
        let ev: i32 = e.parse().unwrap_or(0);
        format!("{}e{}{:02}", mant, if ev < 0 { '-' } else { '+' }, ev.abs())
    } else {
        let decimals = (p as i32 - 1 - exp).max(0) as usize;
        trim_zeros(&format!("{:.*}", decimals, v))
    }
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// Renders `value` according to a printf-like `format`.
pub fn render(format: &str, value: f64) -> Result<String, Box<dyn Error>> {
    let Some(start) = format.find('%') else {
        return Ok(format.to_string());
    };
    let prefix = &format[..start];
    let rest = &format[start + 1..];

    let mut chars = rest.char_indices().peekable();
    let mut left = false;
    let mut zero = false;
    while let Some(&(_, c)) = chars.peek() {
        match c {
            '-' => left = true,
            '0' => zero = true,
            '+' | ' ' | '#' => {}
            _ => break,
        }
        chars.next();
    }

    let mut width = String::new();
    while let Some(&(_, c)) = chars.peek() {
        if c.is_ascii_digit() {
            width.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let mut precision: Option<usize> = None;
    if let Some(&(_, '.')) = chars.peek() {
        chars.next();
        let mut p = String::new();
        while let Some(&(_, c)) = chars.peek() {
            if c.is_ascii_digit() {
                p.push(c);
                chars.next();
            } else {
                break;
            }
        }
        precision = Some(p.parse().unwrap_or(0));
    }

    // length modifiers are irrelevant here: everything is f64
    while let Some(&(_, c)) = chars.peek() {
        if c == 'L' || c == 'l' || c == 'h' {
            chars.next();
        } else {
            break;
        }
    }

    let Some((idx, conv)) = chars.next() else {
        return Err(format!("unsupported format {:?}: no conversion", format).into());
    };
    let suffix = &rest[idx + conv.len_utf8()..];

    let body = match conv {
        'g' | 'G' => g_prec(value, precision.unwrap_or(6)),
        'f' | 'F' => format!("{:.*}", precision.unwrap_or(6), value),
        'e' | 'E' => format!("{:.*e}", precision.unwrap_or(6), value),
        other => {
            return Err(format!(
                "unsupported conversion '%{}' in {:?}; use g, f or e",
                other, format
            )
            .into())
        }
    };
    let body = if conv.is_uppercase() {
        body.to_uppercase()
    } else {
        body
    };

    let w: usize = width.parse().unwrap_or(0);
    let padded = if body.len() >= w {
        body
    } else if left {
        format!("{:<width$}", body, width = w)
    } else if zero {
        let (sign, digits) = match body.strip_prefix('-') {
            Some(d) => ("-", d),
            None => ("", body.as_str()),
        };
        format!("{}{:0>width$}", sign, digits, width = w - sign.len())
    } else {
        format!("{:>width$}", body, width = w)
    };

    Ok(format!("{}{}{}", prefix, padded, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The values that actually appear in codebook filenames and reports.
    #[test]
    fn g_matches_the_c_for_the_epsilons_in_use() {
        assert_eq!(g(0.0005), "0.0005");
        assert_eq!(g(0.05), "0.05");
        assert_eq!(g(1e-05), "1e-05");
        assert_eq!(g(0.0), "0");
        assert_eq!(g(123456.0), "123456");
        assert_eq!(g(1234567.0), "1.23457e+06");
    }

    #[test]
    fn render_handles_the_hmm_show_default() {
        assert_eq!(render("%Lg ", 0.25).unwrap(), "0.25 ");
        assert_eq!(render("%Lg ", 1.0).unwrap(), "1 ");
    }

    #[test]
    fn render_honors_width_and_precision() {
        assert_eq!(render("%.3Lf", 1.0 / 3.0).unwrap(), "0.333");
        assert_eq!(render("%8.3Lf", 1.0 / 3.0).unwrap(), "   0.333");
        assert_eq!(render("%-8.3Lf|", 1.0 / 3.0).unwrap(), "0.333   |");
        assert_eq!(render("%08.3Lf", -1.0 / 3.0).unwrap(), "-000.333");
    }

    #[test]
    fn render_rejects_what_it_cannot_do_safely() {
        assert!(render("%s", 1.0).is_err());
        assert!(render("%n", 1.0).is_err());
    }

    #[test]
    fn a_format_without_a_conversion_is_a_literal() {
        assert_eq!(render("plain", 1.0).unwrap(), "plain");
    }
}
