# ZJIT Testing: Cargo Fuzz & Miri Isolation

This document describes how to execute `cargo-fuzz` fuzzing targets and `cargo miri test` memory-safety verification for pure Rust engine logic in ZJIT.

---

## 1. Fuzzing ZJIT Components (`cargo-fuzz`)

The workspace includes a dedicated fuzzing subcrate located at `/app/fuzz` (`zjit-fuzz`). Fuzzing dependencies (such as `libfuzzer-sys`) are contained entirely within this subcrate and do not enter release or standard library builds.

### Requirements
- Rust Nightly toolchain
- `cargo-fuzz` utility (`cargo install cargo-fuzz`)

### Running Fuzz Targets
Navigate to the repository root or `/app/fuzz` and run:

```bash
# Fuzz ARM64 instruction encoders
cargo fuzz run fuzz_arm64_encoder

# Fuzz x86_64 instruction encoders
cargo fuzz run fuzz_x86_64_encoder

# Fuzz BitSet data structure
cargo fuzz run fuzz_bitset

# Fuzz parallel copy register sequentialization
cargo fuzz run fuzz_parcopy
```

Fuzz targets feed pseudo-random inputs and boundary conditions to verify that pure Rust logic handles invalid bit patterns or invalid arguments gracefully without panicking or triggering undefined behavior.

---

## 2. Miri Verification (`cargo miri test`)

ZJIT isolates C FFI declarations and shims behind `#[cfg(any(miri, fuzzing))]` conditional compilation. This allows Miri to verify pure Rust logic (encoders, data structures, register allocation) without failing on unresolved C symbols.

### Running Miri Tests
To run Miri against the dedicated pure Rust test suite in ZJIT:

```bash
cargo +nightly miri test --package zjit --lib miri_tests
```

### Covered Components
- **ARM64 Encoders**: Invalid bit patterns, out-of-range immediates, bitmask conversion edge cases.
- **x86_64 Encoders**: Operand size matching, displacement bounds, instruction encoding.
- **BitSet**: Out-of-bounds queries, capacity bounds, set union and intersection operations.
- **Parallel Copy Sequentializer**: Register copy ordering, cycle detection, spare register usage.
