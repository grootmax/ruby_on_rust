//! Rust port module for Hash operations.

use core::ffi::{c_int, c_long};
use crate::usdt;

/// Fires the hash creation USDT probe.
#[inline]
pub fn trace_hash_create(length: c_long, filename: &str, lineno: c_int) {
    usdt::hash_create!(length, filename, lineno);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_usdt_macro() {
        trace_hash_create(5, "test.rb", 200);
    }
}
