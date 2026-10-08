# Core Guidelines and Rules

## Rule 6: Exception Safety Across C FFI Boundaries

When porting code from C to Rust in Ruby on Rust:

1. **Destructor-bearing types across FFI calls**:
   A Rust stack frame that calls a C function that can raise a Ruby exception (via `longjmp`) must NOT hold any live `Drop` values (such as `Vec`, `String`, `Box`, or custom types implementing `Drop`) across the C FFI call boundary.
   If a Ruby exception is raised, `longjmp()` skips Rust stack unwinding, causing destructors for local variables to be bypassed and leading to memory or resource leaks.

2. **Exception Guard Macro (`#[ruby_exception_guard]`)**:
   All exported `extern "C"` functions or entrypoints that interact with CRuby and may raise exceptions must be annotated with `#[ruby_exception_guard]`. This attribute macro automatically wraps execution in `catch_unwind` and `rb_protect` / `rb_ensure` calls to trap Ruby exceptions and Rust panics safely.

3. **Declarative Cleanup (`ensure`)**:
   When cleanup logic must run regardless of exceptions, use `#[ruby_exception_guard(ensure = "cleanup_fn")]` to mirror CRuby `rb_ensure` semantics declaratively.

4. **Linter Enforcement**:
   The `ruby_guard_linter` static analysis tool enforces Rule 6 during build and check passes. Any unprotected C FFI call with an active `Drop` type across the call boundary will trigger a compilation lint error referencing this rule.
