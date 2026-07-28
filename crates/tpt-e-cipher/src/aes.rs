//! Constant-time AES-128 block cipher.
//!
//! This implementation uses algebraic operations in GF(2^8) — specifically,
//! the multiplicative inverse computed via an addition-chain exponentiation
//! followed by the AES affine transformation. Every operation is a fixed
//! sequence of bitwise AND, XOR, and shifts with no table lookups and no
//! data-dependent branches, guaranteeing constant-time execution regardless
//! of plaintext or key material.
//!
//! # Constant-Time Guarantee
//!
//! - The S-box is computed algebraically (no T-tables, no lookup tables).
//! - The `gf256_mul` function uses masking instead of branching.
//! - The key schedule uses the same constant-time S-box.
//! - ShiftRows, MixColumns, and AddRoundKey are inherently constant-time
//!   (bit permutation, XOR, and conditional XOR respectively).

use crate::traits::Aes;

// ---------------------------------------------------------------------------
// GF(2^8) arithmetic — AES polynomial: x^8 + x^4 + x^3 + x + 1 = 0x11B
// ---------------------------------------------------------------------------

/// Constant-time GF(2^8) multiplication using Russian peasant algorithm.
///
/// No branches, no table lookups — only shifts, masks, and XOR.
pub(crate) fn gf256_mul(a: u8, b: u8) -> u8 {
    let mut p: u8 = 0;
    let mut a = a;
    let mut b = b;
    for _ in 0..8 {
        // Constant-time conditional XOR: p ^= a if bit 0 of b is set.
        let mask = (b & 1).wrapping_neg(); // 0x00 or 0xFF
        p ^= a & mask;
        // Multiply by x (shift left), conditionally reduce mod AES polynomial.
        let hi = (a >> 7) & 1;
        a = (a << 1) ^ (hi * 0x1B);
        b >>= 1;
    }
    p
}

/// Constant-time GF(2^8) multiplicative inverse via addition-chain
/// exponentiation: x^(-1) = x^254 for x != 0, and 0^(-1) = 0 (AES convention).
///
/// The addition chain uses12 multiplications — all through the constant-time
/// `gf256_mul`. The total operation count is fixed regardless of input.
pub(crate) fn gf256_inv(x: u8) -> u8 {
    let x2 = gf256_mul(x, x);
    let x3 = gf256_mul(x2, x);
    let x6 = gf256_mul(x3, x3);
    let x7 = gf256_mul(x6, x);
    let x14 = gf256_mul(x7, x7);
    let x15 = gf256_mul(x14, x);
    let x30 = gf256_mul(x15, x15);
    let x31 = gf256_mul(x30, x);
    let x62 = gf256_mul(x31, x31);
    let x63 = gf256_mul(x62, x);
    let x126 = gf256_mul(x63, x63);
    let x127 = gf256_mul(x126, x);
    gf256_mul(x127, x127) // x^254
}

// ---------------------------------------------------------------------------
// AES S-box — algebraic (no table lookup), constant-time
// ---------------------------------------------------------------------------

/// Compute the AES S-box for a single byte using algebraic operations.
///
/// The S-box is defined as: S(x) = A · x^(-1) ⊕ 0x63, where the inverse
/// is in GF(2^8) with the AES irreducible polynomial. The affine
/// transformation `A` is expressed as a sequence of bit rotations and XOR:
///
/// ```text
/// y[i] = x[i] ⊕ x[(i+4) mod 8] ⊕ x[(i+5) mod 8] ⊕ x[(i+6) mod 8]
///         ⊕ x[(i+7) mod 8] ⊕ c[i]
/// ```
///
/// where c = 0x63.
///
/// For x = 0: gf256_inv(0) = 0 (since 0^254 = 0), and the affine
/// transform of 0 yields 0x63, matching the AES specification.
pub(crate) fn aes_sbox(x: u8) -> u8 {
    let inv = gf256_inv(x);
    // Affine transformation via rotations — all bitwise, no branches.
    inv ^ inv.rotate_left(4) ^ inv.rotate_left(3) ^ inv.rotate_left(2) ^ inv.rotate_left(1) ^ 0x63
}

// ---------------------------------------------------------------------------
// AES ShiftRows — constant-time byte permutation
// ---------------------------------------------------------------------------

/// Apply the AES ShiftRows transformation to a 16-byte state (column-major).
///
/// Row 0: no shift. Row 1: shift left 1. Row 2: shift left 2. Row 3: shift left 3.
pub(crate) fn shift_rows(state: &mut [u8; 16]) {
    // Row 1: bytes at positions 1,5,9,13 → 5,9,13,1
    let t = state[1];
    state[1] = state[5];
    state[5] = state[9];
    state[9] = state[13];
    state[13] = t;
    // Row 2: bytes at positions 2,6,10,14 → 10,14,2,6
    let t = state[2];
    state[2] = state[10];
    state[10] = t;
    let t = state[6];
    state[6] = state[14];
    state[14] = t;
    // Row 3: bytes at positions 3,7,11,15 → 15,3,7,11
    let t = state[3];
    state[3] = state[15];
    state[15] = state[11];
    state[11] = state[7];
    state[7] = t;
}

// ---------------------------------------------------------------------------
// AES MixColumns — constant-time linear transformation in GF(2^8)
// ---------------------------------------------------------------------------

/// Multiply by x (i.e., by 2) in GF(2^8) with the AES polynomial.
pub(crate) fn xtime(x: u8) -> u8 {
    let hi = (x >> 7) & 1;
    (x << 1) ^ (hi * 0x1B)
}

/// Apply the AES MixColumns transformation to a single 4-byte column.
pub(crate) fn mix_column(col: &mut [u8; 4]) {
    let a = col[0];
    let b = col[1];
    let c = col[2];
    let d = col[3];
    let ae = xtime(a);
    let be = xtime(b);
    let ce = xtime(c);
    let de = xtime(d);
    col[0] = ae ^ be ^ b ^ c ^ d;
    col[1] = a ^ be ^ ce ^ c ^ d;
    col[2] = a ^ b ^ ce ^ de ^ d;
    col[3] = ae ^ a ^ b ^ c ^ de;
}

/// Apply MixColumns to all four columns of the state.
pub(crate) fn mix_columns(state: &mut [u8; 16]) {
    for i in 0..4 {
        let start = i * 4;
        let mut col = [state[start], state[start + 1], state[start + 2], state[start + 3]];
        mix_column(&mut col);
        state[start] = col[0];
        state[start + 1] = col[1];
        state[start + 2] = col[2];
        state[start + 3] = col[3];
    }
}

// ---------------------------------------------------------------------------
// AES AddRoundKey — constant-time XOR
// ---------------------------------------------------------------------------

/// XOR each state byte with the corresponding round key byte.
pub(crate) fn add_round_key(state: &mut [u8; 16], round_key: &[u8; 16]) {
    for i in 0..16 {
        state[i] ^= round_key[i];
    }
}

// ---------------------------------------------------------------------------
// AES-128 Key Expansion
// ---------------------------------------------------------------------------

/// AES round constants for key expansion.
const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36];

/// Expand a 128-bit key into11 round keys (176 bytes).
pub(crate) fn expand_key(key: &[u8; 16]) -> [[u8; 16]; 11] {
    let mut rk = [[0u8; 16]; 11];
    rk[0] = *key;

    for i in 1..=10 {
        // Start with the last 4 bytes of the previous round key.
        let mut temp = [rk[i - 1][12], rk[i - 1][13], rk[i - 1][14], rk[i - 1][15]];
        // RotWord: rotate left by1 byte.
        temp = [temp[1], temp[2], temp[3], temp[0]];
        // SubWord: apply S-box to each byte.
        temp = [aes_sbox(temp[0]), aes_sbox(temp[1]), aes_sbox(temp[2]), aes_sbox(temp[3])];
        // XOR round constant.
        temp[0] ^= RCON[i - 1];
        // First 4 bytes of new round key.
        for j in 0..4 {
            rk[i][j] = rk[i - 1][j] ^ temp[j];
        }
        // Remaining 12 bytes.
        for j in 4..16 {
            rk[i][j] = rk[i - 1][j] ^ rk[i][j - 4];
        }
    }
    rk
}

// ---------------------------------------------------------------------------
// AES-128 Engine — public API
// ---------------------------------------------------------------------------

/// Constant-time AES-128 encryption engine.
///
/// Created with a 128-bit key. Encryption uses algebraic GF(2^8) operations
/// exclusively — no table lookups, no data-dependent branches.
///
/// # Example
///
/// ```rust
/// use tpt_e_cipher::aes::AesEngine;
/// use tpt_e_cipher::traits::Aes;
///
/// let key = [0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
///            0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c];
/// let mut engine = AesEngine::new(&key);
/// let mut block = [0u8; 16];
/// engine.encrypt_block(&mut block);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AesEngine {
    round_keys: [[u8; 16]; 11],
}

impl AesEngine {
    /// Create a new AES-128 engine with the given128-bit key.
    pub fn new(key: &[u8; 16]) -> Self {
        Self { round_keys: expand_key(key) }
    }

    /// Get a reference to the expanded round keys.
    pub fn round_keys(&self) -> &[[u8; 16]; 11] {
        &self.round_keys
    }
}

impl Aes for AesEngine {
    fn encrypt_block(&mut self, block: &mut [u8; 16]) {
        // Initial round key addition.
        add_round_key(block, &self.round_keys[0]);

        // 9 main rounds.
        for round in 1..10 {
            // SubBytes — algebraic S-box, constant-time.
            for byte in block.iter_mut() {
                *byte = aes_sbox(*byte);
            }
            shift_rows(block);
            mix_columns(block);
            add_round_key(block, &self.round_keys[round]);
        }

        // Final round (no MixColumns).
        for byte in block.iter_mut() {
            *byte = aes_sbox(*byte);
        }
        shift_rows(block);
        add_round_key(block, &self.round_keys[10]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Full S-box verification against FIPS 197 Table.
    #[test]
    fn sbox_full_table() {
        let table: [u8; 256] = [
            0x63,0x7C,0x77,0x7B,0xF2,0x6B,0x6F,0xC5,0x30,0x01,0x67,0x2B,0xFE,0xD7,0xAB,0x76,
            0xCA,0x82,0xC9,0x7D,0xFA,0x59,0x47,0xF0,0xAD,0xD4,0xA2,0xAF,0x9C,0xA4,0x72,0xC0,
            0xB7,0xFD,0x93,0x26,0x36,0x3F,0xF7,0xCC,0x34,0xA5,0xE5,0xF1,0x71,0xD8,0x31,0x15,
            0x04,0xC7,0x23,0xC3,0x18,0x96,0x05,0x9A,0x07,0x12,0x80,0xE2,0xEB,0x27,0xB2,0x75,
            0x09,0x83,0x2C,0x1A,0x1B,0x6E,0x5A,0xA0,0x52,0x3B,0xD6,0xB3,0x29,0xE3,0x2F,0x84,
            0x53,0xD1,0x00,0xED,0x20,0xFC,0xB1,0x5B,0x6A,0xCB,0xBE,0x39,0x4A,0x4C,0x58,0xCF,
            0xD0,0xEF,0xAA,0xFB,0x43,0x4D,0x33,0x85,0x45,0xF9,0x02,0x7F,0x50,0x3C,0x9F,0xA8,
            0x51,0xA3,0x40,0x8F,0x92,0x9D,0x38,0xF5,0xBC,0xB6,0xDA,0x21,0x10,0xFF,0xF3,0xD2,
            0xCD,0x0C,0x13,0xEC,0x5F,0x97,0x44,0x17,0xC4,0xA7,0x7E,0x3D,0x64,0x5D,0x19,0x73,
            0x60,0x81,0x4F,0xDC,0x22,0x2A,0x90,0x88,0x46,0xEE,0xB8,0x14,0xDE,0x5E,0x0B,0xDB,
            0xE0,0x32,0x3A,0x0A,0x49,0x06,0x24,0x5C,0xC2,0xD3,0xAC,0x62,0x91,0x95,0xE4,0x79,
            0xE7,0xC8,0x37,0x6D,0x8D,0xD5,0x4E,0xA9,0x6C,0x56,0xF4,0xEA,0x65,0x7A,0xAE,0x08,
            0xBA,0x78,0x25,0x2E,0x1C,0xA6,0xB4,0xC6,0xE8,0xDD,0x74,0x1F,0x4B,0xBD,0x8B,0x8A,
            0x70,0x3E,0xB5,0x66,0x48,0x03,0xF6,0x0E,0x61,0x35,0x57,0xB9,0x86,0xC1,0x1D,0x9E,
            0xE1,0xF8,0x98,0x11,0x69,0xD9,0x8E,0x94,0x9B,0x1E,0x87,0xE9,0xCE,0x55,0x28,0xDF,
            0x8C,0xA1,0x89,0x0D,0xBF,0xE6,0x42,0x68,0x41,0x99,0x2D,0x0F,0xB0,0x54,0xBB,0x16,
        ];
        let mut mismatches = 0;
        for (i, expected) in table.iter().enumerate() {
            let got = aes_sbox(i as u8);
            if got != *expected {
                mismatches += 1;
                if mismatches <= 10 {
                    panic!("S(0x{:02X}) = 0x{:02X}, expected 0x{:02X}", i, got, expected);
                }
            }
        }
        assert_eq!(mismatches, 0, "{} S-box entries wrong", mismatches);
    }

    /// Spot-check a few S-box entries against FIPS 197.
    #[test]
    fn sbox_spot_checks() {
        assert_eq!(aes_sbox(0x00), 0x63);
        assert_eq!(aes_sbox(0x53), 0xED);
        assert_eq!(aes_sbox(0xD0), 0x70);
        assert_eq!(aes_sbox(0xFF), 0x16);
        assert_eq!(aes_sbox(0x01), 0x7C);
        assert_eq!(aes_sbox(0xC2), 0x25);
    }

    /// Verify gf256_mul correctness: a * a_inv = 1.
    #[test]
    fn gf256_mul_inverse() {
        for x in 1u8..=255 {
            let inv = gf256_inv(x);
            let product = gf256_mul(x, inv);
            assert_eq!(product, 1, "gf256_mul(0x{:02X}, gf256_inv(0x{:02X})) = 0x{:02X}, expected 1", x, x, product);
        }
    }

    /// NIST AES-128 test vector (FIPS 197, Appendix B).
    #[test]
    fn aes128_nist_vector() {
        let key = [0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
                   0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C];
        let plaintext = [0x32, 0x43, 0xF6, 0xA8, 0x88, 0x5A, 0x30, 0x8D,
                         0x31, 0x31, 0x98, 0xA2, 0xE0, 0x37, 0x07, 0x34];
        let expected = [0x39, 0x25, 0x84, 0x1D, 0x02, 0xDC, 0x09, 0xFB,
                        0xDC, 0x11, 0x85, 0x97, 0x19, 0x6A, 0x0B, 0x32];

        let mut engine = AesEngine::new(&key);
        let mut block = plaintext;
        engine.encrypt_block(&mut block);
        assert_eq!(block, expected);
    }

    /// NIST AES-128 test vector — all-zero key and plaintext.
    #[test]
    fn aes128_zero_key_zero_plaintext() {
        let key = [0u8; 16];
        let plaintext = [0u8; 16];
        let expected = [0x66, 0xE9, 0x4B, 0xD4, 0xEF, 0x8A, 0x2C, 0x3B,
                        0x88, 0x4C, 0xFA, 0x59, 0xCA, 0x34, 0x2B, 0x2E];

        let mut engine = AesEngine::new(&key);
        let mut block = plaintext;
        engine.encrypt_block(&mut block);
        assert_eq!(block, expected);
    }

    /// Deterministic: same key + same plaintext always yields same ciphertext.
    #[test]
    fn aes128_deterministic() {
        let key = [0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
                   0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C];
        let plaintext = [0x6B, 0xC1, 0xBE, 0xE2, 0x2E, 0x40, 0x9F, 0x96,
                         0xE9, 0x3D, 0x7E, 0x11, 0x73, 0x93, 0x17, 0x2A];

        let mut engine1 = AesEngine::new(&key);
        let mut engine2 = AesEngine::new(&key);
        let mut block1 = plaintext;
        let mut block2 = plaintext;

        engine1.encrypt_block(&mut block1);
        engine2.encrypt_block(&mut block2);
        assert_eq!(block1, block2);
    }

    /// NIST SP 800-38A / CAVP — AES-128 ECB test vector block 1.
    #[test]
    fn aes128_nist_sp800_38a() {
        let key = [0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
                   0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C];
        let plaintext = [0x6B, 0xC1, 0xBE, 0xE2, 0x2E, 0x40, 0x9F, 0x96,
                         0xE9, 0x3D, 0x7E, 0x11, 0x73, 0x93, 0x17, 0x2A];
        let expected = [0x3A, 0xD7, 0x7B, 0xB4, 0x0D, 0x7A, 0x36, 0x60,
                        0xA8, 0x9E, 0xCA, 0xF3, 0x24, 0x66, 0xEF, 0x97];

        let mut engine = AesEngine::new(&key);
        let mut block = plaintext;
        engine.encrypt_block(&mut block);
        assert_eq!(block, expected);
    }
}
