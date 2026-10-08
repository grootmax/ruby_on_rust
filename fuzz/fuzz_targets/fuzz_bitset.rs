#![no_main]

use libfuzzer_sys::fuzz_target;
use zjit::bitset::BitSet;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let capacity = (data[0] as usize) % 256;
    let mut bs1: BitSet<usize> = BitSet::with_capacity(capacity);
    let mut bs2: BitSet<usize> = BitSet::with_capacity(capacity);

    let mut idx = 1;
    while idx + 2 <= data.len() {
        let op = data[idx] % 6;
        let val = (data[idx + 1] as usize) * 8;

        match op {
            0 => { bs1.insert(val); },
            1 => { bs1.remove(val); },
            2 => { bs1.get(val); },
            3 => { bs2.insert(val); },
            4 => { bs1.union_with(&bs2); },
            5 => { bs1.intersect_with(&bs2); },
            _ => {},
        }

        idx += 2;
    }
});
