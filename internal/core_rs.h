#ifndef INTERNAL_CORE_RS_H                               /*-*-C-*-vi:se ft=c:*/
#define INTERNAL_CORE_RS_H
/**
 * @author     Ruby on Rust contributors
 * @copyright  This  file  is   a  part  of  the   programming  language  Ruby.
 *             Permission  is hereby  granted,  to  either  redistribute  and/or
 *             modify this file, provided that  the conditions mentioned in the
 *             file COPYING are met.  Consult the file for details.
 * @brief      Internal (non-public) functions implemented in core_rs.
 *
 * When a static C function is ported to core_rs/ and C code outside the
 * port still calls it, the Rust port exports it as
 * `rb_core_<file>_<name>` and the C file maps the old name to it with a
 * `#define` in its `#if USE_RUST_PORTS` branch.  Those symbols must not
 * become part of libruby's ABI (AGENTS.md §6.7), so they are declared here
 * with hidden visibility: on ELF the linker gives the definition the most
 * constraining visibility of all references, which drops it from the
 * dynamic symbol table.  On macOS, LIBRUBY_DLDFLAGS unexports `_rb_core_*`.
 *
 * Every rb_core_* function core_rs exports must be declared here.
 * `make core-rs-check-hidden` fails if one is visible in libruby.so.
 */
#include "ruby/internal/config.h"

#ifndef USE_RUST_PORTS
# define USE_RUST_PORTS 0
#endif

#if USE_RUST_PORTS

#if defined(__ELF__) && (defined(__GNUC__) || defined(__clang__))
# pragma GCC visibility push(hidden)
#endif

/* complex.c (core_rs/src/complex.rs) */
int rb_core_complex_issign(int c);
int rb_core_complex_isdecimal(int c);
int rb_core_complex_isimagunit(int c);
int rb_core_complex_read_sign(const char **s, char **b);
int rb_core_complex_read_rat_nos(const char **s, int strict, char **b);
int rb_core_complex_read_rat(const char **s, int strict, char **b);
void rb_core_complex_skip_ws(const char **s);

#if defined(__ELF__) && (defined(__GNUC__) || defined(__clang__))
# pragma GCC visibility pop
#endif

#endif /* USE_RUST_PORTS */
#endif /* INTERNAL_CORE_RS_H */
