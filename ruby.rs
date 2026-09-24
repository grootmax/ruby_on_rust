use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

#[cfg(feature = "yjit")]
pub use yjit::*;
#[cfg(feature = "zjit")]
pub use zjit::*;

pub const RB_DEFAULT_PARSER_PARSE_Y: c_int = 0;
pub const RB_DEFAULT_PARSER_PRISM: c_int = 1;

static DEFAULT_PARSER: AtomicI32 = AtomicI32::new(RB_DEFAULT_PARSER_PRISM);

#[unsafe(no_mangle)]
pub extern "C" fn rb_ruby_default_parser() -> c_int {
    DEFAULT_PARSER.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
pub extern "C" fn rb_ruby_default_parser_set(parser: c_int) {
    DEFAULT_PARSER.store(parser, Ordering::SeqCst);
}
