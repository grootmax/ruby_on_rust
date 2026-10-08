# ZJIT Fuzzing and Miri Isolation

This document outlines how to invoke `cargo-fuzz` and `cargo miri test` for ZJIT engine components.

## Fuzzing subcrate (`zjit-fuzz`)

Subcrate location: `fuzz/`

Fuzz targets:
- `fuzz_arm64_encoder`: ARM64 instruction encoding fuzzing
- `fuzz_x86_64_encoder`: x86_64 instruction encoding fuzzing
- `fuzz_bitset`: BitSet data structure operations fuzzing
- `fuzz_parcopy`: Register sequentialize algorithm fuzzing

Command to invoke:
```bash
cargo fuzz run <target_name>
```

## Miri execution harness

Command to invoke:
```bash
cargo +nightly miri test --package zjit --lib miri_tests
```

This verifies pure Rust engine components for zero undefined behavior and clean memory management.
