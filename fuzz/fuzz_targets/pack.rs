#![no_main]

use libfuzzer_sys::fuzz_target;
use core_rs::pack::{hex2num, is_bigendian, pack_alignof, skip_to_eol};
use std::ffi::c_char;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Fuzz hex2num
    let c = data[0] as c_char;
    let _ = hex2num(c);

    // Fuzz pack_alignof
    let natint = (data[0] & 1) as i32;
    let _ = pack_alignof(c, natint);

    // Fuzz is_bigendian
    let _ = is_bigendian();

    // Fuzz skip_to_eol
    if data.len() > 1 {
        let p = data.as_ptr() as *const c_char;
        let pend = unsafe { p.add(data.len()) };
        let res = unsafe { skip_to_eol(p, pend) };
        assert!(res >= (p as *mut c_char) && res <= (pend as *mut c_char));
    }
});
