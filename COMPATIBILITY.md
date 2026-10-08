# Ruby on Rust Behavioral Compatibility Matrix

Automated differential compatibility report verifying Ruby on Rust against reference CRuby.
Generated automatically by `ruby tool/ruby_on_rust/generate_compatibility_report.rb`. Do not edit manually.

## Executive Compatibility Summary

| Subsystem / Check | Status | Passed Cases | Total Cases | Pass Rate |
| :--- | :---: | :---: | :---: | :---: |
| **Differential Snippet Testing** | PASSED | 8 | 8 | 100.0% |
| **API Structure Lock** | PASSED | Locked | Locked | 100.0% |
| **ABI Interface Lock** | PASSED | Locked | Locked | 100.0% |
| **Docs Verification Lock** | PASSED | Locked | Locked | 100.0% |
| **Nightly Edge Case Fuzzing** | PASSED | 59 | 59 | 100.0% |

## Core Class Fuzzing Coverage Matrix

| Core Class | Total Cases | Passed Cases | Failed Cases | Compatibility Status |
| :--- | :---: | :---: | :---: | :---: |
| **String** | 8 | 8 | 0 | 100% Compatible |
| **Array** | 8 | 8 | 0 | 100% Compatible |
| **Hash** | 6 | 6 | 0 | 100% Compatible |
| **Integer** | 9 | 9 | 0 | 100% Compatible |
| **Float** | 9 | 9 | 0 | 100% Compatible |
| **Range** | 6 | 6 | 0 | 100% Compatible |
| **Struct** | 3 | 3 | 0 | 100% Compatible |
| **Time** | 3 | 3 | 0 | 100% Compatible |
| **Comparable** | 1 | 1 | 0 | 100% Compatible |
| **Enumerable** | 6 | 6 | 0 | 100% Compatible |

## Lock & Integrity Invariants

- **API Lock**: All core class instance methods, singleton methods, and constants match baseline CRuby contracts.
- **ABI Lock**: Dynamic symbol exports, `VALUE` alignment, Fixnum boundaries, and header contracts verified.
- **Docs Lock**: Key documentation files (`COMPATIBILITY.md`, `PORTING.md`, `README.md`, `AGENTS.md`) tracked and synchronized.

---
*Report generated automatically by the Ruby on Rust Continuous Differential Harness.*
