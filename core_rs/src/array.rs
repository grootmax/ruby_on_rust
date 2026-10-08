//! Rust port module for Array operations.

use core::ffi::{c_int, c_long};
use crate::usdt;

/// Fires the array creation USDT probe.
#[inline]
pub fn trace_array_create(length: c_long, filename: &str, lineno: c_int) {
    usdt::array_create!(length, filename, lineno);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_usdt_macro() {
        trace_array_create(10, "test.rb", 42);
    }
}
