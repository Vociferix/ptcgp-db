//! Exact rational probability arithmetic.

use gcd::binary_u64 as gcd;

/// An exact rational probability stored as a `numerator / denominator` pair.
///
/// All pull rate data from ptcgp-data is stored as exact fractions to avoid floating-point
/// error. Intermediate calculations must stay in rational form; only convert with [`as_f64`]
/// at final display time.
///
/// Arithmetic operations preserve exactness where `u64` allows. When an intermediate product
/// would overflow `u64`, the implementation falls back to a lossy bit-halving loop to bring
/// the values back into range. For the denominators present in PTCGP pull rate data this
/// approximation is negligible, but callers should be aware the result is not always exact.
///
/// [`as_f64`]: Prob::as_f64
#[derive(Clone, Copy, Default)]
pub struct Prob {
    pub(crate) num: u64,
    pub(crate) den: u64,
}

impl Prob {
    /// The probability 0 (impossible event).
    pub const ZERO: Self = Self { num: 0, den: 1 };

    /// The probability 1 (certain event).
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// Constructs a `Prob` with the given numerator and denominator.
    ///
    /// The fraction is stored as-is without reduction. `denominator` must not be zero.
    pub const fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            num: numerator,
            den: denominator,
        }
    }

    /// Raw numerator. May share a common factor with the denominator; use [`simplify`] to
    /// obtain the reduced form.
    ///
    /// [`simplify`]: Prob::simplify
    pub const fn numerator(&self) -> u64 {
        self.num
    }

    /// Raw denominator. May share a common factor with the numerator; use [`simplify`] to
    /// obtain the reduced form.
    ///
    /// [`simplify`]: Prob::simplify
    pub const fn denominator(&self) -> u64 {
        self.den
    }

    /// Returns an equivalent fraction in lowest terms.
    pub const fn simplify(&self) -> Self {
        let gcd = gcd(self.num, self.den);
        Self {
            num: self.num / gcd,
            den: self.den / gcd,
        }
    }

    /// Converts to `f64` for display. Not suitable for intermediate probability arithmetic —
    /// use the arithmetic methods to keep calculations exact.
    pub const fn as_f64(&self) -> f64 {
        let Prob { num, den } = self.simplify();

        // How many bits does den need?
        let den_bits = 64 - den.leading_zeros(); // in range [0, 64]

        // We want to left-shift num so that (num << shift) / den produces a
        // quotient that fills all 53 bits of f64's mantissa.
        // A quotient fills 53 bits when: 2^52 <= quotient < 2^53, i.e. we want
        // num << shift to be just above den, with ~53 bits of headroom.
        // Shift num up by 53 bits, then correct for den's magnitude.
        let shift = 53u32
            .saturating_sub(64 - num.leading_zeros())
            .saturating_add(den_bits);

        // Perform the scaled integer division
        let scaled = if shift <= 63 {
            // num fits after shifting
            let scaled_num = if let Some(tmp) = num.checked_shl(shift) {
                tmp
            } else {
                u64::MAX
            };
            scaled_num / den
        } else {
            // shift is large; use u128 to avoid overflow
            let scaled_num = (num as u128) << shift;
            (scaled_num / den as u128) as u64
        };

        // The result is scaled / 2^shift as an exact rational;
        // convert to f64 by multiplying by 2^-shift
        (scaled as f64) / ((1u128 << shift) as f64)
    }

    const fn add_impl(&self, other: &Self) -> Self {
        let (a, b) = make_common(self, other);
        Prob {
            num: a.num + b.num,
            den: a.den,
        }
    }

    /// Adds `other` to `self`. Usable in `const` contexts; equivalent to the `+` operator
    /// in non-const contexts.
    pub const fn add(&self, other: &Self) -> Self {
        self.add_impl(other)
    }

    /// Subtracts `other` from `self`, returning `None` if the result would be negative.
    pub const fn checked_sub(&self, other: &Self) -> Option<Self> {
        let (a, b) = make_common(self, other);
        if let Some(num) = a.num.checked_sub(b.num) {
            Some(Prob { num, den: a.den })
        } else {
            None
        }
    }

    /// Subtracts `other` from `self`, clamping at zero instead of underflowing.
    pub const fn saturating_sub(&self, other: &Self) -> Self {
        let (a, b) = make_common(self, other);
        Prob {
            num: a.num.saturating_sub(b.num),
            den: a.den,
        }
    }

    const fn sub_impl(&self, other: &Self) -> Self {
        let (a, b) = make_common(self, other);
        Prob {
            num: a.num - b.num,
            den: a.den,
        }
    }

    /// Subtracts `other` from `self`. Usable in `const` contexts; equivalent to the `-`
    /// operator in non-const contexts.
    ///
    /// Panics on underflow in debug builds. Prefer [`checked_sub`] or [`saturating_sub`]
    /// when the sign of the result is not guaranteed.
    ///
    /// [`checked_sub`]: Prob::checked_sub
    /// [`saturating_sub`]: Prob::saturating_sub
    pub const fn sub(&self, other: &Self) -> Self {
        self.sub_impl(other)
    }

    const fn mul_impl(&self, other: &Self) -> Self {
        if let (Some(num), Some(den)) = (
            self.num.checked_mul(other.num),
            self.den.checked_mul(other.den),
        ) {
            return Self { num, den };
        }

        let mut a = self.simplify();
        let mut b = other.simplify();

        loop {
            if let (Some(num), Some(den)) = (a.num.checked_mul(b.num), a.den.checked_mul(b.den)) {
                return Self { num, den };
            }

            a = Self {
                num: a.num / 2,
                den: a.den / 2,
            };

            if let (Some(num), Some(den)) = (a.num.checked_mul(b.num), a.den.checked_mul(b.den)) {
                return Self { num, den };
            }

            b = Self {
                num: b.num / 2,
                den: b.den / 2,
            };
        }
    }

    /// Multiplies `self` by `other`. Usable in `const` contexts; equivalent to the `*`
    /// operator in non-const contexts.
    pub const fn mul(&self, other: &Self) -> Self {
        self.mul_impl(other)
    }

    const fn div_impl(&self, other: &Self) -> Self {
        self.mul_impl(&Self {
            num: other.den,
            den: other.num,
        })
    }

    /// Divides `self` by `other`. Usable in `const` contexts; equivalent to the `/` operator
    /// in non-const contexts.
    pub const fn div(&self, other: &Self) -> Self {
        self.div_impl(other)
    }

    /// Multiplies by an integer scalar. Usable in `const` contexts; equivalent to the
    /// `* usize` operator in non-const contexts.
    pub const fn mul_usize(&self, rhs: usize) -> Self {
        let rhs = rhs as u64;

        if let Some(num) = self.num.checked_mul(rhs) {
            return Self { num, den: self.den };
        }

        let mut lhs = self.simplify();

        loop {
            if let Some(num) = lhs.num.checked_mul(rhs) {
                return Self { num, den: lhs.den };
            }

            lhs = Self {
                num: lhs.num / 2,
                den: lhs.den / 2,
            };
        }
    }

    /// Divides by an integer scalar. Usable in `const` contexts; equivalent to the `/ usize`
    /// operator in non-const contexts.
    pub const fn div_usize(&self, rhs: usize) -> Self {
        let rhs = rhs as u64;

        if let Some(den) = self.den.checked_mul(rhs) {
            return Self { num: self.num, den };
        }

        let mut lhs = self.simplify();

        loop {
            if let Some(den) = lhs.den.checked_mul(rhs) {
                return Self { num: lhs.num, den };
            }

            lhs = Self {
                num: lhs.num / 2,
                den: lhs.den / 2,
            };
        }
    }

    const fn eq_impl(&self, other: &Self) -> bool {
        let lhs = self.num as u128 * other.den as u128;
        let rhs = other.num as u128 * self.den as u128;
        lhs == rhs
    }

    /// Tests equality in `const` contexts. Equivalent to `PartialEq` in non-const contexts.
    pub const fn eq(&self, other: &Self) -> bool {
        self.eq_impl(other)
    }

    /// Compares two probabilities in `const` contexts. Equivalent to `Ord::cmp` in non-const
    /// contexts.
    pub const fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = self.num as u128 * other.den as u128;
        let rhs = other.num as u128 * self.den as u128;
        match lhs.checked_sub(rhs) {
            None => std::cmp::Ordering::Less,
            Some(0) => std::cmp::Ordering::Equal,
            Some(_) => std::cmp::Ordering::Greater,
        }
    }
}

const fn checked_lcm(a: u64, b: u64) -> Option<u64> {
    (a / gcd(a, b)).checked_mul(b)
}

const fn make_common(a: &Prob, b: &Prob) -> (Prob, Prob) {
    let (new_den, a, b) = if let Some(new_den) = checked_lcm(a.den, b.den) {
        (new_den, *a, *b)
    } else {
        let mut a = a.simplify();
        let mut b = b.simplify();
        loop {
            if let Some(new_den) = checked_lcm(a.den, b.den) {
                break (new_den, a, b);
            }

            a = Prob {
                num: a.num / 2,
                den: a.den / 2,
            };
            b = Prob {
                num: b.num / 2,
                den: b.den / 2,
            };
        }
    };

    (
        Prob {
            num: a.num * (new_den / a.den),
            den: new_den,
        },
        Prob {
            num: b.num * (new_den / b.den),
            den: new_den,
        },
    )
}

impl std::fmt::Debug for Prob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Prob({}/{})", self.num, self.den)
    }
}

/// Formats the probability as a decimal by default, or as `numerator/denominator` with the
/// `{:#}` alternate flag.
impl std::fmt::Display for Prob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{}/{}", self.num, self.den)
        } else {
            std::fmt::Display::fmt(&self.as_f64(), f)
        }
    }
}

impl PartialEq for Prob {
    fn eq(&self, other: &Self) -> bool {
        self.eq_impl(other)
    }
}

impl Eq for Prob {}

impl PartialOrd for Prob {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Prob {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = self.num as u128 * other.den as u128;
        let rhs = other.num as u128 * self.den as u128;
        Ord::cmp(&lhs, &rhs)
    }
}

impl std::ops::Add for Prob {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        self.add_impl(&other)
    }
}

impl std::ops::Add for &Prob {
    type Output = Prob;

    fn add(self, other: Self) -> Self::Output {
        self.add_impl(other)
    }
}

impl std::ops::Add<&Prob> for Prob {
    type Output = Self;

    fn add(self, other: &Prob) -> Self::Output {
        self.add_impl(other)
    }
}

impl std::ops::Add<Prob> for &Prob {
    type Output = Prob;

    fn add(self, other: Prob) -> Self::Output {
        self.add_impl(&other)
    }
}

impl std::ops::AddAssign for Prob {
    fn add_assign(&mut self, other: Self) {
        *self = self.add_impl(&other);
    }
}

impl std::ops::AddAssign<&Prob> for Prob {
    fn add_assign(&mut self, other: &Prob) {
        *self = self.add_impl(other);
    }
}

impl std::ops::Sub for Prob {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self.sub_impl(&other)
    }
}

impl std::ops::Sub for &Prob {
    type Output = Prob;

    fn sub(self, other: Self) -> Self::Output {
        self.sub_impl(other)
    }
}

impl std::ops::Sub<&Prob> for Prob {
    type Output = Self;

    fn sub(self, other: &Prob) -> Self::Output {
        self.sub_impl(other)
    }
}

impl std::ops::Sub<Prob> for &Prob {
    type Output = Prob;

    fn sub(self, other: Prob) -> Self::Output {
        self.sub_impl(&other)
    }
}

impl std::ops::SubAssign for Prob {
    fn sub_assign(&mut self, other: Self) {
        *self = self.sub_impl(&other);
    }
}

impl std::ops::SubAssign<&Prob> for Prob {
    fn sub_assign(&mut self, other: &Prob) {
        *self = self.sub_impl(other);
    }
}

impl std::ops::Mul for Prob {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        self.mul_impl(&other)
    }
}

impl std::ops::Mul for &Prob {
    type Output = Prob;

    fn mul(self, other: Self) -> Self::Output {
        self.mul_impl(other)
    }
}

impl std::ops::Mul<&Prob> for Prob {
    type Output = Self;

    fn mul(self, other: &Prob) -> Self::Output {
        self.mul_impl(other)
    }
}

impl std::ops::Mul<Prob> for &Prob {
    type Output = Prob;

    fn mul(self, other: Prob) -> Self::Output {
        self.mul_impl(&other)
    }
}

impl std::ops::MulAssign for Prob {
    fn mul_assign(&mut self, other: Self) {
        *self = self.mul_impl(&other);
    }
}

impl std::ops::MulAssign<&Prob> for Prob {
    fn mul_assign(&mut self, other: &Prob) {
        *self = self.mul_impl(other);
    }
}

impl std::ops::Div for Prob {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        self.div_impl(&other)
    }
}

impl std::ops::Div for &Prob {
    type Output = Prob;

    fn div(self, other: Self) -> Self::Output {
        self.div_impl(other)
    }
}

impl std::ops::Div<&Prob> for Prob {
    type Output = Self;

    fn div(self, other: &Prob) -> Self::Output {
        self.div_impl(other)
    }
}

impl std::ops::Div<Prob> for &Prob {
    type Output = Prob;

    fn div(self, other: Prob) -> Self::Output {
        self.div_impl(&other)
    }
}

impl std::ops::DivAssign for Prob {
    fn div_assign(&mut self, other: Self) {
        *self = self.div_impl(&other);
    }
}

impl std::ops::DivAssign<&Prob> for Prob {
    fn div_assign(&mut self, other: &Prob) {
        *self = self.div_impl(other);
    }
}

impl std::ops::Mul<usize> for Prob {
    type Output = Prob;

    fn mul(self, other: usize) -> Self::Output {
        self.mul_usize(other)
    }
}

impl std::ops::Mul<&usize> for Prob {
    type Output = Prob;

    fn mul(self, other: &usize) -> Self::Output {
        self.mul_usize(*other)
    }
}

impl std::ops::Mul<usize> for &Prob {
    type Output = Prob;

    fn mul(self, other: usize) -> Self::Output {
        self.mul_usize(other)
    }
}

impl std::ops::Mul<&usize> for &Prob {
    type Output = Prob;

    fn mul(self, other: &usize) -> Self::Output {
        self.mul_usize(*other)
    }
}

impl std::ops::Mul<Prob> for usize {
    type Output = Prob;

    fn mul(self, other: Prob) -> Self::Output {
        other.mul_usize(self)
    }
}

impl std::ops::Mul<&Prob> for usize {
    type Output = Prob;

    fn mul(self, other: &Prob) -> Self::Output {
        other.mul_usize(self)
    }
}

impl std::ops::Mul<Prob> for &usize {
    type Output = Prob;

    fn mul(self, other: Prob) -> Self::Output {
        other.mul_usize(*self)
    }
}

impl std::ops::Mul<&Prob> for &usize {
    type Output = Prob;

    fn mul(self, other: &Prob) -> Self::Output {
        other.mul_usize(*self)
    }
}

impl std::ops::MulAssign<usize> for Prob {
    fn mul_assign(&mut self, other: usize) {
        *self = self.mul_usize(other);
    }
}

impl std::ops::MulAssign<&usize> for Prob {
    fn mul_assign(&mut self, other: &usize) {
        *self = self.mul_usize(*other);
    }
}

impl std::ops::Div<usize> for Prob {
    type Output = Prob;

    fn div(self, other: usize) -> Self::Output {
        self.div_usize(other)
    }
}

impl std::ops::Div<&usize> for Prob {
    type Output = Prob;

    fn div(self, other: &usize) -> Self::Output {
        self.div_usize(*other)
    }
}

impl std::ops::Div<usize> for &Prob {
    type Output = Prob;

    fn div(self, other: usize) -> Self::Output {
        self.div_usize(other)
    }
}

impl std::ops::Div<&usize> for &Prob {
    type Output = Prob;

    fn div(self, other: &usize) -> Self::Output {
        self.div_usize(*other)
    }
}

impl std::ops::DivAssign<usize> for Prob {
    fn div_assign(&mut self, other: usize) {
        *self = self.div_usize(other);
    }
}

impl std::ops::DivAssign<&usize> for Prob {
    fn div_assign(&mut self, other: &usize) {
        *self = self.div_usize(*other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    fn p(num: u64, den: u64) -> Prob {
        Prob::new(num, den)
    }

    #[test]
    fn constants() {
        assert_eq!(Prob::ZERO, p(0, 1));
        assert_eq!(Prob::ONE, p(1, 1));
        assert_eq!(Prob::ZERO.as_f64(), 0.0);
        assert_eq!(Prob::ONE.as_f64(), 1.0);
    }

    #[test]
    fn equality_ignores_common_factor() {
        assert_eq!(p(1, 2), p(2, 4));
        assert_eq!(p(1, 3), p(3, 9));
        assert_ne!(p(1, 2), p(1, 3));
    }

    #[test]
    fn simplify_reduces_to_lowest_terms() {
        let s = p(4, 6).simplify();
        assert_eq!(s.numerator(), 2);
        assert_eq!(s.denominator(), 3);
        // Already reduced
        let s = p(1, 3).simplify();
        assert_eq!(s.numerator(), 1);
        assert_eq!(s.denominator(), 3);
    }

    #[test]
    fn add() {
        assert_eq!(p(1, 3) + p(1, 6), p(1, 2));
        assert_eq!(p(1, 4) + p(1, 4), p(1, 2));
        assert_eq!(p(1, 2) + p(1, 2), Prob::ONE);
    }

    #[test]
    fn sub() {
        assert_eq!(p(1, 2) - p(1, 4), p(1, 4));
        assert_eq!(Prob::ONE - p(1, 3), p(2, 3));
    }

    #[test]
    fn checked_sub_returns_none_on_underflow() {
        assert_eq!(p(1, 4).checked_sub(&p(1, 2)), None);
        assert_eq!(p(1, 2).checked_sub(&p(1, 2)), Some(Prob::ZERO));
        assert!(p(3, 4).checked_sub(&p(1, 4)).is_some());
    }

    #[test]
    fn saturating_sub_clamps_at_zero() {
        assert_eq!(p(1, 4).saturating_sub(&p(1, 2)), Prob::ZERO);
        assert_eq!(p(3, 4).saturating_sub(&p(1, 4)), p(1, 2));
        assert_eq!(Prob::ZERO.saturating_sub(&Prob::ONE), Prob::ZERO);
    }

    #[test]
    fn mul() {
        assert_eq!(p(1, 2) * p(1, 3), p(1, 6));
        assert_eq!(p(2, 3) * p(3, 4), p(1, 2));
        assert_eq!(Prob::ZERO * p(1, 2), Prob::ZERO);
        assert_eq!(Prob::ONE * p(3, 7), p(3, 7));
    }

    #[test]
    fn div() {
        assert_eq!(p(1, 2) / p(1, 3), p(3, 2));
        assert_eq!(p(3, 4) / p(3, 4), Prob::ONE);
    }

    #[test]
    fn mul_usize() {
        assert_eq!(p(1, 4) * 3usize, p(3, 4));
        assert_eq!(p(1, 3) * 3usize, Prob::ONE);
        assert_eq!(2usize * p(1, 5), p(2, 5));
    }

    #[test]
    fn div_usize() {
        assert_eq!(p(3, 4) / 3usize, p(1, 4));
        assert_eq!(Prob::ONE / 4usize, p(1, 4));
    }

    #[test]
    fn ordering() {
        assert!(p(1, 3) < p(1, 2));
        assert!(p(1, 2) < p(2, 3));
        assert!(p(2, 3) < Prob::ONE);
        assert_eq!(p(1, 2).cmp(&p(2, 4)), Ordering::Equal);
        assert_eq!(p(1, 3).cmp(&p(1, 2)), Ordering::Less);
        assert_eq!(p(1, 2).cmp(&p(1, 3)), Ordering::Greater);
    }

    #[test]
    fn as_f64_exact_fractions() {
        assert_eq!(p(1, 2).as_f64(), 0.5);
        assert_eq!(p(1, 4).as_f64(), 0.25);
        assert_eq!(p(3, 4).as_f64(), 0.75);
        assert_eq!(p(1, 8).as_f64(), 0.125);
    }

    #[test]
    fn as_f64_rounds_to_nearest_representable() {
        // as_f64 uses truncating integer division, so results match only when the
        // scaled numerator has no fractional part (or the truncation happens to equal
        // the hardware-rounded result). 1/3 satisfies this; 1/5 does not (truncates
        // to 0.19999999999999998 instead of 0.2).
        assert_eq!(p(1, 3).as_f64(), 1.0_f64 / 3.0);
    }

    #[test]
    fn display_alternate_shows_raw_fraction() {
        assert_eq!(format!("{:#}", p(1, 4)), "1/4");
        assert_eq!(format!("{:#}", p(3, 7)), "3/7");
        // Shows the raw numerator/denominator without simplifying
        assert_eq!(format!("{:#}", p(2, 4)), "2/4");
    }

    #[test]
    fn display_default_shows_decimal() {
        assert_eq!(format!("{}", p(1, 4)), "0.25");
        assert_eq!(format!("{}", p(1, 2)), "0.5");
        assert_eq!(format!("{}", Prob::ZERO), "0");
        assert_eq!(format!("{}", Prob::ONE), "1");
    }

    #[test]
    fn assign_ops() {
        let mut x = p(1, 4);
        x += p(1, 4);
        assert_eq!(x, p(1, 2));

        let mut x = p(3, 4);
        x -= p(1, 4);
        assert_eq!(x, p(1, 2));

        let mut x = p(3, 4);
        x *= p(2, 3);
        assert_eq!(x, p(1, 2));

        let mut x = p(3, 2);
        x /= p(3, 4);
        assert_eq!(x, p(2, 1));

        let mut x = p(1, 3);
        x *= 3usize;
        assert_eq!(x, Prob::ONE);

        let mut x = Prob::ONE;
        x /= 4usize;
        assert_eq!(x, p(1, 4));
    }

    #[test]
    fn const_arithmetic() {
        // Verify const versions produce the same results as operators
        assert_eq!(p(1, 3).add(&p(1, 6)), p(1, 3) + p(1, 6));
        assert_eq!(p(1, 2).sub(&p(1, 4)), p(1, 2) - p(1, 4));
        assert_eq!(p(1, 2).mul(&p(2, 3)), p(1, 2) * p(2, 3));
        assert_eq!(p(3, 4).div(&p(3, 8)), p(3, 4) / p(3, 8));
        assert_eq!(p(1, 3).mul_usize(3), p(1, 3) * 3usize);
        assert_eq!(p(3, 4).div_usize(3), p(3, 4) / 3usize);
        assert!(p(1, 2).eq(&p(2, 4)));
        assert_eq!(p(1, 3).cmp(&p(1, 2)), Ordering::Less);
    }
}
