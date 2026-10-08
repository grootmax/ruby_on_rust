#![no_main]

use libfuzzer_sys::fuzz_target;
use core_rs::sprintf::{fmt_setup, ruby_ultoa, sign_bits};
use std::ffi::c_char;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    // Fuzz sign_bits
    let base = data[0] as i32;
    let char_val = [data[1], 0];
    let p = char_val.as_ptr() as *const c_char;
    let _ = unsafe { sign_bits(base, p) };

    // Fuzz ruby_ultoa
    let val = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as u64;
    let flags = data[0] as i32;
    let mut buf = [0u8; 128];
    let endp = unsafe { buf.as_mut_ptr().add(128) as *mut c_char };
    let _ = unsafe { ruby_ultoa(val, endp, 10, flags) };
    let _ = unsafe { ruby_ultoa(val, endp, 16, flags) };
    let _ = unsafe { ruby_ultoa(val, endp, 8, flags) };

    // Fuzz fmt_setup
    let spec = data[0] as i32;
    let width = data[1] as i32;
    let prec = data[2] as i32;
    let mut fmt_buf = [0u8; 128];
    let fmt_ptr = fmt_buf.as_mut_ptr() as *mut c_char;
    let _ = unsafe { fmt_setup(fmt_ptr, 128, spec, flags, width, prec) };
});
