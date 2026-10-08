//! Rust port of leaf functions from pack.c (pack-A-01).

use core::ffi::{c_char, c_int};

/// Returns 1 if big endian, 0 otherwise.
pub fn is_bigendian() -> c_int {
    cfg!(target_endian = "big") as c_int
}

/// Skips characters until '\n' or `pend`.
///
/// # Safety
/// If non-null, `p` and `pend` must point into the same valid memory buffer with `p <= pend`.
pub unsafe fn skip_to_eol(p: *const c_char, pend: *const c_char) -> *mut c_char {
    if p.is_null() || pend.is_null() || p >= pend {
        return pend as *mut c_char;
    }
    let len = (pend as usize) - (p as usize);
    // SAFETY: caller guarantees p..pend is a valid byte range.
    let slice = unsafe { core::slice::from_raw_parts(p as *const u8, len) };
    if let Some(pos) = slice.iter().position(|&b| b == b'\n') {
        (p as usize + pos + 1) as *mut c_char
    } else {
        pend as *mut c_char
    }
}

/// Alignment required for pack specifiers.
pub fn pack_alignof(type_: c_char, natint: c_int) -> c_int {
    use core::mem::align_of;
    let b = type_ as u8;
    match b {
        b'c' | b'C' => align_of::<c_char>() as c_int,
        b's' | b'S' => {
            if natint != 0 {
                align_of::<core::ffi::c_short>() as c_int
            } else {
                2
            }
        }
        b'i' | b'I' => align_of::<core::ffi::c_int>() as c_int,
        b'l' | b'L' => {
            if natint != 0 {
                align_of::<core::ffi::c_long>() as c_int
            } else {
                4
            }
        }
        b'q' | b'Q' => align_of::<i64>() as c_int,
        b'j' => align_of::<isize>() as c_int,
        b'J' => align_of::<usize>() as c_int,
        b'n' | b'v' => align_of::<u16>() as c_int,
        b'N' | b'V' => align_of::<u32>() as c_int,
        b'f' | b'F' | b'e' | b'g' => align_of::<f32>() as c_int,
        b'd' | b'D' | b'E' | b'G' => align_of::<f64>() as c_int,
        b'p' | b'P' => align_of::<*const c_char>() as c_int,
        _ => 0,
    }
}

/// Converts a hex character to integer value (0..15) or -1.
pub fn hex2num(c: c_char) -> c_int {
    let uc = c as u8;
    match uc {
        b'0'..=b'9' => (uc - b'0') as c_int,
        b'a'..=b'f' => (uc - b'a' + 10) as c_int,
        b'A'..=b'F' => (uc - b'A' + 10) as c_int,
        _ => -1,
    }
}

// C FFI Boundary Exports
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_pack_is_bigendian() -> c_int {
    is_bigendian()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_pack_skip_to_eol(p: *const c_char, pend: *const c_char) -> *mut c_char {
    // SAFETY: forwarded from C call.
    unsafe { skip_to_eol(p, pend) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_pack_pack_alignof(type_: c_char, natint: c_int) -> c_int {
    pack_alignof(type_, natint)
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_core_pack_hex2num(c: c_char) -> c_int {
    hex2num(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bigendian() {
        let be = is_bigendian();
        assert!(be == 0 || be == 1);
        #[cfg(target_endian = "little")]
        assert_eq!(be, 0);
        #[cfg(target_endian = "big")]
        assert_eq!(be, 1);
    }

    #[test]
    fn test_skip_to_eol() {
        let s = b"hello\nworld\n";
        let p = s.as_ptr() as *const c_char;
        let pend = (p as usize + s.len()) as *const c_char;

        let res1 = unsafe { skip_to_eol(p, pend) };
        let offset1 = (res1 as usize) - (p as usize);
        assert_eq!(offset1, 6);

        let p2 = res1;
        let res2 = unsafe { skip_to_eol(p2, pend) };
        let offset2 = (res2 as usize) - (p as usize);
        assert_eq!(offset2, 12);

        let res_end = unsafe { skip_to_eol(pend, pend) };
        assert_eq!(res_end, pend as *mut c_char);
    }

    #[test]
    fn test_pack_alignof() {
        assert_eq!(pack_alignof(b'c' as c_char, 0), 1);
        assert_eq!(pack_alignof(b's' as c_char, 0), 2);
        assert_eq!(pack_alignof(b'i' as c_char, 0), core::mem::align_of::<c_int>() as c_int);
        assert_eq!(pack_alignof(b'f' as c_char, 0), 4);
        assert_eq!(pack_alignof(b'd' as c_char, 0), 8);
        assert_eq!(pack_alignof(b'z' as c_char, 0), 0);
    }

    #[test]
    fn test_hex2num() {
        assert_eq!(hex2num(b'0' as c_char), 0);
        assert_eq!(hex2num(b'9' as c_char), 9);
        assert_eq!(hex2num(b'a' as c_char), 10);
        assert_eq!(hex2num(b'f' as c_char), 15);
        assert_eq!(hex2num(b'A' as c_char), 10);
        assert_eq!(hex2num(b'F' as c_char), 15);
        assert_eq!(hex2num(b'g' as c_char), -1);
        assert_eq!(hex2num(b'x' as c_char), -1);
    }
}
