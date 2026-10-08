//! Ports of util.c: number scanning and word splitting.
//!
//! Public C API (include/ruby/util.h, include/ruby/internal/ctype.h):
//! `ruby_scan_digits`, `ruby_scan_oct`, `ruby_scan_hex`, `ruby_strtoul`,
//! `ruby_each_words`.  The data table `ruby_digit36_to_number_table` stays
//! in util.c because it is public data; `DIGIT36_TO_NUMBER` below is an
//! identical copy so this module has no link-time dependencies.

use core::ffi::{c_char, c_int, c_ulong, c_void, CStr};

use crate::libc;

/// Identical to `ruby_digit36_to_number_table` in util.c.
#[rustfmt::skip]
static DIGIT36_TO_NUMBER: [i8; 256] = [
    /*     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f */
    /*0*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*1*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*2*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*3*/  0, 1, 2, 3, 4, 5, 6, 7, 8, 9,-1,-1,-1,-1,-1,-1,
    /*4*/ -1,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,
    /*5*/ 25,26,27,28,29,30,31,32,33,34,35,-1,-1,-1,-1,-1,
    /*6*/ -1,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,
    /*7*/ 25,26,27,28,29,30,31,32,33,34,35,-1,-1,-1,-1,-1,
    /*8*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*9*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*a*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*b*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*c*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*d*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*e*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    /*f*/ -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
];

/// `rb_isspace()`: ' ' and '\t'..='\r', independent of the C locale.
#[inline]
fn is_space(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}

/// Result of scanning digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scan {
    /// The value, wrapped modulo 2^(bits of unsigned long) like the C code.
    pub value: c_ulong,
    /// Number of bytes consumed.
    pub consumed: usize,
    /// True if the value did not fit in an unsigned long.
    pub overflow: bool,
}

/// Safe core of `ruby_scan_digits`.  Reads bytes until one is not a digit
/// of `base` (or the input ends) and accumulates their value.
///
/// `base` keeps C's `int` semantics: the digit comparison is signed and the
/// multiplier is `(unsigned long)base`.  Callers must pass 2..=36, as in C.
pub fn scan_digits(bytes: impl IntoIterator<Item = u8>, base: c_int) -> Scan {
    let mul = base as c_ulong;
    let mul_overflow = c_ulong::MAX / mul;
    let mut value: c_ulong = 0;
    let mut consumed = 0;
    let mut overflow = false;

    for byte in bytes {
        let d = DIGIT36_TO_NUMBER[byte as usize] as c_int;
        if d == -1 || base <= d {
            break;
        }
        if mul_overflow < value {
            overflow = true;
        }
        value = value.wrapping_mul(mul);
        let before = value;
        value = value.wrapping_add(d as c_ulong);
        if value < before {
            overflow = true;
        }
        consumed += 1;
    }
    Scan { value, consumed, overflow }
}

/// Bytes starting at `ptr`: exactly `len` of them when `len >= 0`, otherwise
/// an unbounded stream that the caller must stop reading at a terminator.
///
/// # Safety
/// When `len >= 0`, `ptr` must be valid for `len` reads.  When `len < 0`,
/// every byte up to and including the first byte the consumer rejects must
/// be readable (for C strings the NUL terminator always qualifies).
unsafe fn c_bytes(ptr: *const c_char, len: isize) -> CBytes {
    CBytes { ptr: ptr as *const u8, remaining: if len < 0 { None } else { Some(len as usize) } }
}

struct CBytes {
    ptr: *const u8,
    remaining: Option<usize>,
}

impl Iterator for CBytes {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        match &mut self.remaining {
            Some(0) => return None,
            Some(n) => *n -= 1,
            None => {}
        }
        // SAFETY: guaranteed by the contract of c_bytes().
        let b = unsafe { *self.ptr };
        // SAFETY: one past a readable byte is a valid pointer.
        self.ptr = unsafe { self.ptr.add(1) };
        Some(b)
    }
}

/// Port of `ruby_scan_digits()` (util.c).
///
/// # Safety
/// Same contract as the C function: `str` is readable for `len` bytes (or up
/// to a non-digit when `len < 0`); `retlen` and `overflow` are writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ruby_scan_digits(
    str: *const c_char,
    len: isize,
    base: c_int,
    retlen: *mut usize,
    overflow: *mut c_int,
) -> c_ulong {
    // SAFETY: forwarded from this function's contract.
    let scan = scan_digits(unsafe { c_bytes(str, len) }, base);
    // SAFETY: the caller passes writable out-parameters, as in C.
    unsafe {
        *overflow = scan.overflow as c_int;
        *retlen = scan.consumed;
    }
    scan.value
}

/// Port of `ruby_scan_oct()` (util.c).
///
/// # Safety
/// `start` is readable for `len` bytes; `retlen` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ruby_scan_oct(start: *const c_char, len: usize, retlen: *mut usize) -> c_ulong {
    let mut overflow: c_int = 0;
    // SAFETY: forwarded; `(ssize_t)len` as in C.
    unsafe { ruby_scan_digits(start, len as isize, 8, retlen, &mut overflow) }
}

/// Port of `ruby_scan_hex()` (util.c).
///
/// # Safety
/// `start` is readable for `len` bytes; `retlen` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ruby_scan_hex(start: *const c_char, len: usize, retlen: *mut usize) -> c_ulong {
    let mut overflow: c_int = 0;
    // SAFETY: forwarded; `(ssize_t)len` as in C.
    unsafe { ruby_scan_digits(start, len as isize, 16, retlen, &mut overflow) }
}

/// Result of [`strtoul`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrToUl {
    pub value: c_ulong,
    /// Offset from the start of the input to store in `*endptr`.
    pub end: usize,
    /// errno value the C function sets, if any.
    pub errno: Option<c_int>,
}

/// Safe core of `ruby_strtoul` over the bytes of a C string (without its
/// NUL terminator).
pub fn strtoul(s: &[u8], base: c_int) -> StrToUl {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);

    if base < 0 || base == 1 || 36 < base {
        return StrToUl { value: 0, end: 0, errno: Some(libc::EINVAL) };
    }

    let mut i = 0;
    while at(i) != 0 && is_space(at(i)) {
        i += 1;
    }
    let mut sign = 0;
    if at(i) == b'+' {
        sign = 1;
        i += 1;
    } else if at(i) == b'-' {
        sign = -1;
        i += 1;
    }

    // `subject_found` in C: starts at the beginning of the input.
    let mut subject_found = 0;
    let b;
    if at(i) == b'0' {
        subject_found = i + 1;
        if base == 0 || base == 16 {
            if at(i + 1) == b'x' || at(i + 1) == b'X' {
                b = 16;
                i += 2;
            } else {
                b = if base == 0 { 8 } else { 16 };
                i += 1;
            }
        } else {
            b = base;
            i += 1;
        }
    } else {
        b = if base == 0 { 10 } else { base };
    }

    // ruby_scan_digits(str, -1, ...) stops at the NUL terminator, which maps
    // to -1 in the digit table; the slice end plays that role here.
    let rest = s.get(i..).unwrap_or(&[]);
    let scan = scan_digits(rest.iter().copied(), b);
    if 0 < scan.consumed {
        subject_found = i + scan.consumed;
    }

    if scan.overflow {
        return StrToUl { value: c_ulong::MAX, end: subject_found, errno: Some(libc::ERANGE) };
    }
    let value = if sign < 0 { scan.value.wrapping_neg() } else { scan.value };
    StrToUl { value, end: subject_found, errno: None }
}

/// Port of `ruby_strtoul()` (util.c).
///
/// # Safety
/// `str` is a NUL-terminated C string; `endptr` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ruby_strtoul(str: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong {
    if base < 0 || base == 1 || 36 < base {
        // As in C: return before reading the string or touching endptr.
        libc::set_errno(libc::EINVAL);
        return 0;
    }
    // SAFETY: `str` is a C string per the contract.
    let bytes = unsafe { CStr::from_ptr(str) }.to_bytes();
    let r = strtoul(bytes, base);
    if !endptr.is_null() {
        // SAFETY: `r.end <= bytes.len()`, so the pointer stays in the string.
        unsafe { *endptr = str.add(r.end) as *mut c_char };
    }
    if let Some(e) = r.errno {
        libc::set_errno(e);
    }
    r.value
}

/// Safe core of `ruby_each_words`: yields `(offset, len)` of each word, where
/// words are separated by spaces and commas.
pub fn each_words(s: &[u8], mut f: impl FnMut(usize, usize)) {
    let mut i = 0;
    while i < s.len() {
        while i < s.len() && (is_space(s[i]) || s[i] == b',') {
            i += 1;
        }
        if i == s.len() {
            break;
        }
        let start = i;
        while i < s.len() && !is_space(s[i]) && s[i] != b',' {
            i += 1;
        }
        f(start, i - start);
    }
}

/// Port of `ruby_each_words()` (util.c).
///
/// The callback may not unwind; C callbacks that longjmp are fine because no
/// Rust frame here owns a destructor.
///
/// # Safety
/// `str` is null or a NUL-terminated C string; `func` is a valid function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ruby_each_words(
    str: *const c_char,
    func: unsafe extern "C" fn(*const c_char, c_int, *mut c_void),
    arg: *mut c_void,
) {
    if str.is_null() {
        return;
    }
    // SAFETY: non-null `str` is a C string per the contract.
    let bytes = unsafe { CStr::from_ptr(str) }.to_bytes();
    each_words(bytes, |offset, len| {
        // SAFETY: offset is within the string; `(int)` truncation as in C,
        // which assumes no word exceeds INT_MAX.
        unsafe { func(str.add(offset), len as c_int, arg) }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(s: &str, base: c_int) -> (c_ulong, usize, bool) {
        let r = scan_digits(s.bytes(), base);
        (r.value, r.consumed, r.overflow)
    }

    #[test]
    fn scan_digits_basic() {
        assert_eq!(scan("", 10), (0, 0, false));
        assert_eq!(scan("123abc", 10), (123, 3, false));
        assert_eq!(scan("ff", 16), (255, 2, false));
        assert_eq!(scan("FFg", 16), (255, 2, false));
        assert_eq!(scan("777", 8), (511, 3, false));
        assert_eq!(scan("8", 8), (0, 0, false));
        assert_eq!(scan("zz", 36), (35 * 36 + 35, 2, false));
        assert_eq!(scan("1_0", 10), (1, 1, false));
    }

    #[test]
    fn scan_digits_overflow_wraps() {
        let max = c_ulong::MAX;
        let s = format!("{max}");
        assert_eq!(scan(&s, 10), (max, s.len(), false));
        let s1 = format!("{max}0");
        let r = scan_digits(s1.bytes(), 10);
        assert!(r.overflow);
        assert_eq!(r.value, max.wrapping_mul(10));
        assert_eq!(r.consumed, s1.len());
    }

    #[test]
    fn scan_with_len_limit() {
        let p = c"12345".as_ptr();
        let (mut retlen, mut ov) = (0usize, 0);
        let v = unsafe { ruby_scan_digits(p, 3, 10, &mut retlen, &mut ov) };
        assert_eq!((v, retlen, ov), (123, 3, 0));
        let v = unsafe { ruby_scan_digits(p, -1, 10, &mut retlen, &mut ov) };
        assert_eq!((v, retlen, ov), (12345, 5, 0));
        let v = unsafe { ruby_scan_digits(p, 0, 10, &mut retlen, &mut ov) };
        assert_eq!((v, retlen, ov), (0, 0, 0));
        let v = unsafe { ruby_scan_oct(c"0777x".as_ptr(), 5, &mut retlen) };
        assert_eq!((v, retlen), (0o777, 4));
        let v = unsafe { ruby_scan_hex(c"7fZ".as_ptr(), 3, &mut retlen) };
        assert_eq!((v, retlen), (0x7f, 2));
    }

    fn sto(s: &str, base: c_int) -> (c_ulong, usize, Option<c_int>) {
        let r = strtoul(s.as_bytes(), base);
        (r.value, r.end, r.errno)
    }

    #[test]
    fn strtoul_matches_c_semantics() {
        assert_eq!(sto("  42xyz", 10), (42, 4, None));
        assert_eq!(sto("0x1f", 0), (31, 4, None));
        assert_eq!(sto("0X1F", 16), (31, 4, None));
        assert_eq!(sto("017", 0), (15, 3, None));
        assert_eq!(sto("0", 0), (0, 1, None));
        assert_eq!(sto("0x", 0), (0, 1, None)); // subject is just "0"
        assert_eq!(sto("08", 0), (0, 1, None)); // octal: 8 is not a digit
        assert_eq!(sto("xyz", 10), (0, 0, None)); // no subject: end = start
        assert_eq!(sto("-1", 10), (c_ulong::MAX, 2, None));
        assert_eq!(sto("+7", 10), (7, 2, None));
        assert_eq!(sto("z", 36), (35, 1, None));
        assert_eq!(sto("1", 1), (0, 0, Some(libc::EINVAL)));
        assert_eq!(sto("1", 37), (0, 0, Some(libc::EINVAL)));
        assert_eq!(sto("1", -2), (0, 0, Some(libc::EINVAL)));
        let big = format!("{}0", c_ulong::MAX);
        assert_eq!(sto(&big, 10), (c_ulong::MAX, big.len(), Some(libc::ERANGE)));
        let neg_big = format!("-{}0", c_ulong::MAX);
        assert_eq!(sto(&neg_big, 10), (c_ulong::MAX, neg_big.len(), Some(libc::ERANGE)));
    }

    #[test]
    fn strtoul_ffi_sets_endptr_and_errno() {
        let s = c"  123rest";
        let mut end: *mut c_char = core::ptr::null_mut();
        let v = unsafe { ruby_strtoul(s.as_ptr(), &mut end, 10) };
        assert_eq!(v, 123);
        assert_eq!(unsafe { end.offset_from(s.as_ptr()) }, 5);
        let v = unsafe { ruby_strtoul(s.as_ptr(), core::ptr::null_mut(), 10) };
        assert_eq!(v, 123);
    }

    fn words(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        each_words(s.as_bytes(), |o, l| out.push(s[o..o + l].to_string()));
        out
    }

    #[test]
    fn each_words_splits_on_space_and_comma() {
        assert_eq!(words(""), Vec::<String>::new());
        assert_eq!(words("  ,, "), Vec::<String>::new());
        assert_eq!(words("a"), vec!["a"]);
        assert_eq!(words("frozen-string-literal,jit\tgems ,did_you_mean"),
                   vec!["frozen-string-literal", "jit", "gems", "did_you_mean"]);
    }

    #[test]
    fn each_words_ffi_calls_back_with_pointers_into_input() {
        unsafe extern "C" fn collect(p: *const c_char, len: c_int, arg: *mut c_void) {
            let out = unsafe { &mut *(arg as *mut Vec<Vec<u8>>) };
            let w = unsafe { core::slice::from_raw_parts(p as *const u8, len as usize) };
            out.push(w.to_vec());
        }
        let mut out: Vec<Vec<u8>> = Vec::new();
        unsafe { ruby_each_words(c"x, yy\nzzz".as_ptr(), collect, &mut out as *mut _ as *mut c_void) };
        assert_eq!(out, vec![b"x".to_vec(), b"yy".to_vec(), b"zzz".to_vec()]);
        unsafe { ruby_each_words(core::ptr::null(), collect, &mut out as *mut _ as *mut c_void) };
        assert_eq!(out.len(), 3);
    }
}
