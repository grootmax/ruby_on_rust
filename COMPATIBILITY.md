# Ruby on Rust Compatibility & Parity Report

Automated compatibility, parity, and quality gate report tracking baseline CRuby interface lock status and C-to-Rust migration progress.

Generated automatically by `ruby tool/ruby_on_rust/update_compatibility.rb`. Do not edit manually.

## Quality Gate Status Summary

| Quality Gate | Status | Allowlisted Exceptions | Description |
| :--- | :--- | :--- | :--- |
| **Core API Lock** | **PASSED** | 0 | Core class/module reflection, method arity, and constant parity |
| **C ABI & Struct Offset Lock** | **PASSED** | 0 | Header macros in `include/ruby/*.h`, exported C symbols, and struct field offsets |
| **Core Documentation Lock** | **PASSED** | 0 | Core RDoc/RI documentation coverage and parity |
| **Porting Ledger Drift** | **PASSED** | - | Synchronization between top-level C files, `PORTING.md`, and `porting_status.yml` |

## C-to-Rust Migration Summary

| Metric | File Count | Lines of Code (LOC) | % of Total LOC |
| :--- | :--- | :--- | :--- |
| **Total C Source Files** | 113 | 317,465 | 100.0% |
| **Not Started** | 103 | 308,463 | 97.2% |
| **In Progress** | 3 | 8,656 | 2.7% |
| **Ported** | 0 | 0 | 0.0% |
| **Blocked** | 0 | 0 | 0.0% |
| **N/A** | 7 | 346 | 0.1% |

## Subsystem Porting Breakdown

| Subsystem | Total Files | Total LOC | Ported / In Progress | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Concurrency & Threads** | 12 | 25,751 | 0 / 12 | Baseline C |
| **Core Data Structures** | 20 | 94,148 | 1 / 20 | 1 active |
| **Encoding & Regex** | 9 | 32,831 | 1 / 9 | 1 active |
| **IO & Filesystem** | 7 | 47,054 | 0 / 7 | Baseline C |
| **JIT Compiler** | 5 | 22,150 | 0 / 5 | Baseline C |
| **Memory & Garbage Collection** | 4 | 10,459 | 0 / 4 | Baseline C |
| **Parser & AST** | 8 | 15,936 | 0 / 8 | Baseline C |
| **Platform & Miscellaneous** | 8 | 3,662 | 0 / 8 | Baseline C |
| **Utilities & Support** | 26 | 30,781 | 1 / 26 | 1 active |
| **Virtual Machine & Execution** | 14 | 34,693 | 0 / 14 | Baseline C |

## Quality Gate Allowlists

Plain-text allowlists filter approved temporary deviations:
- `tool/ruby_on_rust/allowlists/api_allowlist.txt`: 0 exception pattern(s)
- `tool/ruby_on_rust/allowlists/abi_allowlist.txt`: 0 exception pattern(s)
- `tool/ruby_on_rust/allowlists/docs_allowlist.txt`: 0 exception pattern(s)

