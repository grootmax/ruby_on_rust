# Ruby on Rust Custom Port Tests (`test/ruby_on_rust/`)

This directory is designated for custom, port-specific tests in the Ruby on Rust project.

## Overview & Purpose

- **Port-Specific Test Directory**: Under project Rules 2 and 3, custom tests for Rust ported components or features specific to the Ruby on Rust port should be placed in `test/ruby_on_rust/`.
- **Preserve Upstream Test Suites**: Upstream tests in `test/`, `spec/ruby/`, and `bootstraptest/` must remain unmodified unless explicitly required. Adding custom tests here ensures upstream compatibility and prevents test drift.

## Conventions

- **File Naming**: Test files must follow the standard `test_*.rb` naming convention (e.g. `test_sanity.rb`).
- **Auto-Discovery**: `Test::Unit::AutoRunner` automatically discovers and executes tests in this directory when running `make test-all` or `ruby test/runner.rb`.
