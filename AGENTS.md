# Developer Guidelines & Porting Instructions

## Overview

This repository contains the Ruby codebase with an ongoing initiative to port core C components to Rust.
This document provides guidelines for developers and autonomous agents working on this project.

## Environment & Setup Requirements

To build and contribute to this repository, ensure the following dependencies are installed:
- **Python 3**: Required for maintaining porting ledgers and build scripts.
- **Rust Toolchain**: `rustc` 1.85.0+ and `cargo`.
- **C Build System**: `gcc` / `clang`, `make`, `autoconf`, `bison` (or `lrama`).

## Cargo Workspace & Rust Integration (`ruby.rs`)

The Rust integration in this repository uses a unified Cargo workspace structure:
- **`Cargo.toml`**: Configures the workspace members (`yjit`, `zjit`, `jit`) and root package `ruby`.
- **`ruby.rs`**: The root Rust entry point (`crate-type = ["staticlib"]`). It re-exports features based on active target flags (`yjit`, `zjit`).
- **Workspace Members**:
  - `yjit`: Yet Another JIT compiler written in Rust.
  - `zjit`: Experimental JIT compiler components in Rust.
  - `jit`: Common JIT interfaces and abstractions.
- **Cargo Features**:
  - `yjit`: Enables YJIT Rust compiler module (`yjit/`).
  - `zjit`: Enables ZJIT experimental tier-2 compiler module (`zjit/`).
  - `disasm`: Enables disassembly output features.
  - `runtime_checks`: Enables runtime invariant checks.

## Key Build Targets & Commands

- **Build Rust library**:
  ```bash
  cargo build
  cargo build --release
  ```
- **Run build and tests**:
  ```bash
  make test
  make check
  ```
- **Update Porting Ledger (`PORTING.md`)**:
  ```bash
  python3 tool/update_porting_ledger.py
  ```
- **Verify Porting Ledger in CI**:
  ```bash
  python3 tool/update_porting_ledger.py --verify
  ```

## Header Annotation Standard (`@porting-status`)

All C source (`.c`) and header (`.h`) files in the repository must contain an inline `@porting-status` tag in their header comments.

### Format
```c
/* @porting-status: <status> */
```
or with optional notes:
```c
/* @porting-status: <status> - <notes> */
```

### Valid Status Values
- **`unported`**: File is written in original C and porting to Rust has not started.
- **`in-progress`**: Active development is underway to port functions/structures to Rust.
- **`ported`**: Code has been migrated to Rust and integrated via Cargo/`ruby.rs`.
- **`blocked`**: Migration is blocked by architectural dependencies or missing infrastructure.
- **`not-planned`**: File will remain in C and is not scheduled for Rust migration.

### Workflow Rules for Contributors & Agents
1. When creating or modifying a C source or header file, ensure it contains a valid `@porting-status` header tag.
2. When migrating code from C to Rust, update the header annotation in the C file to `in-progress` or `ported`.
3. After updating status tags, run `python3 tool/update_porting_ledger.py` to regenerate `PORTING.md`.
4. Continuous Integration (CI) enforces ledger accuracy using `python3 tool/update_porting_ledger.py --verify`.
