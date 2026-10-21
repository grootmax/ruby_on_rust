//! Ports of array.c: the rotation-count normaliser behind `Array#rotate`
//! and `Array#rotate!` (port unit `array-A-01` of
//! `tool/core_rs/inventory.rb`).
//!
//! `rotate_count` is `static` in C, so it is not public API.  The remaining
//! C (`rb_ary_rotate`, `rb_ary_rotate_m`) calls it as
//! `rb_core_array_rotate_count`, declared hidden in `internal/core_rs.h`.

use core::ffi::c_long;

/// `rotate_count()`: maps any rotation count `cnt` onto `0..len`, so that
/// rotating by the result is the same as rotating by `cnt`.  A negative
/// count rotates the other way: `-1` gives `len - 1`.
///
/// Precondition (as in C): `len > 0`.  Both C callers check it first
/// (`len > 1` in `rb_ary_rotate`, `len > 0` in `rb_ary_rotate_m`); in C a
/// zero `len` divides by zero.  Given `len > 0` no step can overflow:
/// `!cnt` is non-negative when `cnt` is negative, so both remainders lie in
/// `0..len` and `len - r - 1` does too.
pub fn rotate_count(cnt: c_long, len: c_long) -> c_long {
    if cnt < 0 { len - (!cnt % len) - 1 } else { cnt % len }
}

/// C boundary for array.c's `static inline rb_len_t rotate_count(rb_len_t
/// cnt, rb_len_t len)` (`rb_len_t` is `long` on every core_rs target),
/// called through `#define rotate_count rb_core_array_rotate_count`.
///
/// # Safety
///
/// No pointers are involved.  The caller must pass `len > 0`, as both C
/// callers do; `len == 0` aborts the process (a division by zero).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_array_rotate_count(cnt: c_long, len: c_long) -> c_long {
    rotate_count(cnt, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_count_matches_array_rotate_semantics() {
        // [1,2,3,4,5].rotate(n) starts at index rotate_count(n, 5).
        assert_eq!(rotate_count(1, 5), 1);
        assert_eq!(rotate_count(-1, 5), 4);
        assert_eq!(rotate_count(2, 5), 2);
        assert_eq!(rotate_count(-2, 5), 3);
        assert_eq!(rotate_count(13, 5), 3);
        assert_eq!(rotate_count(-13, 5), 2);
        assert_eq!(rotate_count(0, 5), 0);
        assert_eq!(rotate_count(5, 5), 0);
        assert_eq!(rotate_count(-5, 5), 0);
        assert_eq!(rotate_count(7, 1), 0);
        assert_eq!(rotate_count(-7, 1), 0);
    }

    #[test]
    fn rotate_count_extremes_stay_in_range() {
        for &len in &[1, 2, 3, 7, 1 << 20, c_long::MAX - 1, c_long::MAX] {
            for &cnt in &[c_long::MIN, c_long::MIN + 1, -len, -1, 0, 1, len, c_long::MAX] {
                let r = rotate_count(cnt, len);
                assert!(0 <= r && r < len, "cnt={cnt} len={len} r={r}");
                // Same residue class as cnt modulo len, computed in i128.
                assert_eq!((cnt as i128 - r as i128).rem_euclid(len as i128), 0, "cnt={cnt} len={len}");
            }
        }
    }
}
