#![allow(clippy::many_single_char_names)]

///
/// Get cepstral coefficients corresponding to the given prediction vector.
/// Ref: Papamichalis (1987), p. 129.
///
/// ## Arguments:
///
/// * `gain`     - Gain of the system (i.e., `sqrt(prediction_error)`).
/// * `p`        - Prediction order.
/// * `a`        - Prediction coefficients `a[0 ..= p]`.
///                (Note that `a[0]` is always 1, with
///                `a[1]` .. `a[p]` being the actual coefficients.)
/// * `q`        - Number of cepstral coefficients to generate; `q > p`.
/// * `cepstrum` - cepstral coefficients are stored here (`cepstrum[0 .. q]`).
///
#[inline]
pub fn lpca_get_cepstrum(gain: f64, p: usize, a: &[f64], q: usize, cepstrum: &mut [f64]) {
    debug_assert!(p < q);
    cepstrum[0] = gain.ln();
    cepstrum[1] = -a[1];
    for i in 2..=p {
        let mut sum = a[i];
        for k in 1..i {
            sum += ((i - k) as f64) * cepstrum[i - k] * a[k];
        }
        cepstrum[i] = -sum / (i as f64);
    }
    for i in p + 1..q {
        let mut sum = 0f64;
        for k in 1..=p {
            sum += ((i - k) as f64) * cepstrum[i - k] * a[k];
        }
        cepstrum[i] = -sum / (i as f64);
    }
}
