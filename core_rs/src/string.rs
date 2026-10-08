//! Rust port module for String operations.

use core::ffi::{c_int, c_long};
use crate::usdt;

/// Fires the string creation USDT probe.
#[inline]
pub fn trace_string_create(length: c_long, filename: &str, lineno: c_int) {
    usdt::string_create!(length, filename, lineno);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_usdt_macro() {
        trace_string_create(15, "test.rb", 100);
    }
}
