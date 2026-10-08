#![no_main]

use libfuzzer_sys::fuzz_target;
use zjit::backend::parcopy::{sequentialize_register, RegisterCopy};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let spare = u32::from(data[0]);
    let mut copies = Vec::new();

    let mut idx = 1;
    while idx + 2 <= data.len() {
        let src = u32::from(data[idx]);
        let dst = u32::from(data[idx + 1]);
        copies.push(RegisterCopy {
            source: src,
            destination: dst,
        });
        idx += 2;
    }

    let _ = sequentialize_register(&copies, spare);
});
