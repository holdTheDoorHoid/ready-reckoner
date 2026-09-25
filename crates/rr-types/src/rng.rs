//! A tiny seeded pseudo-random generator for any future need (sampling, jitter in tests).
//!
//! The engine is deterministic: never use OS entropy or the `rand` crate (`getrandom` does not
//! build for wasm32-unknown-unknown). Seed a [`SplitMix64`] from something in the input instead.
//! SplitMix64 is Sebastiano Vigna's public-domain generator; it is fast, has a 2^64 period and
//! passes BigCrush, but it is not cryptographic.

/// SplitMix64: 64 bits of state, one addition and two multiply-xorshift rounds per output.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose sequence is fixed by `seed`.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 uniformly distributed bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform `f64` in `[0, 1)` with 53 random bits.
    pub fn next_f64(&mut self) -> f64 {
        // The top 53 bits scaled by 2^-53; every value is exactly representable.
        const SCALE: f64 = 1.0 / (1u64 << 53) as f64;
        (self.next_u64() >> 11) as f64 * SCALE
    }

    /// A uniform integer in `[0, bound)`, without modulo bias (Lemire's method). Returns 0 when
    /// `bound` is 0.
    pub fn next_below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        let mut m = u128::from(self.next_u64()) * u128::from(bound);
        let mut low = m as u64;
        if low < bound {
            let threshold = bound.wrapping_neg() % bound;
            while low < threshold {
                m = u128::from(self.next_u64()) * u128::from(bound);
                low = m as u64;
            }
        }
        (m >> 64) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_sequence() {
        // Reference outputs of Vigna's splitmix64.c, recomputed independently in Python.
        let mut g = SplitMix64::new(0);
        let got: Vec<u64> = (0..4).map(|_| g.next_u64()).collect();
        assert_eq!(
            got,
            [
                0xE220_A839_7B1D_CDAF,
                0x6E78_9E6A_A1B9_65F4,
                0x06C4_5D18_8009_454F,
                0xF88B_B8A8_724C_81EC
            ]
        );
        let mut g = SplitMix64::new(42);
        let got: Vec<u64> = (0..4).map(|_| g.next_u64()).collect();
        assert_eq!(
            got,
            [
                0xBDD7_3226_2FEB_6E95,
                0x28EF_E333_B266_F103,
                0x4752_6757_130F_9F52,
                0x581C_E1FF_0E4A_E394
            ]
        );
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(SplitMix64::new(7).next_u64(), SplitMix64::new(8).next_u64());
    }

    #[test]
    fn floats_are_in_the_unit_interval_and_roughly_uniform() {
        let mut g = SplitMix64::new(1);
        let n = 100_000;
        let mut sum = 0.0;
        let mut buckets = [0u32; 10];
        for _ in 0..n {
            let x = g.next_f64();
            assert!((0.0..1.0).contains(&x));
            sum += x;
            buckets[(x * 10.0) as usize] += 1;
        }
        let mean = sum / f64::from(n);
        assert!((mean - 0.5).abs() < 0.005, "mean {mean}");
        for count in buckets {
            assert!((9_500..=10_500).contains(&count), "bucket count {count}");
        }
    }

    #[test]
    fn bounded_integers_stay_in_range_and_cover_it() {
        let mut g = SplitMix64::new(3);
        let mut seen = [false; 6];
        for _ in 0..10_000 {
            let v = g.next_below(6);
            assert!(v < 6);
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|s| *s));
        assert_eq!(g.next_below(0), 0);
        assert_eq!(g.next_below(1), 0);
        assert!(g.next_below(u64::MAX) < u64::MAX);
    }
}
