/*
 * Differential fuzz: re.c char_to_option, option_to_str,
 * rb_char_to_option_kcode, pair_byte_cmp, match_inspect_name_iter and
 * read_escaped_byte (port unit re-A-01, core_rs/src/re.rs) against the
 * original C.
 *
 * FUZZ-EXTRACT: re.c char_to_option option_to_str rb_char_to_option_kcode pair_byte_cmp match_inspect_name_iter read_escaped_byte
 *
 * Each case drives one of the six functions:
 *
 * - char_to_option / rb_char_to_option_kcode: the option and kcode letters
 *   (and their neighbours, upper case, NUL), any byte as signed and
 *   unsigned char, and any int.  Out-parameters start at different
 *   sentinels on each side; one case in eight passes the same pointer for
 *   both, so the C store order is compared too.
 * - option_to_str: small option sets (every combination of the m/i/x bits),
 *   their complements (as rb_reg_str_with_term passes ~options) and any
 *   int.  The buffer has a byte of slack on each side, all prefilled with
 *   the same random bytes; every byte and the returned pointer (as an
 *   offset) are compared.
 * - pair_byte_cmp: byte positions in the C domain 0..LONG_MAX (offsets into
 *   a string, where the C subtraction cannot overflow): equal, adjacent,
 *   small, large, extreme; char_pos is random and must be ignored.
 * - match_inspect_name_iter: back_num from -2 to 16 (<= 0 runs no loop),
 *   group numbers in range with repeats, names anywhere in one buffer,
 *   empty names included.  The whole array, prefilled with the same random
 *   bytes on both sides, is compared field by field.
 * - read_escaped_byte: sources built from escape tokens (\, M-, C-, c, x,
 *   octal and hex digits, the letter escapes, '-', bytes >= 0x80) so every
 *   branch, success and error alike, is reached, plus random bytes.  The
 *   return value, the advance of *pp (as an offset) and every byte of the
 *   error buffer (prefilled with the same random bytes) are compared.  The
 *   C reads only [*pp, end); the byte after end is a random sentinel on the
 *   C side, so an overread would show up as a mismatch.
 */
#define FUZZ_UNIT "re_01"
#include "ruby/ruby.h"
#include "ruby/onigmo.h"
#include "ruby/util.h"
#include "encindex.h"
#include "internal/re.h"
#include <limits.h>
#include <stddef.h>
#include <string.h>

/* Definitions from re.c that the extracted functions use. */
typedef char onig_errmsg_buffer[ONIG_MAX_ERROR_MESSAGE_LEN];
#define errcpy(err, msg) strlcpy((err), (msg), ONIG_MAX_ERROR_MESSAGE_LEN)
enum { OPTBUF_SIZE = 4 };
typedef struct {
    rb_len_t byte_pos;
    rb_len_t char_pos;
} pair_t;
struct backref_name_tag {
    const UChar *name;
    rb_len_t len;
};

#include FUZZ_REF
#include "fuzz.h"

int rs_rb_core_re_char_to_option(int c);
char *rs_rb_core_re_option_to_str(char *str, int options);
int rs_rb_char_to_option_kcode(int c, int *option, int *kcode);
int rs_rb_core_re_pair_byte_cmp(const void *pair1, const void *pair2);
int rs_rb_core_re_match_inspect_name_iter(const OnigUChar *name, const OnigUChar *name_end,
                                          int back_num, int *back_refs, OnigRegex regex, void *arg0);
int rs_rb_core_re_read_escaped_byte(const char **pp, const char *end, char *err);

/* A character argument: letters that matter, neighbours, any byte as
 * signed or unsigned char, any int. */
static int
draw_char(void)
{
    static const char letters[] = "ixmnesuIXMNESUhjlotvw\0";
    switch (fuzz_below(4)) {
      case 0: return (int)(unsigned char)letters[fuzz_below(sizeof(letters))];
      case 1: return (int)(signed char)fuzz_below(256);
      case 2: return (int)fuzz_below(256);
      default: return (int)(uint32_t)fuzz_next();
    }
}

static void
fuzz_char_to_option(void)
{
    int c = draw_char();
    fuzz_input_num("fn char_to_option", 0);
    fuzz_input_num("c", c);
    FUZZ_EQ("ret", ref_char_to_option(c), rs_rb_core_re_char_to_option((int)fuzz_rs_long(c)));
}

static void
fuzz_option_kcode(void)
{
    int c = draw_char();
    int c_opt = 0x1111, c_kc = 0x2222, rs_opt = 0x3333, rs_kc = 0x4444;
    int c_ret, rs_ret;
    fuzz_input_num("fn rb_char_to_option_kcode", 0);
    fuzz_input_num("c", c);
    if (fuzz_one_in(8)) {
        /* Aliased out-parameters: the last C store wins. */
        fuzz_input_num("aliased", 1);
        c_ret = ref_rb_char_to_option_kcode(c, &c_opt, &c_opt);
        rs_ret = rs_rb_char_to_option_kcode((int)fuzz_rs_long(c), &rs_opt, &rs_opt);
        FUZZ_EQ("ret", c_ret, rs_ret);
        FUZZ_EQ("*option == *kcode", c_opt, rs_opt);
        return;
    }
    c_ret = ref_rb_char_to_option_kcode(c, &c_opt, &c_kc);
    rs_ret = rs_rb_char_to_option_kcode((int)fuzz_rs_long(c), &rs_opt, &rs_kc);
    FUZZ_EQ("ret", c_ret, rs_ret);
    FUZZ_EQ("*option", c_opt, rs_opt);
    FUZZ_EQ("*kcode", c_kc, rs_kc);
}

static void
fuzz_option_to_str(void)
{
    unsigned char c_buf[OPTBUF_SIZE + 2], rs_buf[OPTBUF_SIZE + 2];
    int options;
    char *c_ret, *rs_ret;
    size_t i;

    switch (fuzz_below(4)) {
      case 0: options = (int)fuzz_below(16); break;
      case 1: options = ~(int)fuzz_below(16); break;
      case 2: options = (int)fuzz_below(8) | ((int)fuzz_below(256) << 3); break;
      default: options = (int)(uint32_t)fuzz_next(); break;
    }
    fuzz_input_num("fn option_to_str", 0);
    fuzz_input_num("options", options);
    fuzz_fill(c_buf, sizeof(c_buf), NULL);
    memcpy(rs_buf, c_buf, sizeof(rs_buf));

    c_ret = ref_option_to_str((char *)c_buf + 1, options);
    rs_ret = rs_rb_core_re_option_to_str((char *)rs_buf + 1, (int)fuzz_rs_long(options));
    FUZZ_EQ("ret offset", c_ret - (char *)c_buf, rs_ret - (char *)rs_buf);
    for (i = 0; i < sizeof(c_buf); i++) {
        FUZZ_EQ("buf byte", c_buf[i], rs_buf[i]);
    }
}

/* A byte position in 0..LONG_MAX. */
static long
draw_pos(void)
{
    switch (fuzz_below(5)) {
      case 0: return (long)fuzz_below(8);
      case 1: return (long)fuzz_below(1 << 20);
      case 2: return LONG_MAX - (long)fuzz_below(8);
      case 3: return (long)((unsigned long)INT_MAX + fuzz_below(5)) - 2;
      default: return (long)(fuzz_next() >> 1) & LONG_MAX;
    }
}

static void
fuzz_pair_byte_cmp(void)
{
    pair_t c_a, c_b, rs_a, rs_b;
    long a = draw_pos(), b;

    switch (fuzz_below(4)) {
      case 0: b = a; break;
      case 1: b = a == LONG_MAX ? a - 1 : a + 1; break;
      case 2: b = a == 0 ? 1 : a - 1; break;
      default: b = draw_pos(); break;
    }
    if (fuzz_one_in(2)) { long t = a; a = b; b = t; }
    fuzz_input_num("fn pair_byte_cmp", 0);
    fuzz_input_num("pos1", a);
    fuzz_input_num("pos2", b);
    c_a.byte_pos = a;
    c_b.byte_pos = b;
    c_a.char_pos = (long)fuzz_next();
    c_b.char_pos = (long)fuzz_next();
    rs_a = c_a;
    rs_b = c_b;
    rs_a.byte_pos = fuzz_rs_long(a);
    FUZZ_EQ("ret", ref_pair_byte_cmp(&c_a, &c_b), rs_rb_core_re_pair_byte_cmp(&rs_a, &rs_b));
}

#define NAMEBUF 40
#define MAXGROUPS 16

static void
fuzz_name_iter(void)
{
    static const UChar namebuf[NAMEBUF] = "group_name_one_two_three_four_five_six";
    struct backref_name_tag c_arg[MAXGROUPS], rs_arg[MAXGROUPS];
    int refs[MAXGROUPS], rs_refs[MAXGROUPS];
    size_t ngroups = 1 + (size_t)fuzz_below(MAXGROUPS);
    int back_num, c_ret, rs_ret, i;
    long a = (long)fuzz_below(NAMEBUF - 4), b = a + (long)fuzz_below(NAMEBUF - 4 - a);
    size_t k;

    if (fuzz_one_in(8))
        back_num = (int)fuzz_below(3) - 2; /* -2..0: the loop does not run */
    else
        back_num = 1 + (int)fuzz_below(ngroups);
    for (i = 0; i < back_num; i++) {
        refs[i] = (int)fuzz_below(ngroups);
    }
    memcpy(rs_refs, refs, sizeof(refs));
    fuzz_input_num("fn match_inspect_name_iter", 0);
    fuzz_input_num("name", a);
    fuzz_input_num("name_end", b);
    fuzz_input_num("back_num", back_num);
    fuzz_fill((unsigned char *)c_arg, sizeof(c_arg), NULL);
    memcpy(rs_arg, c_arg, sizeof(rs_arg));

    c_ret = ref_match_inspect_name_iter(namebuf + a, namebuf + b, back_num, refs, NULL, c_arg);
    rs_ret = rs_rb_core_re_match_inspect_name_iter(namebuf + a, namebuf + fuzz_rs_long(b), back_num, rs_refs,
                                                   NULL, rs_arg);
    FUZZ_EQ("ret", c_ret, rs_ret);
    /* Both sides store pointers into the same namebuf (or keep the same
     * random bytes), so the raw values are comparable. */
    for (k = 0; k < MAXGROUPS; k++) {
        FUZZ_EQ("arg[].name", (uintptr_t)c_arg[k].name, (uintptr_t)rs_arg[k].name);
        FUZZ_EQ("arg[].len", c_arg[k].len, rs_arg[k].len);
    }
    for (i = 0; i < back_num; i++) {
        FUZZ_EQ("back_refs untouched", refs[i], rs_refs[i]);
    }
}

#define ESCLEN 24

/* Pieces of escape sequences; concatenated they reach every branch. */
static const char *const tokens[] = {
    "\\", "\\", "\\", "\\\\", "M-", "C-", "c", "M", "C", "-", "x", "\\x", "\\M-", "\\C-", "\\c",
    "0", "1", "3", "7", "8", "9", "377", "400", "777", "a", "f", "F", "g", "4", "41",
    "n", "t", "r", "v", "e", "z", "?", "\x7f", "\x80", "\xe9", "\xff", " ",
};

static size_t
draw_escape(unsigned char *buf)
{
    size_t len = 0, ntok, t, l;

    if (fuzz_one_in(6)) {
        len = fuzz_len(ESCLEN);
        fuzz_fill(buf, len, "\\\\\\MCcx-01234567789afAFgntrve\xe9");
        return len;
    }
    ntok = 1 + (size_t)fuzz_below(6);
    for (t = 0; t < ntok; t++) {
        const char *tok = tokens[fuzz_below(sizeof(tokens) / sizeof(tokens[0]))];
        l = strlen(tok);
        if (len + l > ESCLEN) break;
        memcpy(buf + len, tok, l);
        len += l;
    }
    /* Mostly start like an escape, as the C callers do. */
    if (len > 0 && !fuzz_one_in(8)) buf[0] = '\\';
    return len;
}

static void
fuzz_read_escaped_byte(void)
{
    unsigned char in[ESCLEN + 1];
    onig_errmsg_buffer c_err, rs_err;
    const unsigned char *rs_in;
    const char *c_p, *rs_p;
    size_t len = draw_escape(in), i;
    int c_ret, rs_ret;

    in[len] = (unsigned char)fuzz_below(256); /* sentinel after end */
    fuzz_input_num("fn read_escaped_byte", 0);
    fuzz_input("src", in, len);
    rs_in = fuzz_rs_bytes(in, len);
    fuzz_fill((unsigned char *)c_err, sizeof(c_err), NULL);
    memcpy(rs_err, c_err, sizeof(rs_err));

    c_p = (const char *)in;
    rs_p = (const char *)rs_in;
    c_ret = ref_read_escaped_byte(&c_p, (const char *)in + len, c_err);
    rs_ret = rs_rb_core_re_read_escaped_byte(&rs_p, (const char *)rs_in + len, rs_err);
    FUZZ_EQ("ret", c_ret, rs_ret);
    FUZZ_EQ("*pp advance", c_p - (const char *)in, rs_p - (const char *)rs_in);
    for (i = 0; i < sizeof(c_err); i++) {
        FUZZ_EQ("err byte", (unsigned char)c_err[i], (unsigned char)rs_err[i]);
    }
}

static void
fuzz_one(void)
{
    switch (fuzz_below(8)) {
      case 0: fuzz_char_to_option(); break;
      case 1: fuzz_option_kcode(); break;
      case 2: fuzz_option_to_str(); break;
      case 3: fuzz_pair_byte_cmp(); break;
      case 4: fuzz_name_iter(); break;
      default: fuzz_read_escaped_byte(); break;
    }
}
