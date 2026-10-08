# Ruby on Rust Compatibility Report

Automated test compliance report tracking Ruby test suite compatibility for Ruby on Rust.
Generated automatically by `ruby tool/generate_compatibility_report.rb`. Do not edit manually.

## Summary Statistics

| Test Suite | Total Tests | Passed | Failed | Skipped | Pass Rate |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **core_rs Unit Tests** | 21 | 21 | 0 | 0 | 100.0% |
| **Bootstrap Tests (btest)** | 2,106 | 2,106 | 0 | 0 | 100.0% |
| **Ruby Test Suite (test-all)** | 491 | 491 | 0 | 0 | 100.0% |
| **RubySpec (test-spec)** | 33,323 | 33,312 | 11 | 0 | 100.0% |
| **Total / Overall** | 35,941 | 35,930 | 11 | 0 | 100.0% |

## Detailed Test Suite Breakdown

### core_rs Unit Tests
- **Description:** Rust unit tests for `core_rs` ported modules. (`make core-rs-test`).
- **Total Tests:** 21
- **Passed:** 21
- **Failed:** 0
- **Skipped:** 0
- **Pass Rate:** 100.0%

### Bootstrap Tests (btest)
- **Description:** Core language syntax and VM functionality tests. (`make btest`).
- **Total Tests:** 2,106
- **Passed:** 2,106
- **Failed:** 0
- **Skipped:** 0
- **Pass Rate:** 100.0%

### Ruby Test Suite (test-all)
- **Description:** Standard library and core class unit tests. (`make test-all`).
- **Total Tests:** 491
- **Passed:** 491
- **Failed:** 0
- **Skipped:** 0
- **Pass Rate:** 100.0%

### RubySpec (test-spec)
- **Description:** Ruby language and library specifications. (`make test-spec`).
- **Total Tests:** 33,323
- **Passed:** 33,312
- **Failed:** 11
- **Skipped:** 0
- **Pass Rate:** 100.0%

