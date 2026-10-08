//! Rust port module for Object operations.

use core::ffi::c_int;
use crate::usdt;

/// Fires the object creation USDT probe.
#[inline]
pub fn trace_object_create(classname: &str, filename: &str, lineno: c_int) {
    usdt::object_create!(classname, filename, lineno);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_usdt_macro() {
        trace_object_create("Object", "test.rb", 1);
    }
}
