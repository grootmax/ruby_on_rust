//! Ports of complex.c: the scanner that `String#to_c` and `Complex("...")`
//! use to split a string into the number fragments of a complex literal
//! (port unit `complex-A-01` of `tool/core_rs/inventory.rb`).
//!
//! These are `static` functions in C, so they are not public API.  The ones
//! that the remaining C (`read_comp`, `parse_comp`) still calls are exported
//! as `rb_core_complex_<name>` and declared hidden in `internal/core_rs.h`,
//! which keeps them out of libruby's dynamic symbol table.  `read_digits`,
//! `islettere`, `read_num` and `read_den` are only called from inside this
//! unit and stay private to this module.
//!
//! The C functions walk a NUL-terminated string through `const char **s`
//! and copy the accepted characters to an output buffer through `char **b`.
//! The safe core below does the same with a [`Cursor`] over the input bytes
//! (including the terminating NUL) and an [`Out`] sink for the copies.

use core::ffi::{c_char, c_int, CStr};

unsafe extern "C" {
    // The C library's classification functions, called exactly as the C
    // code does (isspace() depends on the LC_CTYPE locale Ruby sets).
    fn isspace(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
}

/// Input position: `s[pos]` is `**s` in C.  `s` ends with the NUL.
pub struct Cursor<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// A cursor at the start of `s`, which must include the NUL terminator.
    pub fn new(s: &'a [u8]) -> Self {
        Cursor { s, pos: 0 }
    }

    /// Number of bytes consumed since the start.
    

    /// `**s` as a C `char` promoted to `int` (sign extended where `char`
    /// is signed, exactly like the C).  Past the terminator it reads NUL,
    /// which the C never does because nothing consumes the terminator.
    fn cur(&self) -> c_int {
        self.s.get(self.pos).copied().unwrap_or(0) as c_char as c_int
    }

    /// `**s` as an unsigned byte.
    fn byte(&self) -> u8 {
        self.s.get(self.pos).copied().unwrap_or(0)
    }
}

/// Where the C writes `**b = c; (*b)++;` and undoes it with `(*b)--`.
pub trait Out {
    /// `**b = c; (*b)++;`
    fn put(&mut self, c: u8);
    /// `(*b)--;`
    fn back(&mut self);
}

/// `issign()`.
pub fn issign(c: c_int) -> bool {
    c == b'-' as c_int || c == b'+' as c_int
}

/// `isdecimal()`: `isdigit((unsigned char)c)`, returning isdigit()'s own
/// non-zero value (glibc returns a table bit such as 2048, not 1).
pub fn isdecimal_raw(c: c_int) -> c_int {
    // LLVM rewrites a direct isdigit() call into `c - '0' < 10`, which
    // returns 1 instead of the C library's value; call it opaquely.
    let f: unsafe extern "C" fn(c_int) -> c_int = core::hint::black_box(isdigit);
    // SAFETY: isdigit() accepts any unsigned char value.
    unsafe { f(c as u8 as c_int) }
}

/// `isdecimal()` as a truth value.
pub fn isdecimal(c: c_int) -> bool {
    isdecimal_raw(c) != 0
}

/// `islettere()`.
fn islettere(c: c_int) -> bool {
    c == b'e' as c_int || c == b'E' as c_int
}

/// `isimagunit()`.
pub fn isimagunit(c: c_int) -> bool {
    c == b'i' as c_int || c == b'I' as c_int || c == b'j' as c_int || c == b'J' as c_int
}

/// `read_sign()`: copies a leading `+`/`-` and returns it, else `'?'`.
pub fn read_sign(s: &mut Cursor, b: &mut impl Out) -> c_int {
    let mut sign = b'?' as c_int;
    if issign(s.cur()) {
        // `sign = **b = **s;` stores a char, then reads it back as int.
        let c = s.byte();
        b.put(c);
        sign = c as c_char as c_int;
        s.pos += 1;
    }
    sign
}

/// `read_digits()`: digits with single `_` separators.  On a trailing `_`
/// the C steps `*s` back over the underscores, leaving it on the last
/// digit (which has already been copied); this is reproduced as is.
fn read_digits(s: &mut Cursor, strict: bool, b: &mut impl Out) -> bool {
    let mut us = true;

    if !isdecimal(s.cur()) {
        return false;
    }
    while isdecimal(s.cur()) || s.byte() == b'_' {
        if s.byte() == b'_' {
            if us {
                if strict {
                    return false;
                }
                break;
            }
            us = true;
        } else {
            b.put(s.byte());
            us = false;
        }
        s.pos += 1;
    }
    if us {
        // do { (*s)--; } while (**s == '_');  The first byte of this call
        // is a digit, so this never moves before it.
        loop {
            s.pos -= 1;
            if s.byte() != b'_' {
                break;
            }
        }
    }
    true
}

/// `read_num()`: digits, an optional fraction and an optional exponent.
fn read_num(s: &mut Cursor, strict: bool, b: &mut impl Out) -> bool {
    if s.byte() != b'.' && !read_digits(s, strict, b) {
        return false;
    }
    if s.byte() == b'.' {
        b.put(s.byte());
        s.pos += 1;
        if !read_digits(s, strict, b) {
            b.back();
            return false;
        }
    }
    if islettere(s.cur()) {
        b.put(s.byte());
        s.pos += 1;
        read_sign(s, b);
        if !read_digits(s, strict, b) {
            b.back();
            return false;
        }
    }
    true
}

/// `read_den()`.
fn read_den(s: &mut Cursor, strict: bool, b: &mut impl Out) -> bool {
    read_digits(s, strict, b)
}

/// `read_rat_nos()`: an unsigned number with an optional `/denominator`.
pub fn read_rat_nos(s: &mut Cursor, strict: bool, b: &mut impl Out) -> bool {
    if !read_num(s, strict, b) {
        return false;
    }
    if s.byte() == b'/' {
        b.put(s.byte());
        s.pos += 1;
        if !read_den(s, strict, b) {
            b.back();
            return false;
        }
    }
    true
}

/// `read_rat()`: an optional sign, then `read_rat_nos()`.
pub fn read_rat(s: &mut Cursor, strict: bool, b: &mut impl Out) -> bool {
    read_sign(s, b);
    read_rat_nos(s, strict, b)
}

/// `skip_ws()`: `while (isspace((unsigned char)**s)) (*s)++;`
pub fn skip_ws(s: &mut Cursor) {
    // SAFETY: isspace() accepts any unsigned char value.
    while unsafe { isspace(s.byte() as c_int) } != 0 {
        s.pos += 1;
    }
}

// ---- C ABI ------------------------------------------------------------------

/// The C output pointer `*b`, written through exactly as the C does.
struct COut {
    b: *mut c_char,
}

impl Out for COut {
    fn put(&mut self, c: u8) {
        // SAFETY: the callers' contract (see the exported functions): the
        // C caller's buffer has room wherever the C code would write.
        unsafe {
            *self.b = c as c_char;
            self.b = self.b.add(1);
        }
    }
    fn back(&mut self) {
        // SAFETY: only undoes a put() of the same call chain.
        self.b = unsafe { self.b.sub(1) };
    }
}

/// Runs `f` on the C string `*s` and the output pointer `*b`, then stores
/// the advanced pointers back, as the C functions do through `s` and `b`.
///
/// # Safety
/// `s` and `b` are valid; `*s` points into a NUL-terminated string; `*b`
/// has room for every byte the C version of `f` would write (the C callers
/// size it as the whole input string plus its NUL).
unsafe fn with_c<R>(s: *mut *const c_char, b: *mut *mut c_char, f: impl FnOnce(&mut Cursor, &mut COut) -> R) -> R {
    // SAFETY: per the contract.
    let (start, bytes) = unsafe { (*s, CStr::from_ptr(*s).to_bytes_with_nul()) };
    let mut cur = Cursor::new(bytes);
    // SAFETY: per the contract.
    let mut out = COut { b: unsafe { *b } };
    let r = f(&mut cur, &mut out);
    // SAFETY: pos <= the string length, so the pointer stays in the string.
    unsafe {
        *s = start.add(cur.pos);
        *b = out.b;
    }
    r
}

/// Port of `issign()` (complex.c).
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_complex_issign(c: c_int) -> c_int {
    issign(c) as c_int
}

/// Port of `isdecimal()` (complex.c).
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_complex_isdecimal(c: c_int) -> c_int {
    isdecimal_raw(c)
}

/// Port of `isimagunit()` (complex.c).
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_complex_isimagunit(c: c_int) -> c_int {
    isimagunit(c) as c_int
}

/// Port of `read_sign()` (complex.c).
///
/// # Safety
/// See [`with_c`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_complex_read_sign(s: *mut *const c_char, b: *mut *mut c_char) -> c_int {
    // SAFETY: forwarded contract.
    unsafe { with_c(s, b, |s, b| read_sign(s, b)) }
}

/// Port of `read_rat_nos()` (complex.c).
///
/// # Safety
/// See [`with_c`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_complex_read_rat_nos(s: *mut *const c_char, strict: c_int, b: *mut *mut c_char) -> c_int {
    // SAFETY: forwarded contract.
    unsafe { with_c(s, b, |s, b| read_rat_nos(s, strict != 0, b)) as c_int }
}

/// Port of `read_rat()` (complex.c).
///
/// # Safety
/// See [`with_c`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_complex_read_rat(s: *mut *const c_char, strict: c_int, b: *mut *mut c_char) -> c_int {
    // SAFETY: forwarded contract.
    unsafe { with_c(s, b, |s, b| read_rat(s, strict != 0, b)) as c_int }
}

/// Port of `skip_ws()` (complex.c).
///
/// # Safety
/// `s` is valid and `*s` points into a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_complex_skip_ws(s: *mut *const c_char) {
    // SAFETY: per the contract.
    let (start, bytes) = unsafe { (*s, CStr::from_ptr(*s).to_bytes_with_nul()) };
    let mut cur = Cursor::new(bytes);
    skip_ws(&mut cur);
    // SAFETY: pos <= the string length.
    unsafe { *s = start.add(cur.pos) };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct VecOut {
        buf: [u8; 64],
        len: usize,
    }
    impl Out for VecOut {
        fn put(&mut self, c: u8) {
            self.buf[self.len] = c;
            self.len += 1;
        }
        fn back(&mut self) {
            self.len -= 1;
        }
    }

    /// (accepted, bytes consumed, bytes copied)
    fn rat(input: &str, strict: bool) -> (bool, usize, &'static str) {
        let mut v = Vec::from(input.as_bytes());
        v.push(0);
        let mut s = Cursor::new(&v);
        let mut out = VecOut { buf: [0; 64], len: 0 };
        let ok = read_rat(&mut s, strict, &mut out);
        let copied = String::from_utf8(out.buf[..out.len].to_vec()).unwrap();
        (ok, s.pos(), Box::leak(copied.into_boxed_str()))
    }

    #[test]
    fn rationals() {
        assert_eq!(rat("12", false), (true, 2, "12"));
        assert_eq!(rat("-1.5e+3/4x", false), (true, 9, "-1.5e+3/4"));
        assert_eq!(rat("1_000", false), (true, 5, "1000"));
        // "1__0": the second '_' stops the scan, then *s steps back to the '1'.
        assert_eq!(rat("1__0", false), (true, 0, "1"));
        assert_eq!(rat("1__0", true), (false, 2, "1"));
        // A trailing '_' leaves the position on the last digit, as in C.
        assert_eq!(rat("12_", false), (true, 1, "12"));
        assert_eq!(rat("1.", false), (false, 2, "1"));
        assert_eq!(rat("1e", false), (false, 2, "1"));
        assert_eq!(rat("1/", false), (false, 2, "1"));
        assert_eq!(rat(".5", false), (true, 2, ".5"));
        assert_eq!(rat("+", false), (false, 1, "+"));
        assert_eq!(rat("", false), (false, 0, ""));
    }

    #[test]
    fn predicates_and_ws() {
        assert!(issign(b'+' as c_int) && issign(b'-' as c_int) && !issign(b'*' as c_int));
        assert!(isimagunit(b'j' as c_int) && !isimagunit(b'k' as c_int));
        assert!(isdecimal(b'7' as c_int) && !isdecimal(b'a' as c_int) && !isdecimal(0xb9u8 as c_char as c_int));
        let v = b" \t\n1\0";
        let mut s = Cursor::new(v);
        skip_ws(&mut s);
        assert_eq!(s.pos(), 3);
    }
}
