#![no_main]

use libfuzzer_sys::fuzz_target;
use core_rs::strftime::{case_conv, isleap, iso8601wknum, max, min, strftime_size_limit, weeknumber, Tm};
use std::ffi::c_char;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    // Fuzz min / max
    let a = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let b = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let _ = min(a, b);
    let _ = max(a, b);

    // Fuzz isleap
    let year = a as i64;
    let _ = isleap(year);

    // Fuzz strftime_size_limit
    let _ = strftime_size_limit(data.len());

    // Fuzz case_conv
    let mut copy = data.to_vec();
    let ptr = copy.as_mut_ptr() as *mut c_char;
    let flags = data[0] as i32;
    let _ = unsafe { case_conv(ptr, copy.len() as isize, flags) };

    // Fuzz iso8601wknum and weeknumber
    let tm = Tm {
        tm_sec: (data[0] % 60) as i32,
        tm_min: (data[1] % 60) as i32,
        tm_hour: (data[2] % 24) as i32,
        tm_mday: (data[3] % 31 + 1) as i32,
        tm_mon: (data[4] % 12) as i32,
        tm_year: (a % 400 + 100) as i32,
        tm_wday: (data[5] % 7) as i32,
        tm_yday: (data[6] as i32 % 366),
        tm_isdst: 0,
    };
    let _ = iso8601wknum(&tm);
    let _ = weeknumber(&tm, 0);
    let _ = weeknumber(&tm, 1);
});
