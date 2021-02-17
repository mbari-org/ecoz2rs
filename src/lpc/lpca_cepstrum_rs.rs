#![allow(clippy::many_single_char_names)]

///
/// Get cepstral coefficients corresponding to the given prediction vector.
/// Ref: Papamichalis (1987). Practical approaches to speech coding.
///
#[inline]
pub fn lpca_get_cepstrum(p: usize, r0: f64, a: &[f64], q: usize, cepstrum: &mut [f64]) {
    assert!(p < q);
    cepstrum[0] = r0.ln();
    for m in 1..=p {
        let mut sum = -a[m];
        for k in 1..m {
            sum += -((m - k) as f64) * a[k] * cepstrum[m - k];
        }
        cepstrum[m] = sum / (m as f64);
    }
    for m in p + 1..q {
        let mut sum = 0f64;
        for k in 1..=p {
            sum += -((m - k) as f64) * a[k] * cepstrum[m - k];
        }
        cepstrum[m] = sum / (m as f64);
    }
}
