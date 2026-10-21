//! Ports of `siphash.c`: SipHash state initialization, dumping, and 64-bit helper utilities.

use core::ffi::{c_char, c_int, c_uint, c_ulong};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct sip_state {
    pub c: c_int,
    pub d: c_int,
    pub v: [u64; 4],
    pub buf: [u8; 8],
    pub buflen: u8,
    pub msglen_byte: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct sip_hash {
    pub state: [sip_state; 1],
    pub methods: *const core::ffi::c_void,
}

pub const SIP_INIT_STATE: [u64; 4] = [
    0x736f6d6570736575, // "uespemos" in LE u64
    0x646f72616e646f6d, // "modnarod" in LE u64
    0x6c7967656e657261, // "arenegyl" in LE u64
    0x7465646279746573, // "setybdet" in LE u64
];

/// Converts 8 bytes in little-endian order to a 64-bit unsigned integer.
pub fn u8to64_le(bytes: &[u8; 8]) -> u64 {
    u64::from_le_bytes(*bytes)
}

/// Converts a 64-bit unsigned integer into 8 little-endian bytes.
pub fn u64to8_le(v: u64, out: &mut [u8; 8]) {
    *out = v.to_le_bytes();
}

/// Rotates a 64-bit integer left by `s` bits.
pub fn rotl64_to(v: &mut u64, s: u32) -> &mut u64 {
    let shift = s % 64;
    *v = v.rotate_left(shift);
    v
}

/// Swaps the high and low 32-bit halves of a 64-bit integer.
pub fn rotl64_swap(v: &mut u64) -> &mut u64 {
    let hi = (*v >> 32) as u32;
    let lo = *v as u32;
    *v = ((lo as u64) << 32) | (hi as u64);
    v
}

/// Adds `s` to `*v` with wrapping 64-bit arithmetic.
pub fn add64_to(v: &mut u64, s: u64) -> &mut u64 {
    *v = v.wrapping_add(s);
    v
}

/// Bitwise XORs `s` into `*v`.
pub fn xor64_to(v: &mut u64, s: u64) -> &mut u64 {
    *v ^= s;
    v
}

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Dumps the 4 internal 64-bit state words of `state` to stdout.
pub fn int_sip_dump(state: &sip_state) {
    for v in 0..4 {
        // SAFETY: printf format string and arguments match
        unsafe {
            printf(
                c"v%d: %lx\n".as_ptr(),
                v as c_int,
                state.v[v] as c_ulong,
            );
        }
    }
}

/// Initializes `state` with the 16-byte `key` and initial state constants.
pub fn int_sip_init(state: &mut sip_state, key: &[u8; 16]) {
    let k0 = u64::from_le_bytes(key[0..8].try_into().unwrap());
    let k1 = u64::from_le_bytes(key[8..16].try_into().unwrap());

    state.v[0] = k0 ^ SIP_INIT_STATE[0];
    state.v[1] = k1 ^ SIP_INIT_STATE[1];
    state.v[2] = k0 ^ SIP_INIT_STATE[2];
    state.v[3] = k1 ^ SIP_INIT_STATE[3];
}

/// Dumps the SipHash state contained in `h`.
pub fn sip_hash_dump_safe(h: &sip_hash) {
    int_sip_dump(&h.state[0]);
}

// C FFI Exported Wrappers

/// C wrapper for `u8to64_le`.
///
/// # Safety
/// `p` must point to at least 8 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_u8to64_le(p: *const u8) -> u64 {
    // SAFETY: caller guarantees `p` points to 8 readable bytes.
    let bytes = unsafe { &*(p as *const [u8; 8]) };
    u8to64_le(bytes)
}

/// C wrapper for `u64to8_le`.
///
/// # Safety
/// `p` must point to at least 8 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_u64to8_le(p: *mut u8, v: u64) {
    // SAFETY: caller guarantees `p` points to 8 writable bytes.
    let out = unsafe { &mut *(p as *mut [u8; 8]) };
    u64to8_le(v, out);
}

/// C wrapper for `rotl64_to`.
///
/// # Safety
/// `v` must be a valid, non-null pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_rotl64_to(v: *mut u64, s: c_uint) -> *mut u64 {
    // SAFETY: caller guarantees `v` is valid.
    let v_ref = unsafe { &mut *v };
    rotl64_to(v_ref, s as u32);
    v
}

/// C wrapper for `rotl64_swap`.
///
/// # Safety
/// `v` must be a valid, non-null pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_rotl64_swap(v: *mut u64) -> *mut u64 {
    // SAFETY: caller guarantees `v` is valid.
    let v_ref = unsafe { &mut *v };
    rotl64_swap(v_ref);
    v
}

/// C wrapper for `add64_to`.
///
/// # Safety
/// `v` must be a valid, non-null pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_add64_to(v: *mut u64, s: u64) -> *mut u64 {
    // SAFETY: caller guarantees `v` is valid.
    let v_ref = unsafe { &mut *v };
    add64_to(v_ref, s);
    v
}

/// C wrapper for `xor64_to`.
///
/// # Safety
/// `v` must be a valid, non-null pointer to a `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_xor64_to(v: *mut u64, s: u64) -> *mut u64 {
    // SAFETY: caller guarantees `v` is valid.
    let v_ref = unsafe { &mut *v };
    xor64_to(v_ref, s);
    v
}

/// C wrapper for `int_sip_dump`.
///
/// # Safety
/// `state` must point to a valid, readable `sip_state`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_int_sip_dump(state: *const sip_state) {
    // SAFETY: caller guarantees `state` is valid.
    let state_ref = unsafe { &*state };
    int_sip_dump(state_ref);
}

/// C wrapper for `int_sip_init`.
///
/// # Safety
/// `state` must point to a valid, writable `sip_state`. `key` must point to 16 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_siphash_int_sip_init(state: *mut sip_state, key: *const u8) {
    // SAFETY: caller guarantees `state` and `key` are valid.
    let state_ref = unsafe { &mut *state };
    let key_ref = unsafe { &*(key as *const [u8; 16]) };
    int_sip_init(state_ref, key_ref);
}

/// C wrapper for `sip_hash_dump`.
///
/// # Safety
/// `h` must point to a valid, readable `sip_hash`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sip_hash_dump(h: *const sip_hash) {
    // SAFETY: caller guarantees `h` is valid.
    let h_ref = unsafe { &*h };
    sip_hash_dump_safe(h_ref);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8to64_le_and_u64to8_le() {
        let bytes: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        let val = u8to64_le(&bytes);
        assert_eq!(val, 0xf0debc9a78563412);

        let mut out = [0u8; 8];
        u64to8_le(val, &mut out);
        assert_eq!(out, bytes);
    }

    #[test]
    fn test_rotl64_to_and_swap() {
        let mut v = 0x123456789abcdef0u64;
        rotl64_to(&mut v, 8);
        assert_eq!(v, 0x3456789abcdef012u64);

        let mut v2 = 0x123456789abcdef0u64;
        rotl64_swap(&mut v2);
        assert_eq!(v2, 0x9abcdef012345678u64);
    }

    #[test]
    fn test_add64_to_and_xor64_to() {
        let mut v = 100u64;
        add64_to(&mut v, 50);
        assert_eq!(v, 150);

        xor64_to(&mut v, 0xff);
        assert_eq!(v, 150 ^ 0xff);
    }

    #[test]
    fn test_int_sip_init() {
        let mut st = sip_state {
            c: 2,
            d: 4,
            v: [0; 4],
            buf: [0; 8],
            buflen: 0,
            msglen_byte: 0,
        };
        let key: [u8; 16] = *b"0123456789abcdef";
        int_sip_init(&mut st, &key);

        let k0 = u64::from_le_bytes(*b"01234567");
        let k1 = u64::from_le_bytes(*b"89abcdef");
        assert_eq!(st.v[0], k0 ^ SIP_INIT_STATE[0]);
        assert_eq!(st.v[1], k1 ^ SIP_INIT_STATE[1]);
        assert_eq!(st.v[2], k0 ^ SIP_INIT_STATE[2]);
        assert_eq!(st.v[3], k1 ^ SIP_INIT_STATE[3]);
    }
}
