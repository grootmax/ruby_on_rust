//! Rust port of leaf functions from sprintf.c (sprintf-A-01).

use core::ffi::{c_char, c_int, c_ulong};

pub const FSHARP: c_int = 1;
pub const FMINUS: c_int = 2;
pub const FPLUS: c_int = 4;
pub const FZERO: c_int = 8;
pub const FSPACE: c_int = 16;
pub const FWIDTH: c_int = 32;
pub const FPREC: c_int = 64;

static LOWER_HEXDIGITS: &[u8; 16] = b"0123456789abcdef";

/// Sign bit representation for integer bases.
///
/// # Safety
/// If `base == 16`, `p` must point to a valid null-terminated C string or byte.
pub unsafe fn sign_bits(base: c_int, p: *const c_char) -> c_char {
    let mut c = b'.';
    match base {
        16 => {
            if !p.is_null() && unsafe { *p as u8 } == b'X' {
                c = b'F';
            } else {
                c = b'f';
            }
        }
        8 => c = b'7',
        2 => c = b'1',
        _ => {}
    }
    c as c_char
}

/// Converts an unsigned long into digits written backwards into `endp`.
///
/// # Safety
/// `endp` must point into a writable buffer with sufficient capacity to hold the digits before `endp`.
pub unsafe fn ultoa_rs(val: c_ulong, mut endp: *mut c_char, base: c_int, octzero: bool) -> *mut c_char {
    match base {
        10 => {
            let mut v = val;
            if v == 0 {
                // SAFETY: caller guarantees endp points into a valid buffer.
                unsafe {
                    endp = endp.sub(1);
                    *endp = b'0' as c_char;
                }
                return endp;
            }
            while v > 0 {
                let digit = (v % 10) as u8;
                // SAFETY: caller guarantees endp points into a valid buffer.
                unsafe {
                    endp = endp.sub(1);
                    *endp = (b'0' + digit) as c_char;
                }
                v /= 10;
            }
        }
        8 => {
            let mut v = val;
            if v == 0 {
                // SAFETY: caller guarantees endp points into a valid buffer.
                unsafe {
                    endp = endp.sub(1);
                    *endp = b'0' as c_char;
                }
            } else {
                while v > 0 {
                    let digit = (v & 7) as u8;
                    // SAFETY: caller guarantees endp points into a valid buffer.
                    unsafe {
                        endp = endp.sub(1);
                        *endp = (b'0' + digit) as c_char;
                    }
                    v >>= 3;
                }
            }
            if octzero && unsafe { *endp as u8 } != b'0' {
                // SAFETY: caller guarantees endp points into a valid buffer.
                unsafe {
                    endp = endp.sub(1);
                    *endp = b'0' as c_char;
                }
            }
        }
        16 => {
            let mut v = val;
            if v == 0 {
                // SAFETY: caller guarantees endp points into a valid buffer.
                unsafe {
                    endp = endp.sub(1);
                    *endp = b'0' as c_char;
                }
            } else {
                while v > 0 {
                    let digit = (v & 15) as usize;
                    // SAFETY: caller guarantees endp points into a valid buffer.
                    unsafe {
                        endp = endp.sub(1);
                        *endp = LOWER_HEXDIGITS[digit] as c_char;
                    }
                    v >>= 4;
                }
            }
        }
        _ => {}
    }
    endp
}

/// Helper wrapper for ruby_ultoa.
///
/// # Safety
/// `endp` must point into a writable buffer with sufficient space to write digits backwards.
pub unsafe fn ruby_ultoa(val: c_ulong, endp: *mut c_char, base: c_int, flags: c_int) -> *mut c_char {
    let octzero = (flags & FSHARP) != 0;
    // SAFETY: caller guarantees endp valid buffer.
    unsafe { ultoa_rs(val, endp, base, octzero) }
}

/// Format setup building specifier header backwards in `buf`.
///
/// # Safety
/// `buf` must point to writable buffer of length `size`.
pub unsafe fn fmt_setup(
    buf: *mut c_char,
    size: usize,
    c: c_int,
    flags: c_int,
    width: c_int,
    prec: c_int,
) -> *mut c_char {
    // SAFETY: caller guarantees buf points to buffer of size length.
    let mut p = unsafe { buf.add(size) };
    unsafe {
        p = p.sub(1);
        *p = 0;
        p = p.sub(1);
        *p = c as c_char;
    }

    if (flags & FPREC) != 0 {
        p = unsafe { ruby_ultoa(prec as c_ulong, p, 10, 0) };
        unsafe {
            p = p.sub(1);
            *p = b'.' as c_char;
        }
    }

    if (flags & FWIDTH) != 0 {
        p = unsafe { ruby_ultoa(width as c_ulong, p, 10, 0) };
    }

    unsafe {
        if (flags & FSPACE) != 0 {
            p = p.sub(1);
            *p = b' ' as c_char;
        }
        if (flags & FZERO) != 0 {
            p = p.sub(1);
            *p = b'0' as c_char;
        }
        if (flags & FMINUS) != 0 {
            p = p.sub(1);
            *p = b'-' as c_char;
        }
        if (flags & FPLUS) != 0 {
            p = p.sub(1);
            *p = b'+' as c_char;
        }
        if (flags & FSHARP) != 0 {
            p = p.sub(1);
            *p = b'#' as c_char;
        }
        p = p.sub(1);
        *p = b'%' as c_char;
    }
    p
}

// C FFI Boundary Exports
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_sprintf_sign_bits(base: c_int, p: *const c_char) -> c_char {
    // SAFETY: forwarded from C.
    unsafe { sign_bits(base, p) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_sprintf_fmt_setup(
    buf: *mut c_char,
    size: usize,
    c: c_int,
    flags: c_int,
    width: c_int,
    prec: c_int,
) -> *mut c_char {
    // SAFETY: forwarded from C.
    unsafe { fmt_setup(buf, size, c, flags, width, prec) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_sprintf_ruby_ultoa(
    val: c_ulong,
    endp: *mut c_char,
    base: c_int,
    flags: c_int,
) -> *mut c_char {
    // SAFETY: forwarded from C.
    unsafe { ruby_ultoa(val, endp, base, flags) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_bits() {
        let p_x = b"X\0".as_ptr() as *const c_char;
        let p_lower = b"x\0".as_ptr() as *const c_char;
        unsafe {
            assert_eq!(sign_bits(16, p_x), b'F' as c_char);
            assert_eq!(sign_bits(16, p_lower), b'f' as c_char);
            assert_eq!(sign_bits(8, core::ptr::null()), b'7' as c_char);
            assert_eq!(sign_bits(2, core::ptr::null()), b'1' as c_char);
            assert_eq!(sign_bits(10, core::ptr::null()), b'.' as c_char);
        }
    }

    #[test]
    fn test_ruby_ultoa() {
        let mut buf = [0u8; 32];
        let endp = unsafe { buf.as_mut_ptr().add(32) as *mut c_char };

        unsafe {
            let res = ruby_ultoa(12345, endp, 10, 0);
            let len = (endp as usize) - (res as usize);
            let slice = core::slice::from_raw_parts(res as *const u8, len);
            assert_eq!(slice, b"12345");
        }

        unsafe {
            let res = ruby_ultoa(0xfe, endp, 16, 0);
            let len = (endp as usize) - (res as usize);
            let slice = core::slice::from_raw_parts(res as *const u8, len);
            assert_eq!(slice, b"fe");
        }

        unsafe {
            let res = ruby_ultoa(0o755, endp, 8, FSHARP);
            let len = (endp as usize) - (res as usize);
            let slice = core::slice::from_raw_parts(res as *const u8, len);
            assert_eq!(slice, b"0755");
        }
    }

    #[test]
    fn test_fmt_setup() {
        let mut buf = [0u8; 64];
        let buf_ptr = buf.as_mut_ptr() as *mut c_char;
        unsafe {
            let res = fmt_setup(
                buf_ptr,
                64,
                b'd' as c_int,
                FPLUS | FWIDTH | FPREC,
                10,
                5,
            );
            let slice = core::ffi::CStr::from_ptr(res as *const c_char).to_bytes();
            assert_eq!(slice, b"%+10.5d");
        }
    }
}
