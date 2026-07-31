//! P-256 (secp256r1) ECDSA implementation.
//!
//! This is a software implementation — no hardware ECC peripheral backend
//! exists yet, so unlike [`crate::aes`] and [`crate::sha`] this is the
//! crate's only `Ecc` implementation (not gated behind the `mock` feature).
//!
//! **Constant-time status**: point arithmetic (`point_mul`'s inner loop)
//! uses the Renes–Costello–Batina complete addition/doubling formulas
//! (`proj_add`, `proj_double`) plus a bitmask-based accumulator select
//! (`fe_select`), so neither the scalar's bit pattern nor intermediate
//! point structure (identity/doubling/inverse) drives control flow. This
//! closes the previous gap where `point_add`/`point_double` branched on
//! point structure. **Not yet addressed**: the underlying big-integer field
//! arithmetic (`fe_add`/`fe_sub`'s conditional-subtract modular reduction,
//! `fe_inv`, `u256_cmp`) still uses value-dependent branches — a separate,
//! lower-layer timing channel common to schoolbook bignum implementations.
//! Do not use this where signing-timing side channels are in the threat
//! model until that layer is also addressed (e.g. `esp-hal` hardware
//! backing, or fully branchless field arithmetic).
//!
//! The implementation covers:
//! - Field arithmetic in GF(p) where p = 2^256 − 2^224 + 2^192 + 2^96 − 1
//! - Point operations on the Weierstrass curve y² = x³ − 3x + b (mod p)
//! - ECDSA sign and verify (FIPS 186-4), with public-key on-curve
//!   validation, hash-to-scalar reduction mod n, and canonical low-s
//!   signatures (non-malleable)

use crate::traits::Ecc;

// ---------------------------------------------------------------------------
// P-256 constants
// ---------------------------------------------------------------------------

/// P-256 prime: p = 2^256 − 2^224 + 2^192 + 2^96 − 1
const P: [u64; 4] = [0xFFFFFFFFFFFFFFFF, 0x00000000FFFFFFFF, 0x0000000000000000, 0xFFFFFFFF00000001];

/// Curve order n (number of points on the curve).
const N: [u64; 4] = [0xF3B9CAC2FC632551, 0xBCE6FAADA7179E84, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFF00000000];

/// Curve parameter b.
const B: [u64; 4] = [0x3BCE3C3E27D2604B, 0x651D06B0CC53B0F6, 0xB3EBBD55769886BC, 0x5AC635D8AA3A93E7];

/// Generator point Gx coordinate.
const GX: [u64; 4] = [0xF4A13945D898C296, 0x77037D812DEB33A0, 0xF8BCE6E563A440F2, 0x6B17D1F2E12C4247];

/// Generator point Gy coordinate.
const GY: [u64; 4] = [0xCBB6406837BF51F5, 0x2BCE33576B315ECE, 0x8EE7EB4A7C0F9E16, 0x4FE342E2FE1A7F9B];

// ---------------------------------------------------------------------------
// 256-bit arithmetic helpers
// ---------------------------------------------------------------------------

/// Add two 256-bit numbers, returning (result, carry).
fn u256_add(a: [u64; 4], b: [u64; 4]) -> ([u64; 4], u8) {
    let mut result = [0u64; 4];
    let mut carry = 0u128;
    for i in 0..4 {
        let s = (a[i] as u128) + (b[i] as u128) + carry;
        result[i] = s as u64;
        carry = s >> 64;
    }
    (result, carry as u8)
}

/// Subtract two 256-bit numbers, returning (result, borrow).
fn u256_sub(a: [u64; 4], b: [u64; 4]) -> ([u64; 4], u8) {
    let mut result = [0u64; 4];
    let mut borrow = 0i128;
    for i in 0..4 {
        let s = (a[i] as i128) - (b[i] as i128) - borrow;
        result[i] = s as u64;
        borrow = if s < 0 { 1 } else { 0 };
    }
    (result, borrow as u8)
}

/// Compare two 256-bit numbers. Returns Ordering.
fn u256_cmp(a: [u64; 4], b: [u64; 4]) -> core::cmp::Ordering {
    for i in (0..4).rev() {
        match a[i].cmp(&b[i]) {
            core::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    core::cmp::Ordering::Equal
}

/// Check if a 256-bit number is zero.
fn u256_is_zero(a: [u64; 4]) -> bool {
    a.iter().all(|&x| x == 0)
}

/// Right-shift a 256-bit number by 1 bit (divide by 2, rounding down).
fn u256_shr1(a: [u64; 4]) -> [u64; 4] {
    let mut r = [0u64; 4];
    let mut carry = 0u64;
    for i in (0..4).rev() {
        r[i] = (a[i] >> 1) | (carry << 63);
        carry = a[i] & 1;
    }
    r
}

// ---------------------------------------------------------------------------
// P-256 field arithmetic (mod p)
// ---------------------------------------------------------------------------

/// Schoolbook multiplication producing a 512-bit product.
fn mul_wide(a: [u64; 4], b: [u64; 4]) -> [u64; 8] {
    let mut result = [0u64; 8];
    for i in 0..4 {
        let mut carry = 0u128;
        for j in 0..4 {
            let s = (result[i + j] as u128) + (a[i] as u128) * (b[j] as u128) + carry;
            result[i + j] = s as u64;
            carry = s >> 64;
        }
        result[i + 4] += carry as u64;
    }
    result
}

/// P-256 fast reduction: reduce a 512-bit product modulo p.
///
/// Uses the identity 2^256 ≡ 2^224 − 2^192 − 2^96 + 1 (mod p).
/// Repeatedly folds the upper 256 bits into the lower 256 bits by
/// multiplying by C and adding, until the upper part is zero.
fn p256_redc(t: [u64; 8]) -> [u64; 4] {
    // Reduction factor: C = 2^224 - 2^192 - 2^96 + 1
    // In [u64; 4] little-endian: [1, 0xFFFFFFFF00000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFE]
    const C: [u64; 4] = [1, 0xFFFF_FFFF_0000_0000, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFE];

    let mut lo = [t[0], t[1], t[2], t[3]];
    let mut hi = [t[4], t[5], t[6], t[7]];

    // Each iteration reduces hi by ~31 bits. At most ~10 iterations needed.
    for _ in 0..12 {
        if hi == [0, 0, 0, 0] {
            break;
        }
        // V = lo + hi * 2^256 ≡ lo + hi * C (mod p)
        let product = mul_wide(hi, C);
        let (new_lo, carry) = u256_add(lo, [product[0], product[1], product[2], product[3]]);
        // new_hi = product_hi + carry
        let mut new_hi = [product[4], product[5], product[6], product[7]];
        let mut c = carry as u128;
        for i in 0..4 {
            let s = (new_hi[i] as u128) + c;
            new_hi[i] = s as u64;
            c = s >> 64;
        }
        lo = new_lo;
        hi = new_hi;
    }

    sub_p_if_ge(lo)
}

/// If r >= p, return r - p; otherwise return r.
fn sub_p_if_ge(r: [u64; 4]) -> [u64; 4] {
    if u256_cmp(r, P) != core::cmp::Ordering::Less {
        let (diff, _) = u256_sub(r, P);
        diff
    } else {
        r
    }
}

/// Field addition mod p.
fn fe_add(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let (sum, carry) = u256_add(a, b);
    // If there was a carry or sum >= p, subtract p
    if carry != 0 || u256_cmp(sum, P) != core::cmp::Ordering::Less {
        let (diff, _) = u256_sub(sum, P);
        diff
    } else {
        sum
    }
}

/// Field subtraction mod p.
fn fe_sub(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let (diff, borrow) = u256_sub(a, b);
    if borrow != 0 {
        let (sum, _) = u256_add(diff, P);
        sum
    } else {
        diff
    }
}

/// Field multiplication mod p.
fn fe_mul(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let product = mul_wide(a, b);
    p256_redc(product)
}

/// Field squaring mod p.
fn fe_sqr(a: [u64; 4]) -> [u64; 4] {
    fe_mul(a, a)
}

/// Field doubling (a + a) mod p.
fn fe_double(a: [u64; 4]) -> [u64; 4] {
    fe_add(a, a)
}

/// Constant-time select: returns `a` if `bit` (0 or 1) is 1, else `b`.
///
/// Implemented via arithmetic bitmasking rather than a Rust `if`, so this
/// never becomes a data-dependent branch instruction: selecting on a secret
/// scalar bit via `if bit == 1 { a } else { b }` is exactly the kind of
/// control-flow-leaks-the-bit timing side channel this function exists to
/// avoid (used by [`point_mul`] to choose the accumulator after a
/// conditional add).
fn fe_select(bit: u64, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let mask = 0u64.wrapping_sub(bit & 1);
    let mut r = [0u64; 4];
    for i in 0..4 {
        r[i] = (a[i] & mask) | (b[i] & !mask);
    }
    r
}

/// Field inversion using Fermat's little theorem: a^(-1) = a^(p-2) mod p.
fn fe_inv(a: [u64; 4]) -> [u64; 4] {
    // Use an addition chain for a^(p-2) mod p.
    // p - 2 = 0xFFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFD
    // We use a square-and-multiply approach with a known efficient chain.
    //
    // Addition chain (from "Guide to Elliptic Curve Cryptography"):
    // t1 = a^2, t2 = t1^2^2 = a^8, t3 = t2^2 = a^16, t4 = t3^2^2^2 = a^128
    // Then combine: a^(p-2) = a^(2^256 - 2^224 + 2^192 + 2^96 - 3)
    //
    // For simplicity, we use a standard square-and-multiply.
    let mut result = [1u64, 0, 0, 0];
    // p - 2 in binary has specific bit patterns; we iterate over all 256 bits.
    // p - 2 = 0xFFFFFFFF0000000100000000000000000000000000000000FFFFFFFFFFFFFFFD
    let exp: [u64; 4] = [
        0xFFFFFFFFFFFFFFFD,
        0x00000000FFFFFFFF,
        0x0000000000000000,
        0xFFFFFFFF00000001,
    ];

    // Start from the second-highest set bit (bit 254, since bit 255 is the highest).
    let mut started = false;
    for limb in exp.iter().rev() {
        for bit in (0..64).rev() {
            if started {
                result = fe_sqr(result);
            }
            if (limb >> bit) & 1 == 1 {
                started = true;
                result = fe_mul(result, a);
            }
        }
    }
    result
}

/// Check if two field elements are equal.
fn fe_eq(a: [u64; 4], b: [u64; 4]) -> bool {
    a == b
}

/// Check if a field element is zero.
fn fe_is_zero(a: [u64; 4]) -> bool {
    u256_is_zero(a)
}

// ---------------------------------------------------------------------------
// P-256 point operations (affine coordinates)
// ---------------------------------------------------------------------------

/// An affine point on P-256. The special "point at infinity" is represented
/// by `is_infinity = true`.
#[derive(Debug, Clone, Copy)]
struct AffinePoint {
    x: [u64; 4],
    y: [u64; 4],
    is_infinity: bool,
}

/// The identity (point at infinity).
fn point_identity() -> AffinePoint {
    AffinePoint { x: [0; 4], y: [0; 4], is_infinity: true }
}

/// Check if a point is on the P-256 curve: y² ≡ x³ − 3x + b (mod p).
fn point_on_curve(p: &AffinePoint) -> bool {
    if p.is_infinity {
        return true;
    }
    let y2 = fe_sqr(p.y);
    let x3 = fe_mul(fe_sqr(p.x), p.x);
    let three_x = fe_mul(p.x, [3, 0, 0, 0]);
    // x^3 - 3x + b
    let rhs = fe_add(fe_sub(x3, three_x), B);
    fe_eq(y2, rhs)
}

/// A point in Jacobian-free projective (homogeneous) coordinates:
/// affine `(x/z, y/z)`, with the identity (point at infinity) represented
/// as `z == 0` rather than a separate flag.
///
/// Representing infinity via `z == 0` — instead of [`AffinePoint`]'s
/// explicit `is_infinity` boolean — is exactly what lets [`proj_add`] and
/// [`proj_double`] below handle every input (identity, doubling, mutual
/// inverses) with the *same* fixed sequence of field operations and no
/// `if` on point structure at all.
#[derive(Debug, Clone, Copy)]
struct ProjectivePoint {
    x: [u64; 4],
    y: [u64; 4],
    z: [u64; 4],
}

/// Convert an affine point to projective form.
fn to_projective(p: AffinePoint) -> ProjectivePoint {
    if p.is_infinity {
        ProjectivePoint { x: [0, 0, 0, 0], y: [1, 0, 0, 0], z: [0, 0, 0, 0] }
    } else {
        ProjectivePoint { x: p.x, y: p.y, z: [1, 0, 0, 0] }
    }
}

/// Convert a projective point back to affine form.
///
/// This performs one field inversion and one branch on whether the *final*
/// result is the identity (`z == 0`). Unlike the per-scalar-bit branching
/// this module previously had, that single check depends only on the
/// public output of a completed operation (the same public quantity
/// `sign`/`verify` already branch on via `r_point.is_infinity`), not on
/// individual secret scalar bits — so it is not a side channel on `k` or
/// the private key.
fn to_affine(p: ProjectivePoint) -> AffinePoint {
    if fe_is_zero(p.z) {
        return point_identity();
    }
    let z_inv = fe_inv(p.z);
    AffinePoint { x: fe_mul(p.x, z_inv), y: fe_mul(p.y, z_inv), is_infinity: false }
}

/// Constant-time select between two projective points on a scalar bit.
fn proj_select(bit: u64, a: ProjectivePoint, b: ProjectivePoint) -> ProjectivePoint {
    ProjectivePoint {
        x: fe_select(bit, a.x, b.x),
        y: fe_select(bit, a.y, b.y),
        z: fe_select(bit, a.z, b.z),
    }
}

/// Complete point addition for P-256 (`a = -3`), in projective coordinates.
///
/// This is the Renes–Costello–Batina 2015 "complete addition formulas for
/// prime order elliptic curves" Algorithm 4, specialized for `a = -3` (as
/// P-256 is): <https://eprint.iacr.org/2015/1060>. Cross-checked line-for-line
/// against the `EquationAIsMinusThree::add_assign` implementation in
/// RustCrypto's `primeorder` crate (used by the widely-audited `p256`
/// crate). "Complete" means this single, fixed sequence of field
/// operations is correct for *every* input pair — both/either operand
/// being the identity, the operands being equal (doubling), and the
/// operands being mutual inverses (P + (-P)) — with no `if` anywhere in
/// this function distinguishing those cases. That structural branching
/// (which secret-dependent point relationships take which code path) was
/// the residual gap the previous affine `point_add` disclosed; this
/// closes it.
fn proj_add(p: ProjectivePoint, q: ProjectivePoint) -> ProjectivePoint {
    let xx = fe_mul(p.x, q.x);
    let yy = fe_mul(p.y, q.y);
    let zz = fe_mul(p.z, q.z);
    let xy_pairs = fe_sub(fe_mul(fe_add(p.x, p.y), fe_add(q.x, q.y)), fe_add(xx, yy));
    let yz_pairs = fe_sub(fe_mul(fe_add(p.y, p.z), fe_add(q.y, q.z)), fe_add(yy, zz));
    let xz_pairs = fe_sub(fe_mul(fe_add(p.x, p.z), fe_add(q.x, q.z)), fe_add(xx, zz));

    let bzz_part = fe_sub(xz_pairs, fe_mul(B, zz));
    let bzz3_part = fe_add(fe_double(bzz_part), bzz_part);
    let yy_m_bzz3 = fe_sub(yy, bzz3_part);
    let yy_p_bzz3 = fe_add(yy, bzz3_part);

    let zz3 = fe_add(fe_double(zz), zz);
    let bxz_part = fe_sub(fe_mul(B, xz_pairs), fe_add(zz3, xx));
    let bxz3_part = fe_add(fe_double(bxz_part), bxz_part);
    let xx3_m_zz3 = fe_sub(fe_add(fe_double(xx), xx), zz3);

    let x3 = fe_sub(fe_mul(yy_p_bzz3, xy_pairs), fe_mul(yz_pairs, bxz3_part));
    let y3 = fe_add(fe_mul(yy_p_bzz3, yy_m_bzz3), fe_mul(xx3_m_zz3, bxz3_part));
    let z3 = fe_add(fe_mul(yy_m_bzz3, yz_pairs), fe_mul(xy_pairs, xx3_m_zz3));

    ProjectivePoint { x: x3, y: y3, z: z3 }
}

/// Complete point doubling for P-256 (`a = -3`), in projective coordinates.
///
/// Renes–Costello–Batina 2015 Algorithm 6 (the exception-free doubling
/// formula specialized for `a = -3`), cross-checked against
/// `EquationAIsMinusThree::double_in_place` in RustCrypto's `primeorder`
/// crate. Like [`proj_add`], correct (and branch-free on point structure)
/// for every input, including the identity and points with `y == 0`.
fn proj_double(p: ProjectivePoint) -> ProjectivePoint {
    let xx = fe_sqr(p.x);
    let yy = fe_sqr(p.y);
    let zz = fe_sqr(p.z);
    let xy2 = fe_double(fe_mul(p.x, p.y));
    let xz2 = fe_double(fe_mul(p.x, p.z));

    let bzz_part = fe_sub(fe_mul(B, zz), xz2);
    let bzz3_part = fe_add(fe_double(bzz_part), bzz_part);
    let yy_m_bzz3 = fe_sub(yy, bzz3_part);
    let yy_p_bzz3 = fe_add(yy, bzz3_part);
    let y_frag = fe_mul(yy_p_bzz3, yy_m_bzz3);
    let x_frag = fe_mul(yy_m_bzz3, xy2);

    let zz3 = fe_add(fe_double(zz), zz);
    let bxz2_part = fe_sub(fe_mul(B, xz2), fe_add(zz3, xx));
    let bxz6_part = fe_add(fe_double(bxz2_part), bxz2_part);
    let xx3_m_zz3 = fe_sub(fe_add(fe_double(xx), xx), zz3);

    let y3 = fe_add(y_frag, fe_mul(xx3_m_zz3, bxz6_part));
    let yz2 = fe_double(fe_mul(p.y, p.z));
    let x3 = fe_sub(x_frag, fe_mul(bxz6_part, yz2));
    let z3 = fe_double(fe_double(fe_mul(yz2, yy)));

    ProjectivePoint { x: x3, y: y3, z: z3 }
}

/// Point addition, complete over every affine input (identity, doubling,
/// mutual inverses) via [`proj_add`] — see that function's doc for the
/// formula's provenance and the structural-branching gap it closes.
fn point_add(p: AffinePoint, q: AffinePoint) -> AffinePoint {
    to_affine(proj_add(to_projective(p), to_projective(q)))
}

/// Point doubling, complete over every affine input via [`proj_double`].
///
/// `point_mul` calls `proj_double` directly rather than through this affine
/// wrapper (to avoid a to-affine/to-projective round trip per loop
/// iteration), so this function is currently exercised only by the test
/// module's cross-checks (e.g. `generator_order`, which verifies `point_mul`
/// against an independent doubling/addition chain) — hence `cfg(test)`.
#[cfg(test)]
fn point_double(p: AffinePoint) -> AffinePoint {
    to_affine(proj_double(to_projective(p)))
}

/// Scalar multiplication k * P using double-and-add.
///
/// Always performs exactly 256 double+add+select iterations regardless of
/// the scalar's bit length (every bit position is visited, including
/// leading zero bits). Both the point arithmetic (`proj_add`/`proj_double`,
/// the Renes–Costello–Batina complete formulas) and the accumulator choice
/// (`proj_select`, an arithmetic bitmask rather than an `if`) are free of
/// branches on secret data — neither the scalar's bit pattern nor the
/// intermediate points' structure (identity/doubling/inverse) influences
/// control flow. The single remaining branch is in `to_affine`, executed
/// once after the loop on the public final result, not per-bit — see that
/// function's doc.
///
/// This closes the previously-disclosed gap where `point_add`/`point_double`
/// branched on point structure. The underlying field arithmetic
/// (`fe_add`/`fe_sub`'s conditional-subtract modular reduction, and
/// `fe_inv`/`u256_cmp`) still uses value-dependent branches and is *not*
/// constant-time at that lower layer — a separate, deeper class of timing
/// leak common to schoolbook big-integer arithmetic, not addressed by this
/// fix. See the crate-level "Constant-time status" section.
fn point_mul(k: [u64; 4], p: AffinePoint) -> AffinePoint {
    let base = to_projective(p);
    let mut result = to_projective(point_identity());
    for limb in k.iter().rev() {
        for bit in (0..64).rev() {
            result = proj_double(result);
            let sum = proj_add(result, base);
            result = proj_select((limb >> bit) & 1, sum, result);
        }
    }
    to_affine(result)
}

/// The generator point G.
fn generator() -> AffinePoint {
    AffinePoint { x: GX, y: GY, is_infinity: false }
}

// ---------------------------------------------------------------------------
// Modular arithmetic mod n (curve order)
// ---------------------------------------------------------------------------

/// Add two numbers mod n.
fn n_add(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let (sum, carry) = u256_add(a, b);
    if carry != 0 || u256_cmp(sum, N) != core::cmp::Ordering::Less {
        let (diff, _) = u256_sub(sum, N);
        diff
    } else {
        sum
    }
}

/// Subtract two numbers mod n.
fn n_sub(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let (diff, borrow) = u256_sub(a, b);
    if borrow != 0 {
        let (sum, _) = u256_add(diff, N);
        sum
    } else {
        diff
    }
}

/// Multiply two numbers mod n.
fn n_mul(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    // Reduction factor: C_N = 2^256 mod n = 2^256 - n (since n < 2^256)
    const C_N: [u64; 4] = [
        0x0C46353D039CDAAF,
        0x4319055258E8617B,
        0x0000000000000000,
        0x00000000FFFFFFFF,
    ];

    let product = mul_wide(a, b);
    let mut lo = [product[0], product[1], product[2], product[3]];
    let mut hi = [product[4], product[5], product[6], product[7]];

    for _ in 0..12 {
        if hi == [0, 0, 0, 0] {
            break;
        }
        let p = mul_wide(hi, C_N);
        let (new_lo, carry) = u256_add(lo, [p[0], p[1], p[2], p[3]]);
        let mut new_hi = [p[4], p[5], p[6], p[7]];
        let mut c = carry as u128;
        for i in 0..4 {
            let s = (new_hi[i] as u128) + c;
            new_hi[i] = s as u64;
            c = s >> 64;
        }
        lo = new_lo;
        hi = new_hi;
    }

    // Final conditional subtraction (result < 2*n at this point)
    if u256_cmp(lo, N) != core::cmp::Ordering::Less {
        let (diff, _) = u256_sub(lo, N);
        diff
    } else {
        lo
    }
}

/// Invert mod n using Fermat's little theorem: a^(n-2) mod n.
fn n_inv(a: [u64; 4]) -> [u64; 4] {
    let exp = {
        // n - 2
        let (e, _) = u256_sub(N, [2, 0, 0, 0]);
        e
    };
    let mut result = [1u64, 0, 0, 0];
    let mut started = false;
    for limb in exp.iter().rev() {
        for bit in (0..64).rev() {
            if started {
                result = n_mul(result, result);
            }
            if (limb >> bit) & 1 == 1 {
                started = true;
                result = n_mul(result, a);
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Convert bytes ↔ field elements
// ---------------------------------------------------------------------------

/// Convert a big-endian32-byte slice to a field element.
fn bytes_to_fe(bytes: &[u8; 32]) -> [u64; 4] {
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        let offset = (3 - i) * 8;
        limbs[i] = u64::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
    }
    // limbs[3] is MSW, limbs[0] is LSW
    [limbs[0], limbs[1], limbs[2], limbs[3]]
}

/// Convert a field element to a big-endian32-byte array.
fn fe_to_bytes(fe: [u64; 4]) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for i in 0..4 {
        let be = fe[i].to_be_bytes();
        let offset = (3 - i) * 8;
        bytes[offset..offset + 8].copy_from_slice(&be);
    }
    bytes
}

/// Convert bytes to a scalar mod n.
#[allow(dead_code)]
fn bytes_to_n(bytes: &[u8; 32]) -> [u64; 4] {
    bytes_to_fe(bytes) // Same layout; reduction happens in n_mul
}

/// Reduce a message hash to a scalar mod n, per FIPS 186-4.
///
/// A 32-byte hash is interpreted as a 256-bit big-endian integer, which can
/// be `>= n` (n is a few bits short of 2^256). A single conditional
/// subtraction suffices since the hash is always `< 2^256 < 2*n`.
fn hash_to_scalar(hash: &[u8; 32]) -> [u64; 4] {
    let raw = bytes_to_fe(hash);
    if u256_cmp(raw, N) != core::cmp::Ordering::Less {
        let (reduced, _) = u256_sub(raw, N);
        reduced
    } else {
        raw
    }
}

// ---------------------------------------------------------------------------
// ECDSA
// ---------------------------------------------------------------------------

/// A P-256 ECDSA public key (uncompressed point).
#[derive(Debug, Clone, Copy)]
pub struct PublicKey {
    point: AffinePoint,
}

/// A P-256 ECDSA private key (scalar).
#[derive(Debug, Clone, Copy)]
pub struct PrivateKey {
    scalar: [u64; 4],
}

/// A P-256 ECDSA signature (r, s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature {
    r: [u64; 4],
    s: [u64; 4],
}

/// P-256 ECDSA engine (mock, not constant-time).
#[derive(Debug, Clone, Copy)]
pub struct P256Ecc;

/// Deterministic mock PRNG for nonce generation (NOT cryptographically secure).
///
/// For a real implementation, use a CSPRNG. This exists only so the mock
/// `sign` function has a deterministic nonce for testing.
struct MockRng {
    state: u64,
}

impl MockRng {
    fn next_u64(&mut self) -> u64 {
        // xorshift64 — NOT cryptographically secure, mock only.
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

impl Ecc for P256Ecc {
    type PublicKey = PublicKey;
    type PrivateKey = PrivateKey;
    type Signature = Signature;

    fn keygen(&self, seed: &[u8; 32]) -> (PublicKey, PrivateKey) {
        // Map the caller-supplied entropy onto a scalar in [1, n-1] via a
        // single reduction mod n (see `Ecc::keygen`'s doc: `seed` must come
        // from a CSPRNG, so reducing to exactly zero has probability
        // ~2^-224 and the fallback below is effectively unreachable).
        let raw = bytes_to_fe(seed);
        let reduced = n_mul(raw, [1, 0, 0, 0]);
        let scalar = if u256_is_zero(reduced) { [1, 0, 0, 0] } else { reduced };
        let point = point_mul(scalar, generator());
        (PublicKey { point }, PrivateKey { scalar })
    }

    fn sign(&self, hash: &[u8; 32], private_key: &PrivateKey) -> Signature {
        let z = hash_to_scalar(hash);
        let mut rng = MockRng { state: u64::from_le_bytes(hash[..8].try_into().unwrap()) };

        let (r, s) = loop {
            // Generate ephemeral nonce k (deterministic mock for testing).
            let k = [rng.next_u64(), rng.next_u64(), rng.next_u64(), rng.next_u64()];
            // Ensure k is in [1, n-1]
            let k_mod = {
                let k_n = n_mul(k, [1, 0, 0, 0]);
                if u256_is_zero(k_n) || u256_cmp(k_n, N) == core::cmp::Ordering::Equal {
                    continue;
                }
                k_n
            };

            // R = k * G
            let r_point = point_mul(k_mod, generator());
            if r_point.is_infinity {
                continue;
            }

            // r = R.x mod n
            let r = {
                let rx = r_point.x;
                if u256_cmp(rx, N) != core::cmp::Ordering::Less {
                    let (d, _) = u256_sub(rx, N);
                    d
                } else {
                    rx
                }
            };
            if u256_is_zero(r) {
                continue;
            }

            // s = k^(-1) * (z + r * d) mod n
            let k_inv = n_inv(k_mod);
            let rd = n_mul(r, private_key.scalar);
            let z_plus_rd = n_add(z, rd);
            let mut s = n_mul(k_inv, z_plus_rd);
            if u256_is_zero(s) {
                continue;
            }

            // Canonicalize to low-s form: (r, s) and (r, n-s) both verify
            // for the same message/key (ECDSA malleability), so always
            // emit the smaller of the two to make signatures unique.
            if u256_cmp(s, u256_shr1(N)) == core::cmp::Ordering::Greater {
                s = n_sub(N, s);
            }

            break (r, s);
        };

        Signature { r, s }
    }

    fn verify(&self, hash: &[u8; 32], signature: &Signature, public_key: &PublicKey) -> bool {
        // Reject zero components.
        if u256_is_zero(signature.r) || u256_is_zero(signature.s) {
            return false;
        }
        // Reject if r or s >= n.
        if u256_cmp(signature.r, N) != core::cmp::Ordering::Less
            || u256_cmp(signature.s, N) != core::cmp::Ordering::Less
        {
            return false;
        }
        // Reject non-canonical (high-s) signatures — see the low-s
        // canonicalization comment in `sign`.
        if u256_cmp(signature.s, u256_shr1(N)) == core::cmp::Ordering::Greater {
            return false;
        }
        // Reject invalid public keys: the identity point, or a point that
        // does not actually lie on the curve (an "invalid curve" attack
        // surface for keys built from untrusted bytes via `from_xy`).
        if public_key.point.is_infinity || !point_on_curve(&public_key.point) {
            return false;
        }

        let z = hash_to_scalar(hash);
        let s_inv = n_inv(signature.s);
        let u1 = n_mul(z, s_inv);
        let u2 = n_mul(signature.r, s_inv);

        // P = u1*G + u2*Q
        let p1 = point_mul(u1, generator());
        let p2 = point_mul(u2, public_key.point);
        let point = point_add(p1, p2);

        if point.is_infinity {
            return false;
        }

        // Verify: point.x mod n == r
        let vx = {
            if u256_cmp(point.x, N) != core::cmp::Ordering::Less {
                let (d, _) = u256_sub(point.x, N);
                d
            } else {
                point.x
            }
        };

        fe_eq(vx, signature.r)
    }
}

impl PublicKey {
    /// Create a public key from a 32-byte x and y coordinate pair.
    ///
    /// Returns `None` if the point does not lie on the P-256 curve — an
    /// "invalid curve" attack surface if constructed from untrusted bytes
    /// without validation.
    pub fn from_xy(x: &[u8; 32], y: &[u8; 32]) -> Option<Self> {
        let point = AffinePoint {
            x: bytes_to_fe(x),
            y: bytes_to_fe(y),
            is_infinity: false,
        };
        if point_on_curve(&point) {
            Some(Self { point })
        } else {
            None
        }
    }

    /// Get the x and y coordinates as32-byte arrays.
    pub fn to_xy(&self) -> ([u8; 32], [u8; 32]) {
        (fe_to_bytes(self.point.x), fe_to_bytes(self.point.y))
    }
}

impl PrivateKey {
    /// Create a private key from a32-byte scalar.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self { scalar: bytes_to_fe(bytes) }
    }
}

impl Signature {
    /// Create a signature from r and s components.
    pub fn from_rs(r: &[u8; 32], s: &[u8; 32]) -> Self {
        Self { r: bytes_to_fe(r), s: bytes_to_fe(s) }
    }

    /// Get r and s as32-byte arrays.
    pub fn to_rs(&self) -> ([u8; 32], [u8; 32]) {
        (fe_to_bytes(self.r), fe_to_bytes(self.s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the generator point is on the curve.
    #[test]
    fn generator_on_curve() {
        let g = generator();
        assert!(point_on_curve(&g));
    }

    /// Verify that G * n = O (order of the generator).
    #[test]
    fn generator_order() {
        let g = generator();
        // Verify small multiples first
        let g2_pm = point_mul([2, 0, 0, 0], g);
        let g2_d = point_double(g);
        assert!(fe_eq(g2_pm.x, g2_d.x) && fe_eq(g2_pm.y, g2_d.y), "2G mismatch");
        
        let g3_pm = point_mul([3, 0, 0, 0], g);
        let g3_a = point_add(g, g2_d);
        assert!(fe_eq(g3_pm.x, g3_a.x) && fe_eq(g3_pm.y, g3_a.y), "3G mismatch");
        
        let g4_pm = point_mul([4, 0, 0, 0], g);
        let g4_d = point_double(g2_d);
        assert!(fe_eq(g4_pm.x, g4_d.x) && fe_eq(g4_pm.y, g4_d.y), "4G mismatch");

        let result = point_mul(N, g);
        if !result.is_infinity {
            panic!("n*G not inf: x=[{:016x},{:016x},{:016x},{:016x}] y=[{:016x},{:016x},{:016x},{:016x}]",
                result.x[0], result.x[1], result.x[2], result.x[3],
                result.y[0], result.y[1], result.y[2], result.y[3]);
        }
    }

    /// Basic keygen/verify round-trip.
    #[test]
    fn ecdsa_sign_verify_round_trip() {
        let ecc = P256Ecc;
        let seed = [1u8; 32];
        let (pk, sk) = ecc.keygen(&seed);
        let hash = [0xABu8; 32];
        let sig = ecc.sign(&hash, &sk);
        assert!(ecc.verify(&hash, &sig, &pk), "valid signature must verify");
    }

    /// Signature must not verify under a different hash.
    #[test]
    fn ecdsa_wrong_hash_fails() {
        let ecc = P256Ecc;
        let seed = [2u8; 32];
        let (pk, sk) = ecc.keygen(&seed);
        let hash1 = [0xABu8; 32];
        let hash2 = [0xCDu8; 32];
        let sig = ecc.sign(&hash1, &sk);
        assert!(!ecc.verify(&hash2, &sig, &pk), "wrong hash must not verify");
    }

    /// Signature must not verify under a different public key.
    #[test]
    fn ecdsa_wrong_key_fails() {
        let ecc = P256Ecc;
        let seed1 = [3u8; 32];
        let seed2 = [4u8; 32];
        let (_, sk) = ecc.keygen(&seed1);
        let (pk2, _) = ecc.keygen(&seed2);
        let hash = [0xABu8; 32];
        let sig = ecc.sign(&hash, &sk);
        assert!(!ecc.verify(&hash, &sig, &pk2), "wrong key must not verify");
    }

    /// Cross-check against an independent P-256 implementation (.NET's
    /// `System.Security.Cryptography.ECDsa`, CNG-backed), not this crate's
    /// own `sign()`/`verify()` round-tripping against itself.
    ///
    /// The root `todo.md` flagged this as a real gap: every other ECDSA
    /// test here only proves internal self-consistency (a sign-convention
    /// swap or similar systematic algebra error would round-trip fine and
    /// go undetected). No NIST/CAVP vector file was hand-transcribed here
    /// (too easy to introduce a wrong constant that fails or silently
    /// passes for the wrong reason) -- instead these constants were
    /// generated fresh via a live, independent, trusted crypto stack and
    /// copied verbatim from its output. Regenerate with (PowerShell):
    ///
    /// ```powershell
    /// Add-Type -AssemblyName System.Security
    /// $ecdsa = [System.Security.Cryptography.ECDsa]::Create([System.Security.Cryptography.ECCurve+NamedCurves]::nistP256)
    /// $params = $ecdsa.ExportParameters($true)
    /// $hash = 1..32 | ForEach-Object { [byte]$_ }
    /// $sig = $ecdsa.SignHash($hash)
    /// # $params.D / $params.Q.X / $params.Q.Y / $hash / $sig[0..31] / $sig[32..63]
    /// ```
    ///
    /// Two independent properties are checked against the *same* generated
    /// key/signature:
    /// 1. This crate's scalar multiplication (`keygen`, which for a
    ///    seed already `< n` reduces to a no-op -- see `keygen`'s doc)
    ///    reproduces the exact public-key point .NET derived from the same
    ///    private scalar `D`.
    /// 2. This crate's `verify()` accepts a signature `.NET` produced with
    ///    its own (different, opaque) nonce -- verification has no nonce
    ///    dependency, so this exercises the full point-add/point-mul/
    ///    modular-inverse/`hash_to_scalar` verify path against real,
    ///    externally-produced values.
    #[test]
    fn ecdsa_cross_check_against_dotnet_cng() {
        let ecc = P256Ecc;

        // Private scalar D, from .NET's ECParameters.D.
        let d: [u8; 32] = hex32(ECDSA_KAT_D);
        // Public key (Qx, Qy), from .NET's ECParameters.Q.
        let qx: [u8; 32] = hex32(ECDSA_KAT_QX);
        let qy: [u8; 32] = hex32(ECDSA_KAT_QY);
        // The 32-byte value that was signed (SignHash signs its input
        // directly, no internal hashing) -- bytes 0x01..=0x20.
        let hash: [u8; 32] = hex32(ECDSA_KAT_HASH);
        // Signature (r, s) .NET produced (IEEE P1363 r||s), from SignHash.
        let r: [u8; 32] = hex32(ECDSA_KAT_R);
        let s: [u8; 32] = hex32(ECDSA_KAT_S);

        // Property 1: our scalar multiplication reproduces .NET's public key.
        let (pk, _sk) = ecc.keygen(&d);
        let expected_x = bytes_to_fe(&qx);
        let expected_y = bytes_to_fe(&qy);
        assert!(
            fe_eq(pk.point.x, expected_x) && fe_eq(pk.point.y, expected_y),
            "point_mul(D, G) must reproduce .NET's independently-derived public key"
        );

        // Property 2: our verify() accepts .NET's independently-produced signature.
        let external_pk = PublicKey::from_xy(&qx, &qy).expect("valid on-curve point");
        let external_sig = Signature::from_rs(&r, &s);
        assert!(
            ecc.verify(&hash, &external_sig, &external_pk),
            "verify() must accept a signature produced by an independent P-256 implementation"
        );
    }

    // KAT constants for `ecdsa_cross_check_against_dotnet_cng`, kept as
    // named consts (rather than inline literals) so their exact lengths are
    // trivially greppable/diffable -- these were mistranscribed by hand
    // (a dropped trailing hex digit) multiple times while authoring this
    // test, caught only by `hex32`'s own length assertion. `const` items
    // don't help catch that class of error any more than a `let` would,
    // but keeping them separate from the test body makes the block below
    // easy to regenerate wholesale from the PowerShell snippet in the doc
    // comment above without touching the test logic.
    const ECDSA_KAT_D: &str = "aecdfca13c6f1abf6657cf7ed8092a8aa4fda60c3928082b6b04d340b18f0d0e";
    const ECDSA_KAT_QX: &str = "996e6d550521c83f1047beb79e20d2cb7475f9e57e794eb5c3bdcce495c47127";
    const ECDSA_KAT_QY: &str = "176c54c635269588016341cd182b27307cf2784d3c51cc353b5a394ccf2ebe85";
    const ECDSA_KAT_HASH: &str = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    const ECDSA_KAT_R: &str = "379bb06c124b10b5a6f0863c0c5035dc0324bb56d3fda2d6d14085e93adc0d35";
    const ECDSA_KAT_S: &str = "543b9b903ad48aa9d9280384b3165eb8b929543e985af58acae17887127f1150";

    /// Parses a 64-hex-char string into a 32-byte array. Panics on
    /// malformed input -- test-only helper, not a general hex parser.
    fn hex32(s: &str) -> [u8; 32] {
        assert_eq!(s.len(), 64, "expected 64 hex chars (32 bytes)");
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("valid hex byte");
        }
        out
    }

    /// Deterministic signing produces identical signatures for same input.
    #[test]
    fn ecdsa_deterministic() {
        let ecc = P256Ecc;
        let seed = [5u8; 32];
        let (_, sk) = ecc.keygen(&seed);
        let hash = [0x42u8; 32];
        let sig1 = ecc.sign(&hash, &sk);
        let sig2 = ecc.sign(&hash, &sk);
        assert_eq!(sig1, sig2, "deterministic signing must produce identical signatures");
    }

    /// Verify that point operations preserve curve membership.
    #[test]
    fn point_operations_on_curve() {
        let g = generator();
        let g2 = point_double(g);
        assert!(point_on_curve(&g2));
        let g3 = point_add(g, g2);
        assert!(point_on_curve(&g3));
    }

    /// The complete (Renes-Costello-Batina) formulas replaced code that
    /// special-cased the identity, doubling, and mutual-inverse inputs with
    /// explicit `if`s. This proves the unified, branch-free replacement is
    /// still correct on exactly those cases -- the ones a structural branch
    /// used to distinguish.
    #[test]
    fn point_add_handles_every_special_case() {
        let g = generator();
        let o = point_identity();
        let neg_g = AffinePoint { x: g.x, y: fe_sub([0, 0, 0, 0], g.y), is_infinity: false };
        assert!(point_on_curve(&neg_g), "-G must be on curve");

        let sum = point_add(o, g);
        assert!(!sum.is_infinity && fe_eq(sum.x, g.x) && fe_eq(sum.y, g.y), "O + G must equal G");
        let sum = point_add(g, o);
        assert!(!sum.is_infinity && fe_eq(sum.x, g.x) && fe_eq(sum.y, g.y), "G + O must equal G");

        let doubled = point_double(g);
        let added = point_add(g, g);
        assert!(
            !added.is_infinity && fe_eq(doubled.x, added.x) && fe_eq(doubled.y, added.y),
            "G + G must equal double(G)"
        );
        assert!(point_on_curve(&doubled));

        assert!(point_add(g, neg_g).is_infinity, "G + (-G) must be the identity");
        assert!(point_add(o, o).is_infinity, "O + O must be the identity");
        assert!(point_double(o).is_infinity, "double(O) must be the identity");
    }

    /// Verify field element inversion: a * a^(-1) == 1.
    #[test]
    fn fe_inv_round_trip() {
        let a = [7, 0, 0, 0];
        let a_inv = fe_inv(a);
        let product = fe_mul(a, a_inv);
        assert_eq!(product, [1, 0, 0, 0]);
    }

    /// Basic field arithmetic checks.
    #[test]
    fn fe_arithmetic_basics() {
        // fe_mul(2, 3) = 6
        assert_eq!(fe_mul([2, 0, 0, 0], [3, 0, 0, 0]), [6, 0, 0, 0]);
        // fe_mul(0, x) = 0
        assert_eq!(fe_mul([0, 0, 0, 0], [42, 0, 0, 0]), [0, 0, 0, 0]);
        // fe_mul(1, x) = x
        assert_eq!(fe_mul([1, 0, 0, 0], [42, 0, 0, 0]), [42, 0, 0, 0]);
        // fe_sqr(1) = 1
        assert_eq!(fe_sqr([1, 0, 0, 0]), [1, 0, 0, 0]);
        // fe_mul(x, 1) = x for large x
        assert_eq!(fe_mul(GX, [1, 0, 0, 0]), GX);
        // fe_mul(x, 2) = x+x for large x (verify no reduction issue)
        let gx2 = fe_mul(GX, [2, 0, 0, 0]);
        let gx_add_gx = fe_add(GX, GX);
        assert_eq!(gx2, gx_add_gx);
        // fe_sqr(x) = fe_mul(x, x)
        let gx_sqr = fe_sqr(GX);
        let gx_mul = fe_mul(GX, GX);
        assert_eq!(gx_sqr, gx_mul);
    }

    /// Diagnostic: check p256_redc with a known case.
    #[test]
    fn fe_mul_known_products() {
        // p-1 = P - 1
        let p_minus_1: [u64; 4] = [
            0xFFFFFFFFFFFFFFFE,
            0x00000000FFFFFFFF,
            0x0000000000000000,
            0xFFFFFFFF00000001,
        ];
        // p-2 = P - 2
        let p_minus_2: [u64; 4] = [
            0xFFFFFFFFFFFFFFFD,
            0x00000000FFFFFFFF,
            0x0000000000000000,
            0xFFFFFFFF00000001,
        ];
        // (p-1) * 1 = p-1
        assert_eq!(fe_mul(p_minus_1, [1, 0, 0, 0]), p_minus_1);
        // (p-1) * 2 = p - 2 (mod p)
        assert_eq!(fe_mul(p_minus_1, [2, 0, 0, 0]), p_minus_2);
        // x * x_inv = 1 for a non-trivial element
        let a = GX;
        let a_inv = fe_inv(a);
        assert_eq!(fe_mul(a, a_inv), [1, 0, 0, 0]);
    }

    /// Direct mul_wide correctness check.
    #[test]
    fn mul_wide_known() {
        let a: [u64; 4] = [0xFFFFFFFFFFFFFFFF, 0, 0, 0];
        let b: [u64; 4] = [0xFFFFFFFFFFFFFFFF, 0, 0, 0];
        // (2^64-1)^2 = 2^128 - 2^65 + 1
        let expected: [u64; 8] = [1, 0xFFFFFFFFFFFFFFFE, 0, 0, 0, 0, 0, 0];
        assert_eq!(mul_wide(a, b), expected, "(2^64-1)^2");

        // 2^128 * 2^128 = 2^256
        let c: [u64; 4] = [0, 0, 1, 0];
        let d: [u64; 4] = [0, 0, 1, 0];
        let expected2: [u64; 8] = [0, 0, 0, 0, 1, 0, 0, 0];
        assert_eq!(mul_wide(c, d), expected2, "2^128 * 2^128 = 2^256");
    }
    #[test]
    fn n_mul_basic() {
        // 1 * 1 = 1
        assert_eq!(n_mul([1, 0, 0, 0], [1, 0, 0, 0]), [1, 0, 0, 0]);
        // N * 1 = 0 (mod N)
        assert_eq!(n_mul(N, [1, 0, 0, 0]), [0, 0, 0, 0]);
        // (N-1) * 2 = N - 2 (mod N)
        let n_minus_1 = [N[0] - 1, N[1], N[2], N[3]];
        let expected = [N[0] - 2, N[1], N[2], N[3]];
        assert_eq!(n_mul(n_minus_1, [2, 0, 0, 0]), expected, "(N-1)*2 mod N = N-2");
        // a * a_inv = 1
        let a: [u64; 4] = [0x12345678, 0x9ABCDEF0, 0xFEDCBA98, 0x76543210];
        let a_inv = n_inv(a);
        let prod = n_mul(a, a_inv);
        assert_eq!(prod, [1, 0, 0, 0], "n_mul(a, n_inv(a)) must be 1");
    }

    #[test]
    fn n_mul_trace() {
        let a = [N[0] - 1, N[1], N[2], N[3]]; // N-1
        let b = [2u64, 0, 0, 0];
        let product = mul_wide(a, b);
        let lo = [product[0], product[1], product[2], product[3]];
        let hi = [product[4], product[5], product[6], product[7]];
        
        let n_minus_2 = [N[0] - 2, N[1], N[2], N[3]];
        
        // Check mul_wide result
        let expected_product = {
            // 2*(N-1) = 2N-2 = 2^256 + (2N-2-2^256)
            // Since N = 2^256 - C_N, 2N = 2^257 - 2*C_N
            // 2N-2 = 2^257 - 2*C_N - 2
            // So hi=2, lo = -2*C_N - 2 mod 2^256? No...
            // Actually just compute 2*(N-1) limb by limb with carry
            let (s0, c0) = {
                let v = (a[0] as u128) * 2;
                (v as u64, (v >> 64) as u64)
            };
            let (s1, c1) = {
                let v = (a[1] as u128) * 2 + (c0 as u128);
                (v as u64, (v >> 64) as u64)
            };
            let (s2, c2) = {
                let v = (a[2] as u128) * 2 + (c1 as u128);
                (v as u64, (v >> 64) as u64)
            };
            let (s3, c3) = {
                let v = (a[3] as u128) * 2 + (c2 as u128);
                (v as u64, (v >> 64) as u64)
            };
            [s0, s1, s2, s3, c3, 0, 0, 0]
        };
        
        assert_eq!(product, expected_product, "mul_wide(N-1, 2) must be correct");
        assert_eq!(lo, [expected_product[0], expected_product[1], expected_product[2], expected_product[3]]);
        assert_eq!(hi, [1, 0, 0, 0], "hi must be 1");
        
        // Manually compute expected result: N-2
        let expected_lo = [N[0] - 2, N[1], N[2], N[3]];
        
        // Manually compute what the reduction should give
        // lo + hi*C_N = lo + C_N
        let (sum_lo, sum_carry) = u256_add(lo, C_N);
        let sum_hi_raw = sum_carry as u64;
        
        assert_eq!(sum_lo, expected_lo, "lo + C_N must equal N-2");
        assert_eq!(sum_hi_raw, 0, "no carry expected");
        
        // Now run n_mul and check
        let result = n_mul(a, b);
        assert_eq!(result, n_minus_2, "n_mul(N-1, 2) must be N-2");
    }

    // Expose C_N for testing
    const C_N: [u64; 4] = [
        0x0C46353D039CDAAF,
        0x4319055258E8617B,
        0x0000000000000000,
        0x00000000FFFFFFFF,
    ];

    #[test]
    fn ecdsa_sign_verify_debug() {
        let g = generator();
        let scalar = [0xDEADBEEF_CAFEBABE, 0x12345678_9ABCDEF0, 0xFEEDFACE_DEADBEEF, 0x01234567_89ABCDEF];
        let pk = point_mul(scalar, g);
        assert!(point_on_curve(&pk), "public key must be on curve");

        let hash = [0xABu8; 32];
        let z = bytes_to_fe(&hash);

        let mut rng = MockRng { state: u64::from_le_bytes(hash[..8].try_into().unwrap()) };
        let k = [rng.next_u64(), rng.next_u64(), rng.next_u64(), rng.next_u64()];
        let k_mod = n_mul(k, [1, 0, 0, 0]);
        let k_inv = n_inv(k_mod);

        // Verify k * k_inv = 1 mod n
        let kk = n_mul(k_mod, k_inv);
        assert_eq!(kk, [1, 0, 0, 0], "k * k_inv must be 1 mod n");

        let r_point = point_mul(k_mod, g);
        assert!(!r_point.is_infinity, "kG must not be infinity");
        assert!(point_on_curve(&r_point), "kG must be on curve");

        let rx = r_point.x;
        let r = if u256_cmp(rx, N) != core::cmp::Ordering::Less {
            let (d, _) = u256_sub(rx, N);
            d
        } else {
            rx
        };

        let rd = n_mul(r, scalar);
        let z_plus_rd = n_add(z, rd);
        let s = n_mul(k_inv, z_plus_rd);

        // Manual verify: u1 = z*s_inv, u2 = r*s_inv
        let s_inv = n_inv(s);
        let u1 = n_mul(z, s_inv);
        let u2 = n_mul(r, s_inv);

        // Verify u1*G + u2*Q = R
        let p1 = point_mul(u1, g);
        let p2 = point_mul(u2, pk);
        let point = point_add(p1, p2);
        assert!(!point.is_infinity, "u1*G + u2*Q must not be infinity");
        let vx = if u256_cmp(point.x, N) != core::cmp::Ordering::Less {
            let (d, _) = u256_sub(point.x, N);
            d
        } else {
            point.x
        };
        assert!(fe_eq(vx, r), "verification equation must hold: vx == r");
    }

    #[test]
    fn fe_mul_diagnostics() {
        // P * 1 = 0 (mod p)
        assert_eq!(fe_mul(P, [1, 0, 0, 0]), [0, 0, 0, 0], "P * 1 mod p must be 0");
        // fe_inv(1) must be 1
        assert_eq!(fe_inv([1, 0, 0, 0]), [1, 0, 0, 0], "inv(1) must be 1");
        // 2^128 * 2^128 = 2^256 ≡ C mod p
        let c_expected: [u64; 4] = [1, 0xFFFF_FFFF_0000_0000, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFE];
        assert_eq!(fe_mul([0, 0, 1, 0], [0, 0, 1, 0]), c_expected, "2^128*2^128 mod p must be C");
        // 2^192 * 2^192 = 2^384 ≡ C * 2^128 mod p, then reduce once more
        let expected_384: [u64; 4] = [0xFFFF_FFFE_FFFF_FFFE, 0x0000_0002_FFFF_FFFF, 0x0000_0000_0000_0002, 0xFFFF_FFFE_0000_0001];
        assert_eq!(fe_mul([0, 0, 0, 1], [0, 0, 0, 1]), expected_384, "2^192*2^192 mod p");
    }
}
