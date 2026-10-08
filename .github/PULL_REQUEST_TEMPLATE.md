## Objective
<!-- One outcome. Link the task or review comment this PR answers. -->

## Dependencies
<!-- "Blocked on #N" or "None". Do not push while a dependency is unmerged. -->

## Scope
<!-- The files this PR is allowed to touch (from the task). Explain any file outside that list. -->

## Rules checklist (AGENTS.md §6–§8)
- [ ] No changes under `yjit/`, `zjit/`, `jit/`
- [ ] No CI step or test removed, skipped, `if: false`'d, loosened or re-versioned
- [ ] Each workflow edit listed below with its reason (or "none")
- [ ] Ported C kept behind `#if !USE_RUST_PORTS` (ports only)
- [ ] Behaviour identical to C, with build-time macros honoured (ports only)
- [ ] No new exported symbols (`nm -D --defined-only libruby.so` matches master)
- [ ] Ledger changed only via `tool/porting_status.yml` and regenerated
- [ ] No `#[cfg(test)]` stubs of the functions under test

## Ported Functions/Files
<!-- List C functions and source files ported to Rust or touched in this PR (Conversion Rule 16). -->

## Exported Symbols Kept
<!-- List exported C symbols (rb_* or ruby_*) retained or added for ABI/FFI compatibility. Write "None" if none. -->

## Test Run Results
<!-- Provide test output and summary from cargo test, make test, make test-all, etc. -->

## Benchmark Numbers
<!-- Provide performance benchmark numbers or comparison metrics against C baseline. Write "N/A" if non-porting. -->

## Unsafe Blocks & Safety Rationales
<!-- List all unsafe Rust blocks in ported code and state the explicit safety rationale for each. Write "None" if no unsafe code. -->

## Evidence (AGENTS.md §9)
<!-- Exact commands + last lines of output. One block per acceptance criterion.
     Ports: results for both ./configure and ./configure --without-rust-ports. -->

## CI status at time of writing
<!-- State it honestly: green / red (which checks) / pending. Never write "verified" while anything is red or pending. -->
