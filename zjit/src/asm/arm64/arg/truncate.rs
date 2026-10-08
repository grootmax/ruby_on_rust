// There are many instances in AArch64 instruction encoding where you represent
// an integer value with a particular bit width that isn't a power of 2. These
// functions represent truncating those integer values down to the appropriate
// number of bits.

/// Truncate a signed immediate to fit into a compile-time known width. It is
/// assumed before calling this function that the value fits into the correct
/// size. If it doesn't, then this function will panic.
///
/// When the value is positive, this should effectively be a no-op since we're
/// just dropping leading zeroes. When the value is negative we should only be
/// dropping leading ones.
pub fn truncate_imm<T: Into<i32>, const WIDTH: usize>(imm: T) -> u32 {
    let value: i32 = imm.into();
    if WIDTH >= 32 {
        value as u32
    } else {
        (value as u32) & ((1 << WIDTH) - 1)
    }
}

pub fn truncate_uimm<T: Into<u32>, const WIDTH: usize>(uimm: T) -> u32 {
    let value: u32 = uimm.into();
    if WIDTH >= 32 {
        value
    } else {
        value & ((1 << WIDTH) - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_imm_positive() {
        let inst = truncate_imm::<i32, 4>(5);
        let result: u32 = inst;
        assert_eq!(0b0101, result);
    }

    #[test]
    fn test_truncate_imm_negative() {
        let inst = truncate_imm::<i32, 4>(-5);
        let result: u32 = inst;
        assert_eq!(0b1011, result);
    }

    #[test]
    fn test_truncate_uimm() {
        let inst = truncate_uimm::<u32, 4>(5);
        let result: u32 = inst;
        assert_eq!(0b0101, result);
    }
}
