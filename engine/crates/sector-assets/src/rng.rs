//! Deterministic random numbers (PCG32, XSH-RR). The same seed gives the same
//! sequence on every platform, so every client in a match can build the same
//! procedural content from a shared seed.

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
    increment: u64,
}

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Stream selection from the seed's high bits, as recommended for PCG.
        let mut rng = Rng { state: 0, increment: (seed.rotate_left(32) << 1) | 1 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.increment);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        xorshifted.rotate_right((old >> 59) as u32)
    }

    /// Uniform integer in `lo..=hi` (unbiased).
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi, "empty range {lo}..={hi}");
        let span = (hi as i64 - lo as i64 + 1) as u64;
        if span > u32::MAX as u64 {
            return lo.wrapping_add(self.next_u32() as i32);
        }
        let span = span as u32;
        // Rejection sampling removes modulo bias.
        let limit = u32::MAX - u32::MAX % span;
        loop {
            let value = self.next_u32();
            if value < limit {
                return lo + (value % span) as i32;
            }
        }
    }

    /// Uniform index in `0..len`.
    pub fn index(&mut self, len: usize) -> usize {
        assert!(len > 0, "index into an empty collection");
        self.range_i32(0, len as i32 - 1) as usize
    }

    /// Uniform float in `lo..=hi` with 24 bits of precision.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u32() >> 8) as f32 / ((1u32 << 24) - 1) as f32;
        lo + (hi - lo) * unit
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.range_i32(0, i as i32) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let first: Vec<u32> = (0..8).map(|_| a.next_u32()).collect();
        assert_eq!(first, (0..8).map(|_| b.next_u32()).collect::<Vec<_>>());
        let mut c = Rng::new(43);
        assert_ne!(first, (0..8).map(|_| c.next_u32()).collect::<Vec<_>>());
    }

    #[test]
    fn sequence_is_pinned() {
        // Guards against accidental algorithm changes, which would change every seeded city.
        let mut rng = Rng::new(42);
        let values: Vec<u32> = (0..3).map(|_| rng.next_u32()).collect();
        let mut again = Rng::new(42);
        assert_eq!(values, (0..3).map(|_| again.next_u32()).collect::<Vec<_>>());
        assert_eq!(values, PINNED.to_vec());
    }

    const PINNED: [u32; 3] = [1_757_512_667, 2_037_027_432, 2_844_168_553];

    #[test]
    fn ranges_stay_in_bounds_and_cover_them() {
        let mut rng = Rng::new(7);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let v = rng.range_i32(-2, 2);
            assert!((-2..=2).contains(&v));
            seen[(v + 2) as usize] = true;
            let f = rng.range_f32(1.5, 2.5);
            assert!((1.5..=2.5).contains(&f));
        }
        assert!(seen.iter().all(|s| *s));
        assert_eq!(rng.range_i32(3, 3), 3);
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut items: Vec<u32> = (0..50).collect();
        Rng::new(1).shuffle(&mut items);
        assert_ne!(items, (0..50).collect::<Vec<_>>());
        items.sort_unstable();
        assert_eq!(items, (0..50).collect::<Vec<_>>());
    }
}
