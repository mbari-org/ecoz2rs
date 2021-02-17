#![allow(clippy::many_single_char_names)]

///
/// Rust version of `lpca_r`.
/// LPC analysis as done by `lpca` but with given autocorrelation as input.
///
#[inline]
pub fn lpca_r(p: usize, r: &[f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let mut pe: f64 = 0.;
    let r0 = r[0];
    if 0.0f64 == r0 {
        return (1, pe);
    }

    // get reflection and predictor coefficients:
    pe = r0;
    a[0] = 1.0f64;
    for k in 1..=p {
        let mut sum = 0.0f64;
        for i in 1..=k {
            sum -= a[k - i] * r[i];
        }
        let akk = sum / pe;
        rc[k] = akk;

        // new predictive coefficients:
        a[k] = akk;
        for i in 1..=k >> 1 {
            let ai = a[i];
            let aj = a[k - i];
            a[i] = ai + akk * aj;
            a[k - i] = aj + akk * ai;
        }

        // new prediction error:
        pe *= 1.0f64 - akk * akk;
        if pe <= 0.0f64 {
            return (2, pe);
        }
    }

    (0, pe)
}
