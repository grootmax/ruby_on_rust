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

#include <stdint.h>
#include "ruby/internal/special_consts.h"
#include "ruby/internal/static_assert.h"

/* core_rs/src/ffi/value.rs transcribes the special constants (selected by
 * USE_FLONUM, which core_rs/core_rs.mk takes from this preprocessor) and
 * assumes `long` is as wide as VALUE.  Fail the C build if the headers
 * disagree with the transcription. */
RBIMPL_STATIC_ASSERT(core_rs_long_is_value_sized, SIZEOF_LONG == SIZEOF_VALUE);
RBIMPL_STATIC_ASSERT(core_rs_qnil, RUBY_Qnil == (USE_FLONUM ? 0x04 : 0x02));
RBIMPL_STATIC_ASSERT(core_rs_qtrue, RUBY_Qtrue == (USE_FLONUM ? 0x14 : 0x06));
RBIMPL_STATIC_ASSERT(core_rs_qundef, RUBY_Qundef == (USE_FLONUM ? 0x24 : 0x0a));
RBIMPL_STATIC_ASSERT(core_rs_qfalse, RUBY_Qfalse == 0);
RBIMPL_STATIC_ASSERT(core_rs_immediate_mask, RUBY_IMMEDIATE_MASK == (USE_FLONUM ? 0x07 : 0x03));
RBIMPL_STATIC_ASSERT(core_rs_fixnum_flag, RUBY_FIXNUM_FLAG == 0x01);
RBIMPL_STATIC_ASSERT(core_rs_flonum_mask, RUBY_FLONUM_MASK == (USE_FLONUM ? 0x03 : 0x00));
RBIMPL_STATIC_ASSERT(core_rs_flonum_flag, RUBY_FLONUM_FLAG == 0x02);
RBIMPL_STATIC_ASSERT(core_rs_symbol_flag, RUBY_SYMBOL_FLAG == (USE_FLONUM ? 0x0c : 0x0e));
RBIMPL_STATIC_ASSERT(core_rs_special_shift, RUBY_SPECIAL_SHIFT == 8);

#include "siphash.h"

#if defined(__ELF__) && (defined(__GNUC__) || defined(__clang__))
# pragma GCC visibility push(hidden)
#endif

/* array.c (core_rs/src/array.rs) */
long rb_core_array_rotate_count(long cnt, long len);

/* bignum.c (core_rs/src/bignum.rs).  The digit parameters are BDIGIT,
 * which is `unsigned int` in every configuration core_rs supports;
 * bignum.c asserts that in its #if USE_RUST_PORTS branch.
 * rb_core_bignum_bigdivrem1 takes a struct big_div_struct *, which
 * bignum.c defines and checks against the #[repr(C)] mirror. */
int rb_core_bignum_bary_addc(unsigned int *zds, size_t zn, const unsigned int *xds, size_t xn, const unsigned int *yds, size_t yn, int carry);
int rb_core_bignum_bary_add(unsigned int *zds, size_t zn, const unsigned int *xds, size_t xn, const unsigned int *yds, size_t yn);
int rb_core_bignum_bary_add_one(unsigned int *ds, size_t n);
int rb_core_bignum_bary_mulsub_1xN(unsigned int *zds, size_t zn, unsigned int x, const unsigned int *yds, size_t yn);
void rb_core_bignum_bary_sq_fast(unsigned int *zds, size_t zn, const unsigned int *xds, size_t xn);
size_t rb_core_bignum_max_size(size_t a, size_t b);
void *rb_core_bignum_bigdivrem1(void *ptr);

/* complex.c (core_rs/src/complex.rs) */
int rb_core_complex_issign(int c);
int rb_core_complex_isdecimal(int c);
int rb_core_complex_isimagunit(int c);
int rb_core_complex_read_sign(const char **s, char **b);
int rb_core_complex_read_rat_nos(const char **s, int strict, char **b);
int rb_core_complex_read_rat(const char **s, int strict, char **b);
void rb_core_complex_skip_ws(const char **s);

/* io_buffer.c (core_rs/src/io_buffer.rs).  The C definitions qualify the
 * pointer parameters with `restrict`; callers guarantee the ranges do not
 * overlap. */
void rb_core_io_buffer_memory_or_inplace(unsigned char *base, size_t size, unsigned char *mask, size_t mask_size);
void rb_core_io_buffer_memory_xor_inplace(unsigned char *base, size_t size, unsigned char *mask, size_t mask_size);
void rb_core_io_buffer_memory_not_inplace(unsigned char *base, size_t size);
size_t rb_core_io_buffer_memory_bit_count(const unsigned char *base, size_t size);

/* missing/mt19937.c, compiled as part of random.c (core_rs/src/mt19937.rs).
 * struct MT is defined in missing/mt19937.c, which asserts that its layout
 * matches the #[repr(C)] mirror in mt19937.rs. */
struct MT;
void rb_core_mt19937_init_genrand(struct MT *mt, unsigned int s);
void rb_core_mt19937_init_by_array(struct MT *mt, const uint32_t init_key[], int key_length);
unsigned int rb_core_mt19937_genrand_int32(struct MT *mt);

/* numeric.c (core_rs/src/numeric.rs) */
double rb_core_numeric_round_half_up(double x, double s);
double rb_core_numeric_round_half_down(double x, double s);
double rb_core_numeric_round_half_even(double x, double s);
double rb_core_numeric_double_div_double(double x, double y);
VALUE rb_core_numeric_rb_dbl_hash(double d);
SIGNED_VALUE rb_core_numeric_int_round_half_even(SIGNED_VALUE x, SIGNED_VALUE y);
SIGNED_VALUE rb_core_numeric_int_round_half_up(SIGNED_VALUE x, SIGNED_VALUE y);
SIGNED_VALUE rb_core_numeric_int_round_half_down(SIGNED_VALUE x, SIGNED_VALUE y);
int rb_core_numeric_float_round_overflow(int ndigits, int binexp);
int rb_core_numeric_float_round_underflow(int ndigits, int binexp);
double ruby_float_step_size(double beg, double end, double unit, int excl);

/* siphash.c (core_rs/src/siphash.rs) */
uint64_t rb_core_siphash_u8to64_le(const uint8_t *p);
void rb_core_siphash_u64to8_le(uint8_t *p, uint64_t v);
uint64_t *rb_core_siphash_rotl64_to(uint64_t *v, unsigned int s);
uint64_t *rb_core_siphash_rotl64_swap(uint64_t *v);
uint64_t *rb_core_siphash_add64_to(uint64_t *v, const uint64_t s);
uint64_t *rb_core_siphash_xor64_to(uint64_t *v, const uint64_t s);
void rb_core_siphash_int_sip_dump(sip_state *state);
void rb_core_siphash_int_sip_init(sip_state *state, const uint8_t key[16]);

/* pack.c (core_rs/src/pack.rs) */
int rb_core_pack_is_bigendian(void);
char *rb_core_pack_skip_to_eol(const char *p, const char *pend);
size_t rb_core_pack_pack_alignof(char type, int natint);
int rb_core_pack_hex2num(char c);

/* rational.c (core_rs/src/rational.rs) */
long rb_core_rational_i_gcd(long x, long y);
int rb_core_rational_read_sign(const char **s, const char *const e);
int rb_core_rational_islettere(int c);
int rb_core_rational_isletterr(int c);
const char *rb_core_rational_skip_ws(const char *s, const char *e);

/* re.c (core_rs/src/re.rs) */
int rb_core_re_check_unicode_range(unsigned long code, char *err);
struct re_pattern_buffer; /* OnigRegexType, include/ruby/onigmo.h */
int rb_core_re_char_to_option(int c);
char *rb_core_re_option_to_str(char *str, int options);
int rb_core_re_pair_byte_cmp(const void *pair1, const void *pair2);
int rb_core_re_match_inspect_name_iter(const unsigned char *name, const unsigned char *name_end,
                                       int back_num, int *back_refs, struct re_pattern_buffer *regex, void *arg0);
int rb_core_re_read_escaped_byte(const char **pp, const char *end, char *err);

#if defined(__ELF__) && (defined(__GNUC__) || defined(__clang__))
# pragma GCC visibility pop
#endif

#endif /* USE_RUST_PORTS */
#endif /* INTERNAL_CORE_RS_H */
