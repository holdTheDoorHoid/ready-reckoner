//! Money and large counts as the packet and the app print them: whole dollars with thousands
//! separators ("$4,600"), the same rule as `rr-plan`'s packet text, shared here so every engine
//! crate writes amounts the same way.

/// Whole dollars with thousands separators: "$30", "$4,600", "$16,800"; an amount above zero but
/// under half a dollar is "under $1"; negative or non-finite amounts print as "$0".
pub fn usd(x: f64) -> String {
    if x > 0.0 && x < 0.5 {
        return "under $1".to_owned();
    }
    let whole = if x.is_finite() {
        x.round().max(0.0)
    } else {
        0.0
    };
    format!("${}", thousands(whole as u64))
}

/// A whole number with thousands separators: "950", "1,200", "1,000,000".
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollars_and_counts_group_by_thousands() {
        assert_eq!(usd(4600.0), "$4,600");
        assert_eq!(usd(16_800.4), "$16,800");
        assert_eq!(usd(30.0), "$30");
        assert_eq!(usd(999.5), "$1,000");
        assert_eq!(usd(0.3), "under $1");
        assert_eq!(usd(0.0), "$0");
        assert_eq!(usd(-5.0), "$0");
        assert_eq!(usd(f64::NAN), "$0");
        assert_eq!(thousands(950), "950");
        assert_eq!(thousands(1200), "1,200");
        assert_eq!(thousands(1_000_000), "1,000,000");
    }
}
