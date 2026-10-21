//! Ports of `numeric.c` helper functions (port unit `numeric-A-01`).

use core::ffi::c_int;
use crate::ffi::value::{SIGNED_VALUE, VALUE};

unsafe extern "C" {
    fn floor(x: f64) -> f64;
    fn ceil(x: f64) -> f64;
    fn fmod(x: f64, y: f64) -> f64;
    fn modf(x: f64, iptr: *mut f64) -> f64;
    fn rb_dbl_long_hash(d: f64) -> core::ffi::c_long;
}

/// Safe core of `round(double x)`.
pub fn round_rs(mut x: f64) -> f64 {
    if x > 0.0 {
        let f = unsafe { floor(x) };
        x = f + if x - f >= 0.5 { 1.0 } else { 0.0 };
    } else if x < 0.0 {
        let f = unsafe { ceil(x) };
        x = f - if f - x >= 0.5 { 1.0 } else { 0.0 };
    }
    x
}

/// C boundary for `round`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn round(x: f64) -> f64 {
    round_rs(x)
}

/// Safe core of `round_half_up(double x, double s)`.
pub fn round_half_up_rs(x: f64, s: f64) -> f64 {
    let xs = x * s;
    let mut f = round_rs(xs);
    if s == 1.0 {
        return f;
    }
    if x > 0.0 {
        if ((f + 0.5) / s) <= x {
            f += 1.0;
        }
        f
    } else {
        if ((f - 0.5) / s) >= x {
            f -= 1.0;
        }
        f
    }
}

/// C boundary for `round_half_up`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_round_half_up(x: f64, s: f64) -> f64 {
    round_half_up_rs(x, s)
}

/// Safe core of `round_half_down(double x, double s)`.
pub fn round_half_down_rs(x: f64, s: f64) -> f64 {
    let xs = x * s;
    let mut f = round_rs(xs);
    if x > 0.0 {
        if ((f - 0.5) / s) >= x {
            f -= 1.0;
        }
        f
    } else {
        if ((f + 0.5) / s) <= x {
            f += 1.0;
        }
        f
    }
}

/// C boundary for `round_half_down`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_round_half_down(x: f64, s: f64) -> f64 {
    round_half_down_rs(x, s)
}

/// Safe core of `round_half_even(double x, double s)`.
pub fn round_half_even_rs(mut x: f64, s: f64) -> f64 {
    let mut u = 0.0;
    let v = unsafe { modf(x, &mut u) };
    let us = u * s;
    let vs = v * s;

    if x > 0.0 {
        let f = unsafe { floor(vs) };
        let uf = us + f;
        let mut d = vs - f;
        if d > 0.5 {
            d = 1.0;
        } else if d == 0.5 || (((uf + 0.5) / s) <= x) {
            d = unsafe { fmod(uf, 2.0) };
        } else {
            d = 0.0;
        }
        x = f + d;
    } else if x < 0.0 {
        let f = unsafe { ceil(vs) };
        let uf = us + f;
        let mut d = f - vs;
        if d > 0.5 {
            d = 1.0;
        } else if d == 0.5 || (((uf - 0.5) / s) >= x) {
            d = unsafe { fmod(-uf, 2.0) };
        } else {
            d = 0.0;
        }
        x = f - d;
    }
    us + x
}

/// C boundary for `round_half_even`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_round_half_even(x: f64, s: f64) -> f64 {
    round_half_even_rs(x, s)
}

/// Safe core of `double_div_double(double x, double y)`.
pub fn double_div_double_rs(x: f64, y: f64) -> f64 {
    if y != 0.0 {
        x / y
    } else if x == 0.0 {
        f64::NAN
    } else {
        let z = if y.is_sign_negative() { -1.0 } else { 1.0 };
        x * z * f64::INFINITY
    }
}

/// C boundary for `double_div_double`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_double_div_double(x: f64, y: f64) -> f64 {
    double_div_double_rs(x, y)
}

#[inline]
fn st2fix(i: core::ffi::c_long) -> VALUE {
    let mut x = i;
    if x >= 0 {
        x &= crate::ffi::value::FIXNUM_MAX as core::ffi::c_long;
    } else {
        x |= crate::ffi::value::FIXNUM_MIN as core::ffi::c_long;
    }
    crate::ffi::value::INT2FIX(x)
}

/// Safe core of `rb_dbl_hash(double d)`.
pub fn dbl_hash_rs(d: f64) -> VALUE {
    let h = unsafe { rb_dbl_long_hash(d) };
    st2fix(h)
}

/// C boundary for `rb_dbl_hash`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_rb_dbl_hash(d: f64) -> VALUE {
    dbl_hash_rs(d)
}

/// Safe core of `int_round_half_even(SIGNED_VALUE x, SIGNED_VALUE y)`.
pub fn int_round_half_even_rs(x: SIGNED_VALUE, y: SIGNED_VALUE) -> SIGNED_VALUE {
    if y == 0 {
        return 0;
    }
    let mut z = (x.wrapping_add(y / 2)).wrapping_div(y);
    if (z.wrapping_mul(y).wrapping_sub(x)).wrapping_mul(2) == y {
        z &= !1;
    }
    z.wrapping_mul(y)
}

/// C boundary for `int_round_half_even`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_int_round_half_even(x: SIGNED_VALUE, y: SIGNED_VALUE) -> SIGNED_VALUE {
    int_round_half_even_rs(x, y)
}

/// Safe core of `int_round_half_up(SIGNED_VALUE x, SIGNED_VALUE y)`.
pub fn int_round_half_up_rs(x: SIGNED_VALUE, y: SIGNED_VALUE) -> SIGNED_VALUE {
    if y == 0 {
        return 0;
    }
    (x.wrapping_add(y / 2)).wrapping_div(y).wrapping_mul(y)
}

/// C boundary for `int_round_half_up`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_int_round_half_up(x: SIGNED_VALUE, y: SIGNED_VALUE) -> SIGNED_VALUE {
    int_round_half_up_rs(x, y)
}

/// Safe core of `int_round_half_down(SIGNED_VALUE x, SIGNED_VALUE y)`.
pub fn int_round_half_down_rs(x: SIGNED_VALUE, y: SIGNED_VALUE) -> SIGNED_VALUE {
    if y == 0 {
        return 0;
    }
    (x.wrapping_add(y / 2).wrapping_sub(1)).wrapping_div(y).wrapping_mul(y)
}

/// C boundary for `int_round_half_down`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_int_round_half_down(x: SIGNED_VALUE, y: SIGNED_VALUE) -> SIGNED_VALUE {
    int_round_half_down_rs(x, y)
}

/// Safe core of `float_round_overflow(int ndigits, int binexp)`.
pub fn float_round_overflow_rs(ndigits: c_int, binexp: c_int) -> c_int {
    const FLOAT_DIG: c_int = 17; // DBL_DIG + 2
    let threshold = FLOAT_DIG - if binexp > 0 { binexp / 4 } else { binexp / 3 - 1 };
    if ndigits >= threshold {
        1
    } else {
        0
    }
}

/// C boundary for `float_round_overflow`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_float_round_overflow(ndigits: c_int, binexp: c_int) -> c_int {
    float_round_overflow_rs(ndigits, binexp)
}

/// Safe core of `float_round_underflow(int ndigits, int binexp)`.
pub fn float_round_underflow_rs(ndigits: c_int, binexp: c_int) -> c_int {
    let threshold = -if binexp > 0 { binexp / 3 + 1 } else { binexp / 4 };
    if ndigits < threshold {
        1
    } else {
        0
    }
}

/// C boundary for `float_round_underflow`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_numeric_float_round_underflow(ndigits: c_int, binexp: c_int) -> c_int {
    float_round_underflow_rs(ndigits, binexp)
}

/// Safe core of `ruby_float_step_size(double beg, double end, double unit, int excl)`.
pub fn float_step_size_rs(beg: f64, end: f64, unit: f64, excl: c_int) -> f64 {
    const EPSILON: f64 = core::f64::EPSILON;

    if unit == 0.0 {
        return f64::INFINITY;
    }
    if unit.is_infinite() {
        return if unit > 0.0 {
            if beg <= end { 1.0 } else { 0.0 }
        } else {
            if beg >= end { 1.0 } else { 0.0 }
        };
    }
    let mut n = (end - beg) / unit;
    let mut err = (beg.abs() + end.abs() + (end - beg).abs()) / unit.abs() * EPSILON;
    if err > 0.5 {
        err = 0.5;
    }
    if excl != 0 {
        if n <= 0.0 {
            return 0.0;
        }
        if n < 1.0 {
            n = 0.0;
        } else {
            n = unsafe { floor(n - err) };
        }
        let d = ((n + 1.0) * unit) + beg;
        if beg < end {
            if d < end {
                n += 1.0;
            }
        } else if beg > end {
            if d > end {
                n += 1.0;
            }
        }
    } else {
        if n < 0.0 {
            return 0.0;
        }
        n = unsafe { floor(n + err) };
        let d = ((n + 1.0) * unit) + beg;
        if beg < end {
            if d <= end {
                n += 1.0;
            }
        } else if beg > end {
            if d >= end {
                n += 1.0;
            }
        }
    }
    n + 1.0
}

/// C boundary for `ruby_float_step_size`.
/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ruby_float_step_size(beg: f64, end: f64, unit: f64, excl: c_int) -> f64 {
    float_step_size_rs(beg, end, unit, excl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round() {
        assert_eq!(round_rs(1.4), 1.0);
        assert_eq!(round_rs(1.5), 2.0);
        assert_eq!(round_rs(-1.4), -1.0);
        assert_eq!(round_rs(-1.5), -2.0);
        assert_eq!(round_rs(0.0), 0.0);
    }

    #[test]
    fn test_round_half_up() {
        assert_eq!(round_half_up_rs(1.5, 1.0), 2.0);
        assert_eq!(round_half_up_rs(2.5, 1.0), 3.0);
        assert_eq!(round_half_up_rs(-1.5, 1.0), -2.0);
        assert_eq!(round_half_up_rs(-2.5, 1.0), -3.0);
    }

    #[test]
    fn test_round_half_down() {
        assert_eq!(round_half_down_rs(1.5, 1.0), 1.0);
        assert_eq!(round_half_down_rs(2.5, 1.0), 2.0);
        assert_eq!(round_half_down_rs(-1.5, 1.0), -1.0);
        assert_eq!(round_half_down_rs(-2.5, 1.0), -2.0);
    }

    #[test]
    fn test_round_half_even() {
        assert_eq!(round_half_even_rs(1.5, 1.0), 2.0);
        assert_eq!(round_half_even_rs(2.5, 1.0), 2.0);
        assert_eq!(round_half_even_rs(-1.5, 1.0), -2.0);
        assert_eq!(round_half_even_rs(-2.5, 1.0), -2.0);
    }

    #[test]
    fn test_double_div_double() {
        assert_eq!(double_div_double_rs(10.0, 2.0), 5.0);
        assert!(double_div_double_rs(0.0, 0.0).is_nan());
        assert_eq!(double_div_double_rs(1.0, 0.0), f64::INFINITY);
        assert_eq!(double_div_double_rs(1.0, -0.0), f64::NEG_INFINITY);
    }

    #[test]
    fn test_int_round_half() {
        assert_eq!(int_round_half_even_rs(15, 10), 20);
        assert_eq!(int_round_half_even_rs(25, 10), 20);
        assert_eq!(int_round_half_up_rs(15, 10), 20);
        assert_eq!(int_round_half_up_rs(25, 10), 30);
        assert_eq!(int_round_half_down_rs(15, 10), 10);
        assert_eq!(int_round_half_down_rs(25, 10), 20);
    }

    #[test]
    fn test_float_round_overflow_underflow() {
        assert_eq!(float_round_overflow_rs(100, 10), 1);
        assert_eq!(float_round_overflow_rs(1, 10), 0);
        assert_eq!(float_round_underflow_rs(-100, 10), 1);
        assert_eq!(float_round_underflow_rs(1, 10), 0);
    }

    #[test]
    fn test_float_step_size() {
        assert_eq!(float_step_size_rs(1.0, 10.0, 1.0, 0), 10.0);
        assert_eq!(float_step_size_rs(1.0, 10.0, 1.0, 1), 9.0);
        assert_eq!(float_step_size_rs(1.0, 10.0, 0.0, 0), f64::INFINITY);
    }
}
