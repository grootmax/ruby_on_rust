# Code Audit Guidelines: Manual Clean-Up Guard Pattern & Pre-Call Object Flushing

## Context & Purpose

In CRuby, C functions such as `rb_raise()`, `rb_error_arity()`, or `rb_funcallv()` execute C `longjmp()` stack unwinding upon encountering an error or raising an exception. Because `longjmp()` unwinds stack frames at the C level, it bypasses Rust's standard stack unwinding mechanisms and skips Rust `Drop` destructors for any local variables living on active Rust stack frames.

To avoid the performance overhead of wrapping every C FFI invocation in `rb_protect()` callbacks while ensuring zero memory leaks or undefined behavior:

1. Engineers must audit all Rust functions calling raw raising C APIs.
2. Every local variable across a raw C FFI boundary must implement trivial drop (`!needs_drop::<T>()`) or be explicitly dropped/flushed before the raising C API call.
3. Developers must use `core_rs::ffi::UnwindGuard` to manage and record explicit pre-unwind resource flushing.

---

## Code Audit Rules for Developers & Reviewers

### Rule 1: Pre-Call Object Flushing
Before calling any raw C API function that can execute `longjmp` (e.g., `rb_raise`, `rb_funcallv`, `rb_yield`), developers must ensure that any owned Rust heap allocations or resources (such as `Vec`, `String`, `Box`, or custom structs with `impl Drop`) are dropped or flushed.

```rust
use core_rs::ffi::{rb_eArgError, rb_raise, UnwindGuard};

pub unsafe fn process_input(ptr: *const u8, len: usize) -> VALUE {
    // 1. Instantiate UnwindGuard for tracking resource cleanup
    let mut guard = UnwindGuard::with_cleanup(|| {
        // Free temporary buffers or release resources
    });

    if ptr.is_null() {
        // 2. Explicitly flush guard BEFORE invoking raising C API
        guard.flush();
        
        // 3. Invoke raw raising C function
        unsafe {
            rb_raise(rb_eArgError, c"pointer cannot be null".as_ptr());
        }
    }

    // 4. Dismiss guard on normal exit path
    guard.dismiss();
    Qnil
}
```

### Rule 2: Trivial Drop Assertion Across Call Boundaries
Every local variable active at the point of calling a raw C function must satisfy `!core::mem::needs_drop::<T>()`. Developers and code reviewers should use `assert_trivial_drop::<T>()` or `ensure_trivial_drop(&val)` to enforce this property at compile-time/runtime.

```rust
use core_rs::ffi::{assert_trivial_drop, ensure_trivial_drop};

let val: VALUE = ...;
// Verify that VALUE has a trivial drop implementation
assert_trivial_drop::<VALUE>();
ensure_trivial_drop(&val);
```

### Rule 3: Avoid Non-Trivial `impl Drop` Types Across Unprotected Boundaries
If a function requires complex local state with destructors that cannot be manually dropped before a raising call, developers MUST wrap the raising call using `core_rs::ffi::protect::protect()` (`rb_protect`) instead of raw FFI calls.

---

## Audit Checklist for PR Reviewers

When reviewing Rust code in `core_rs/`:
- [ ] Are all C API declarations imported from `core_rs::ffi`?
- [ ] If calling `rb_raise()` or other raising C functions directly:
  - Is an `UnwindGuard` instantiated and explicitly flushed via `guard.flush()` prior to the call?
  - Do all live local variables implement trivial drop (`!needs_drop::<T>()`)?
- [ ] If non-trivial `Drop` types exist across the exception boundary, is `core_rs::ffi::protect::protect()` used to guard the call?
