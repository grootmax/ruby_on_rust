//! Rust port module for Garbage Collection operations.

use core::ffi::c_int;
use crate::usdt;

/// Fires the GC mark begin USDT probe.
#[inline]
pub fn trace_gc_mark_begin() {
    usdt::gc_mark_begin!();
}

/// Fires the GC mark end USDT probe.
#[inline]
pub fn trace_gc_mark_end() {
    usdt::gc_mark_end!();
}

/// Fires the GC enter USDT probe.
#[inline]
pub fn trace_gc_enter(event: c_int) {
    usdt::gc_enter!(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_usdt_macro() {
        trace_gc_mark_begin();
        trace_gc_enter(1);
        trace_gc_mark_end();
    }
}
