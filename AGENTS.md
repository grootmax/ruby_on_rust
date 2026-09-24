# Core Runtime Migration Reference: C-to-Rust Exported Functions

This document serves as the canonical reference pattern for migrating core C functions and global state in CRuby to Rust.

## Canonical Pattern: Exported C Functions with Atomic State

When migrating C scalar accessors and global state to Rust:

1. **Rust Export Signature (`#[no_mangle] pub extern "C"`)**:
   - Use `#[no_mangle]` and `pub extern "C"` on the Rust function definition.
   - Align function names with symbol retention filters in `defs/jit.mk` (functions starting with `rb_`).
   - Use FFI-compatible types like `std::ffi::c_int`.

2. **Thread-Safe Global State (`std::sync::atomic`)**:
   - Use Rust's atomic primitives (`AtomicI32`, `AtomicBool`, etc.) for scalar global state to guarantee thread safety without locks.
   - Use atomic memory orderings like `Ordering::SeqCst` for consistent global visibility across OS threads.

3. **Reference Example**:
   - `rb_ruby_default_parser` & `rb_ruby_default_parser_set` in `ruby.rs`:
     ```rust
     use std::ffi::c_int;
     use std::sync::atomic::{AtomicI32, Ordering};

     pub const RB_DEFAULT_PARSER_PARSE_Y: c_int = 0;
     pub const RB_DEFAULT_PARSER_PRISM: c_int = 1;

     static DEFAULT_PARSER: AtomicI32 = AtomicI32::new(RB_DEFAULT_PARSER_PRISM);

     #[no_mangle]
     pub extern "C" fn rb_ruby_default_parser() -> c_int {
         DEFAULT_PARSER.load(Ordering::SeqCst)
     }

     #[no_mangle]
     pub extern "C" fn rb_ruby_default_parser_set(parser: c_int) {
         DEFAULT_PARSER.store(parser, Ordering::SeqCst);
     }
     ```

4. **C Header Retention**:
   - Keep C declarations in headers (e.g. `internal/parse.h`) unchanged so C callers continue seamlessly without modified include files.
   - Remove original C implementations from source files (e.g. `version.c`).
