//! Rust port of leaf functions from strftime.c (strftime-A-01).

use core::ffi::{c_char, c_int, c_long};

pub const LEFT: u32 = 0;
pub const CHCASE: u32 = 1;
pub const LOWER: u32 = 2;
pub const UPPER: u32 = 3;

#[inline]
pub fn bit_of(n: u32) -> c_int {
    (1u32 << n) as c_int
}

pub fn min(a: c_int, b: c_int) -> c_int {
    if a < b { a } else { b }
}

pub fn max(a: c_int, b: c_int) -> c_int {
    if a > b { a } else { b }
}

/// Modifies letter case in `s` of length `i` according to `flags`.
///
/// # Safety
/// `s` must point to a mutable buffer of at least `i` bytes.
pub unsafe fn case_conv(s: *mut c_char, i: isize, flags: c_int) -> *mut c_char {
    if s.is_null() || i <= 0 {
        return s;
    }
    let upper_bit = bit_of(UPPER);
    let lower_bit = bit_of(LOWER);
    let mask = upper_bit | lower_bit;
    // SAFETY: caller guarantees s points to a valid buffer of length i.
    let slice = unsafe { core::slice::from_raw_parts_mut(s as *mut u8, i as usize) };
    match flags & mask {
        b if b == upper_bit => {
            for byte in slice.iter_mut() {
                if byte.is_ascii_lowercase() {
                    *byte = byte.to_ascii_uppercase();
                }
            }
        }
        b if b == lower_bit => {
            for byte in slice.iter_mut() {
                if byte.is_ascii_uppercase() {
                    *byte = byte.to_ascii_lowercase();
                }
            }
        }
        _ => {}
    }
    // SAFETY: returns pointer at end of processed range (s + i)
    unsafe { s.offset(i) }
}

pub fn strftime_size_limit(format_len: usize) -> usize {
    let limit = format_len.saturating_mul(1024 * 1024);
    if limit < format_len {
        format_len
    } else if limit < 1024 {
        1024
    } else {
        limit
    }
}

pub fn isleap(year: c_long) -> c_int {
    ((year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)) as c_int
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
}

pub fn weeknumber(timeptr: &Tm, firstweekday: c_int) -> c_int {
    let mut wday = timeptr.tm_wday;
    if firstweekday == 1 {
        if wday == 0 {
            wday = 6;
        } else {
            wday -= 1;
        }
    }
    let ret = (timeptr.tm_yday + 7 - wday) / 7;
    if ret < 0 { 0 } else { ret }
}

pub fn iso8601wknum(timeptr: &Tm) -> c_int {
    let mut weeknum = weeknumber(timeptr, 1);
    let mut jan1day = timeptr.tm_wday - (timeptr.tm_yday % 7);
    if jan1day < 0 {
        jan1day += 7;
    }

    match jan1day {
        1 => {}
        2 | 3 | 4 => {
            weeknum += 1;
        }
        5 | 6 | 0 => {
            if weeknum == 0 {
                let mut dec31ly = *timeptr;
                dec31ly.tm_year -= 1;
                dec31ly.tm_mon = 11;
                dec31ly.tm_mday = 31;
                dec31ly.tm_wday = if jan1day == 0 { 6 } else { jan1day - 1 };
                dec31ly.tm_yday = 364 + isleap((dec31ly.tm_year + 1900) as c_long);
                weeknum = iso8601wknum(&dec31ly);
            }
        }
        _ => {}
    }

    if timeptr.tm_mon == 11 {
        let wday = timeptr.tm_wday;
        let mday = timeptr.tm_mday;
        if (wday == 1 && (mday >= 29 && mday <= 31))
            || (wday == 2 && (mday == 30 || mday == 31))
            || (wday == 3 && mday == 31)
        {
            weeknum = 1;
        }
    }

    weeknum
}

// C FFI Boundary Exports
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_strftime_min(a: c_int, b: c_int) -> c_int {
    min(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_strftime_max(a: c_int, b: c_int) -> c_int {
    max(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_strftime_case_conv(s: *mut c_char, i: isize, flags: c_int) -> *mut c_char {
    // SAFETY: forwarded from C.
    unsafe { case_conv(s, i, flags) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_strftime_strftime_size_limit(format_len: usize) -> usize {
    strftime_size_limit(format_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_strftime_isleap(year: c_long) -> c_int {
    isleap(year)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_strftime_iso8601wknum(timeptr: *const Tm) -> c_int {
    if timeptr.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees non-null timeptr points to valid Tm struct.
    let tm = unsafe { &*timeptr };
    iso8601wknum(tm)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_strftime_weeknumber(timeptr: *const Tm, firstweekday: c_int) -> c_int {
    if timeptr.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees non-null timeptr points to valid Tm struct.
    let tm = unsafe { &*timeptr };
    weeknumber(tm, firstweekday)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max() {
        assert_eq!(min(10, 20), 10);
        assert_eq!(min(-5, 5), -5);
        assert_eq!(max(10, 20), 20);
        assert_eq!(max(-5, 5), 5);
    }

    #[test]
    fn test_isleap() {
        assert_eq!(isleap(2000), 1);
        assert_eq!(isleap(1900), 0);
        assert_eq!(isleap(2024), 1);
        assert_eq!(isleap(2023), 0);
    }

    #[test]
    fn test_strftime_size_limit() {
        assert_eq!(strftime_size_limit(0), 1024);
        assert_eq!(strftime_size_limit(10), 10 * 1024 * 1024);
    }

    #[test]
    fn test_case_conv() {
        let mut buf = *b"Hello World!";
        let ptr = buf.as_mut_ptr() as *mut c_char;
        unsafe {
            let res = case_conv(ptr, 12, bit_of(UPPER));
            assert_eq!(&buf, b"HELLO WORLD!");
            assert_eq!(res, ptr.add(12));

            case_conv(ptr, 12, bit_of(LOWER));
            assert_eq!(&buf, b"hello world!");
        }
    }

    #[test]
    fn test_iso8601wknum() {
        let mut tm = Tm::default();
        tm.tm_year = 124; // 2024
        tm.tm_mon = 0;   // Jan
        tm.tm_mday = 1;  // Jan 1, 2024 (Monday)
        tm.tm_wday = 1;  // Mon
        tm.tm_yday = 0;
        assert_eq!(iso8601wknum(&tm), 1);
    }
}
