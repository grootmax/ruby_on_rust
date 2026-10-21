/*
 * Differential fuzz: siphash.c (port unit siphash-A-01, core_rs/src/siphash.rs)
 * against the original C reference.
 *
 * FUZZ-EXTRACT: siphash.c sip_init_state_bin:186-193 int_sip_dump int_sip_init sip_hash_dump
 */
#define FUZZ_UNIT "siphash_01"
#include <unistd.h>
#include "ruby/ruby.h"
#include "siphash.h"

#define U8TO32_LE(p)         						\
    (((uint32_t)((p)[0])       ) | ((uint32_t)((p)[1]) <<  8) |  	\
     ((uint32_t)((p)[2]) <<  16) | ((uint32_t)((p)[3]) << 24))		\

#define U32TO8_LE(p, v)			\
do {					\
    (p)[0] = (uint8_t)((v)      );	\
    (p)[1] = (uint8_t)((v) >>  8); 	\
    (p)[2] = (uint8_t)((v) >> 16);	\
    (p)[3] = (uint8_t)((v) >> 24);	\
} while (0)

#define U8TO64_LE(p) 							\
    ((uint64_t)U8TO32_LE(p) | ((uint64_t)U8TO32_LE((p) + 4)) << 32 )

#define U64TO8_LE(p, v) \
do {						\
    U32TO8_LE((p),     (uint32_t)((v)      )); 	\
    U32TO8_LE((p) + 4, (uint32_t)((v) >> 32));	\
} while (0)

#define ROTL64(v, s)			\
    ((v) << (s)) | ((v) >> (64 - (s)))

#define ROTL64_TO(v, s) ((v) = ROTL64((v), (s)))

#define ADD64_TO(v, s) ((v) += (s))
#define XOR64_TO(v, s) ((v) ^= (s))

#include FUZZ_REF
#include "fuzz.h"

uint64_t rs_rb_core_siphash_u8to64_le(const uint8_t *p);
void rs_rb_core_siphash_u64to8_le(uint8_t *p, uint64_t v);
uint64_t *rs_rb_core_siphash_rotl64_to(uint64_t *v, unsigned int s);
uint64_t *rs_rb_core_siphash_rotl64_swap(uint64_t *v);
uint64_t *rs_rb_core_siphash_add64_to(uint64_t *v, const uint64_t s);
uint64_t *rs_rb_core_siphash_xor64_to(uint64_t *v, const uint64_t s);
void rs_rb_core_siphash_int_sip_dump(sip_state *state);
void rs_rb_core_siphash_int_sip_init(sip_state *state, const uint8_t key[16]);
void sip_hash_dump(sip_hash *h);

static uint64_t ref_u8to64_le(const uint8_t *p) {
    return U8TO64_LE(p);
}
static void ref_u64to8_le(uint8_t *p, uint64_t v) {
    U64TO8_LE(p, v);
}
static uint64_t *ref_rotl64_to(uint64_t *v, unsigned int s) {
    ROTL64_TO(*v, s);
    return v;
}
static uint64_t *ref_rotl64_swap(uint64_t *v) {
    uint32_t hi = (uint32_t)(*v >> 32);
    uint32_t lo = (uint32_t)(*v);
    *v = ((uint64_t)lo << 32) | (uint64_t)hi;
    return v;
}
static uint64_t *ref_add64_to(uint64_t *v, const uint64_t s) {
    ADD64_TO(*v, s);
    return v;
}
static uint64_t *ref_xor64_to(uint64_t *v, const uint64_t s) {
    XOR64_TO(*v, s);
    return v;
}

static void
fuzz_one(void)
{
    uint8_t key[16];
    uint8_t bytes[8];
    uint8_t c_out8[8], rs_out8[8];
    uint64_t v_c, v_rs, s_val;
    sip_state c_state, rs_state;
    sip_hash c_hash, rs_hash;
    unsigned int shift;
    uint64_t which = fuzz_below(7);

    switch (which) {
      case 0: { // u8to64_le and u64to8_le
        fuzz_fill(bytes, 8, "\x00\x01\x7f\x80\xff\xfe\x12\x34\x56\x78");
        fuzz_input("bytes", bytes, 8);
        const uint8_t *rs_p = fuzz_rs_bytes(bytes, 8);

        v_c = ref_u8to64_le(bytes);
        v_rs = rs_rb_core_siphash_u8to64_le(rs_p);
        FUZZ_EQ("u8to64_le", v_c, v_rs);

        memset(c_out8, 0xaa, 8);
        memset(rs_out8, 0xaa, 8);
        ref_u64to8_le(c_out8, v_c);
        rs_rb_core_siphash_u64to8_le(rs_out8, v_rs);
        FUZZ_EQ("u64to8_le match", memcmp(c_out8, rs_out8, 8), 0);
        break;
      }
      case 1: { // rotl64_to
        v_c = fuzz_below(UINT64_MAX);
        s_val = fuzz_below(UINT64_MAX);
        v_c ^= (s_val << 32);
        v_rs = v_c;
        shift = (unsigned int)fuzz_below(65);
        fuzz_input_num("val", v_c);
        fuzz_input_num("shift", shift);
        v_rs = (uint64_t)fuzz_rs_long((long)v_rs);

        ref_rotl64_to(&v_c, shift);
        rs_rb_core_siphash_rotl64_to(&v_rs, shift);
        FUZZ_EQ("rotl64_to", v_c, v_rs);
        break;
      }
      case 2: { // rotl64_swap
        v_c = fuzz_below(UINT64_MAX);
        s_val = fuzz_below(UINT64_MAX);
        v_c ^= (s_val << 32);
        v_rs = v_c;
        fuzz_input_num("val", v_c);
        v_rs = (uint64_t)fuzz_rs_long((long)v_rs);

        ref_rotl64_swap(&v_c);
        rs_rb_core_siphash_rotl64_swap(&v_rs);
        FUZZ_EQ("rotl64_swap", v_c, v_rs);
        break;
      }
      case 3: { // add64_to
        v_c = fuzz_below(UINT64_MAX);
        s_val = fuzz_below(UINT64_MAX);
        v_rs = v_c;
        fuzz_input_num("val", v_c);
        fuzz_input_num("s", s_val);
        v_rs = (uint64_t)fuzz_rs_long((long)v_rs);

        ref_add64_to(&v_c, s_val);
        rs_rb_core_siphash_add64_to(&v_rs, s_val);
        FUZZ_EQ("add64_to", v_c, v_rs);
        break;
      }
      case 4: { // xor64_to
        v_c = fuzz_below(UINT64_MAX);
        s_val = fuzz_below(UINT64_MAX);
        v_rs = v_c;
        fuzz_input_num("val", v_c);
        fuzz_input_num("s", s_val);
        v_rs = (uint64_t)fuzz_rs_long((long)v_rs);

        ref_xor64_to(&v_c, s_val);
        rs_rb_core_siphash_xor64_to(&v_rs, s_val);
        FUZZ_EQ("xor64_to", v_c, v_rs);
        break;
      }
      case 5: { // int_sip_init
        fuzz_fill(key, 16, "0123456789abcdef\x00\xff\x7f\x80");
        fuzz_input("key", key, 16);
        const uint8_t *rs_k = fuzz_rs_bytes(key, 16);

        memset(&c_state, 0x00, sizeof(c_state));
        memset(&rs_state, 0x00, sizeof(rs_state));
        c_state.c = 2; c_state.d = 4;
        rs_state.c = 2; rs_state.d = 4;

        ref_int_sip_init(&c_state, key);
        rs_rb_core_siphash_int_sip_init(&rs_state, rs_k);

        FUZZ_EQ("state.v[0]", c_state.v[0], rs_state.v[0]);
        FUZZ_EQ("state.v[1]", c_state.v[1], rs_state.v[1]);
        FUZZ_EQ("state.v[2]", c_state.v[2], rs_state.v[2]);
        FUZZ_EQ("state.v[3]", c_state.v[3], rs_state.v[3]);
        break;
      }
      case 6: { // int_sip_dump & sip_hash_dump
        int saved_stdout = dup(fileno(stdout));
        FILE *devnull = fopen("/dev/null", "w");
        if (devnull) {
            dup2(fileno(devnull), fileno(stdout));
            fclose(devnull);
        }

        memset(&c_state, 0x12, sizeof(c_state));
        memset(&rs_state, 0x12, sizeof(rs_state));
        for (int i = 0; i < 4; i++) {
            c_state.v[i] = fuzz_below(UINT64_MAX);
            rs_state.v[i] = c_state.v[i];
        }
        fuzz_input_num("v0", c_state.v[0]);

        ref_int_sip_dump(&c_state);
        rs_rb_core_siphash_int_sip_dump(&rs_state);

        memset(&c_hash, 0x00, sizeof(c_hash));
        memset(&rs_hash, 0x00, sizeof(rs_hash));
        c_hash.state[0] = c_state;
        rs_hash.state[0] = rs_state;

        ref_sip_hash_dump(&c_hash);
        sip_hash_dump(&rs_hash);

        fflush(stdout);
        if (saved_stdout >= 0) {
            dup2(saved_stdout, fileno(stdout));
            close(saved_stdout);
        }

        FUZZ_EQ("state.v[0] after dump", c_state.v[0], rs_state.v[0]);
        break;
      }
    }
}
