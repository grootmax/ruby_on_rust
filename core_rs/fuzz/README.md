# Differential fuzzing of core_rs ports

Every function ported to `core_rs/` must be proven to behave exactly like the
C it replaces (AGENTS.md §6.5). The proof is a differential fuzz unit in this
directory. It calls the original C and the Rust port with the same random
inputs and compares every observable result. The required result is
**0 mismatches over at least 1,000,000 cases**, plus a passing **mutation
check**.

## Layout

| Path | What |
|---|---|
| `fuzz.h` | PRNG, input generators, `FUZZ_EQ`, mutation mode, `main()` |
| `run.rb` | Extracts the C reference, builds and runs units (standard Ruby only) |
| `template.c` | Starting point for a new unit |
| `units/*.c` | One harness per port unit; all of them run in CI |

## How a unit is built

`run.rb` needs two build trees of the same source, both configured with
`--enable-shared`: one with the Rust ports and one `--without-rust-ports`.

1. **C reference.** Every name on the harness's `FUZZ-EXTRACT: <file.c> name...`
   line is copied **verbatim** from the CRuby source into
   `<unit>.ref.c` and renamed `ref_<name>`. Functions, static helpers and
   static tables are all extracted the same way, in source order, with
   prototypes so callers may precede callees. The copy is made fresh on
   every run, so it can never drift from the real C. When a definition has
   `#if` variants, the first one in the file is taken; write `name:FIRST-LAST`
   to copy an exact line range instead.
2. **Rust port.** `libcore_rs.a` from the with-ports build is copied and its
   defined `rb_*` / `ruby_*` symbols are renamed `rs_*`. The harness declares
   the `rs_` prototypes it calls.
3. **Link.** The harness, the reference and the renamed archive are linked
   against the `--without-rust-ports` `libruby.so`. Anything the C
   reference calls (tables, `rb_bug`, encodings) therefore resolves to the
   original C, never to a port.
4. **Run.** First the differential run (`-n`, default 1,000,000 cases; exit 0
   only with 0 mismatches), then the mutation check (default 100,000 cases).
   In mutation mode `fuzz_rs_bytes()` / `fuzz_rs_long()` hand the Rust side a
   perturbed input in every case, and the check passes only if the harness
   reports mismatches. A harness that compares nothing, or compares the
   wrong thing, fails here.

```sh
ruby core_rs/fuzz/run.rb --with-ports ../build --without-ports ../build-noports \
     core_rs/fuzz/units/*.c
# fuzz util_scan: differential cases=1000000 mismatches=0 seed=1
# fuzz util_scan: MUTATION cases=100000 mismatches=... seed=1
```

On a mismatch the first 10 cases are printed with their inputs and a case
seed. Rebuild nothing and replay one case with
`<build>/target/core_rs/fuzz/fuzz_<unit> --case <case-seed>`.

Supported: Linux with GNU or LLVM `nm`/`objcopy`. macOS is not supported
(its symbol prefix and `objcopy` differ); the Port gate runs units on Linux.

## Writing a unit

Copy `template.c` to `units/<c-file>_<topic>.c` and:

1. List on `FUZZ-EXTRACT` every C definition the port replaces, plus the
   static helpers and tables they use.
2. Include the headers the extracted C needs (those of its `.c` file).
3. Write `fuzz_one()`:
   - Generate inputs that reach **every branch**: boundary lengths (0, 1,
     the maximum), every flag and base value including invalid ones the C
     accepts, overflow, signedness edge cases (bytes >= 0x80). Bias the
     alphabet with `fuzz_fill(buf, len, "interesting bytes")`.
   - Record inputs with `fuzz_input()` / `fuzz_input_num()`.
   - Give the Rust side its inputs through `fuzz_rs_bytes()` (or
     `fuzz_rs_bytes_slot(1, ...)` for a second buffer) and `fuzz_rs_long()`.
   - Compare **every** observable effect with `FUZZ_EQ`: return value,
     out-parameters (initialise them to different sentinels on each side),
     `errno` (set it to the same sentinel before each call), every byte
     written, every callback and its arguments. Compare pointers as offsets
     from their own buffer.
   - Only call functions with inputs that satisfy the C function's
     documented preconditions (e.g. `ruby_scan_digits` needs base 2..36).
     Passing inputs that are undefined behaviour in C proves nothing.
   - If the C reads past the end of a buffer (some search loops do), give
     both sides the same zero-filled slack after the input, and say so in a
     comment.
4. If the C needs a running VM (encodings, symbols), define
   `FUZZ_HAVE_SETUP` and call `ruby_init()` in `fuzz_setup()`
   (see `units/re_memsearch.c`).

## Rules

- A unit is required for every port PR. Its file name is listed in the PR's
  Evidence section with the last two output lines of `run.rb`.
- Never weaken a unit to make it pass: no lowering the case count, no
  narrowing inputs to dodge a mismatch, no deleting comparisons. A mismatch
  is a fidelity bug in the port (or a precondition the harness violates;
  say which, with evidence).
- Keep units deterministic: all randomness comes from `fuzz.h`.
