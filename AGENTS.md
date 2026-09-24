# Developer & AI Agent Guidelines (AGENTS.md)

Welcome to the **Ruby on Rust** repository. This document provides instructions, rules, and workflows for both human contributors and AI agents working on porting CRuby C source files to Rust.

---

## 1. Architecture & Cargo Setup

This repository contains the CRuby codebase incrementally migrating core C components to Rust.

- **Cargo Workspace**: The root `Cargo.toml` defines a workspace containing crates such as `yjit`, `zjit`, and `jit`.
- **Static Library Integration**: Cargo compiles a single static library target defined in `ruby.rs` (`crate-type = ["staticlib"]`) to integrate Rust modules into the C runtime.
- **Cargo Features**:
  - `yjit`: Enables YJIT Rust compiler module (`yjit/`).
  - `zjit`: Enables ZJIT experimental tier-2 compiler module (`zjit/`).
  - `disasm`: Enables disassembly output features.
  - `runtime_checks`: Enables runtime invariant checks.

---

## 2. Build Commands

### Rust Workspace Build
To compile the Rust workspace crates:
```bash
# Build debug artifacts across all workspace crates
cargo build --workspace

# Build optimized release artifacts
cargo build --release --workspace
```

### Full Ruby Build Workflow
To build the complete Ruby binary with Rust integration:
```bash
# 1. Generate configure script (if needed)
./autogen.sh

# 2. Configure build environment (enabling desired Rust JIT engines)
./configure --enable-yjit

# 3. Build Ruby executable and static library
make -j$(nproc)
```

---

## 3. Test Workflows

To verify changes across Rust and Ruby components, run the following test suites:

### Rust Unit & Integration Tests
```bash
cargo test --workspace
```

### Ruby Test Suites
```bash
# Fast bootstrap test suite
make test

# Full Ruby unit test suite
make test-all

# Ruby specification suite (ruby/spec)
make test-spec
```

### Porting Ledger Drift Verification
Before pushing changes, ensure the migration ledger `PORTING.md` is synchronized with `tool/porting_status.yml` and top-level C source files:
```bash
ruby tool/generate_porting_ledger.rb --check
```

---

## 4. C-to-Rust Porting Workflow

When porting or updating a C source file:

1. **Keep Baseline C Files Intact**: Core C source files in the repository root serve as the baseline reference and must remain unchanged unless required for C runtime linkage.
2. **Update Status Configuration**: Modify `tool/porting_status.yml` to reflect changes in file porting status:
   - Valid status values: `Not Started`, `In Progress`, `Ported`, `Blocked`, `N/A`.
   - Update target Rust crate/module name (e.g., `yjit`, `zjit`, `crate::array`).
   - Add notes or description as appropriate.
3. **Regenerate Ledger**: Run the ledger generation tool to update `PORTING.md`:
   ```bash
   ruby tool/generate_porting_ledger.rb
   ```
4. **Verify Ledger Freshness**: Run the check command to ensure `PORTING.md` has no drift:
   ```bash
   ruby tool/generate_porting_ledger.rb --check
   ```

---

## 5. Rules & Guardrails for AI Agents

- **Follow Workspace Rules**: Always observe instructions in `AGENTS.md` and any subdirectory `AGENTS.md` files.
- **Do Not Mask Test Failures**: Do not modify test assertions or delete test cases to pass CI unless explicitly intended.
- **Standard Ruby Tooling**: Scripts placed in `tool/` must rely strictly on standard Ruby libraries (`yaml`, `optparse`, etc.) without external gem dependencies.
- **Path Resolution**: Tools and scripts should resolve repository paths relative to `__dir__` to work reliably across environments.
