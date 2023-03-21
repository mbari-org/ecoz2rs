#![allow(clippy::many_single_char_names)]

extern crate libc;

use self::libc::{c_double, c_int};

extern "C" {
    fn ecoz2_lpca(
        x: *mut c_double,
        n: c_int,
        p: c_int,
        r: *mut c_double,
        rc: *mut c_double,
        a: *mut c_double,
        pe: *mut c_double,
    ) -> c_int;
}

pub fn lpca(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let n = x.len();
    let mut pe: c_double = 0f64;

    unsafe {
        let res = ecoz2_lpca(
            x.as_ptr() as *mut c_double,
            n as c_int,
            p as c_int,
            r.as_ptr() as *mut c_double,
            rc.as_ptr() as *mut c_double,
            a.as_ptr() as *mut c_double,
            &mut pe,
        );

        (res, pe)
    }
}
