# Upstream base

Ruby on Rust is a fork of [ruby/ruby](https://github.com/ruby/ruby). The owner
made a "Sync fork" on 2026-10-07 and a last one on 2026-10-08 (the cutoff
day), after which the fork stopped syncing from upstream.

| | |
|---|---|
| Last upstream sync | `70f1b59a0ce29e2e59b09780a2f94cfb782d61ff` ("Merge branch 'ruby:master' into master", 2026-10-08) |
| Upstream base commit | `e4672be48dca6184fe63ef858f3f335eb347a72d` (ruby/ruby, "ZJIT: Inline write barrier check in LIR (#18641)", 2026-10-08) |
| Previous sync | `4d835210ecbf9de894f39041ae2d7d59b80bb06f` (2026-10-07) |
| Cutoff | 2026-10-08. No upstream merges after this point |
| Ruby version line | 4.1.0dev |

## Policy after the cutoff

- **No merges from ruby/ruby `master`.** Ported files diverge by design, and
  upstream merges would conflict with the Rust ports.
- **Security fixes are cherry-picked.** A weekly scheduled task reviews
  https://www.ruby-lang.org/en/security/ and upstream security commits made
  after the base commit above. Each applicable fix is opened as its own PR
  labelled `security-backport`. If the affected function has a Rust port,
  the fix is applied to both the C implementation (behind
  `#if !USE_RUST_PORTS`) and the Rust port, with a regression test.
- **Other upstream bug fixes** may be cherry-picked case by case, as normal
  PRs that reference the upstream commit.

To compare with upstream: `git diff e4672be48dca6184fe63ef858f3f335eb347a72d`
shows everything Ruby on Rust has changed since the base.
