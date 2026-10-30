//! Display formatting for card version collector numbers.

use std::fmt;

/// A collector number formatted for display: the magnitude is zero-padded to three digits and
/// a negative sign is kept in front (e.g. `025`, `-005`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardNumber(pub isize);

impl fmt::Display for CardNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{sign}{:03}", self.0.unsigned_abs())
    }
}

#[cfg(test)]
mod tests {
    use super::CardNumber;

    #[test]
    fn pads_magnitude_and_keeps_sign() {
        assert_eq!(CardNumber(25).to_string(), "025");
        assert_eq!(CardNumber(1234).to_string(), "1234");
        assert_eq!(CardNumber(0).to_string(), "000");
        assert_eq!(CardNumber(-5).to_string(), "-005");
    }
}
