use gcd::binary_u64 as gcd;

#[derive(Clone, Copy, Default)]
pub struct Prob {
    pub(crate) num: u64,
    pub(crate) den: u64,
}

impl Prob {
    pub const ZERO: Self = Self { num: 0, den: 1 };

    pub const ONE: Self = Self { num: 1, den: 1 };

    pub const fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            num: numerator,
            den: denominator,
        }
    }

    pub const fn numerator(&self) -> u64 {
        self.num
    }

    pub const fn denominator(&self) -> u64 {
        self.den
    }

    pub const fn simplify(&self) -> Self {
        let gcd = gcd(self.num, self.den);
        Self {
            num: self.num / gcd,
            den: self.den / gcd,
        }
    }

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
        (scaled as f64) / ((1u64 << shift) as f64)
    }

    const fn add_impl(&self, other: &Self) -> Self {
        let (a, b) = make_common(self, other);
        Prob {
            num: a.num + b.num,
            den: a.den,
        }
    }

    pub const fn add(&self, other: &Self) -> Self {
        self.add_impl(other)
    }

    pub const fn checked_sub(&self, other: &Self) -> Option<Self> {
        let (a, b) = make_common(self, other);
        if let Some(num) = a.num.checked_sub(b.num) {
            Some(Prob { num, den: a.den })
        } else {
            None
        }
    }

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

    pub const fn mul(&self, other: &Self) -> Self {
        self.mul_impl(other)
    }

    const fn div_impl(&self, other: &Self) -> Self {
        self.mul_impl(&Self {
            num: other.den,
            den: other.num,
        })
    }

    pub const fn div(&self, other: &Self) -> Self {
        self.div_impl(other)
    }

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

    pub const fn eq(&self, other: &Self) -> bool {
        self.eq_impl(other)
    }

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
