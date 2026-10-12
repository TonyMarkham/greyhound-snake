use crate::error::{Error, Result};

/// SplitMix64 (Steele, Lea, Flood 2014) — the deterministic PRNG
/// behind seeded selection.
///
/// Chosen because the whole algorithm is a handful of fixed 64-bit
/// ops: the same seed produces the same stream on every platform,
/// with no platform RNG and no dependency. Golden vectors for seed 0
/// (first three draws) and seed `0xdeadbeef` are pinned by
/// `tests::prng::golden_vectors_match_the_reference_implementation`.
pub struct SplitMix64 {
    state: u64,
}

/// The golden-ratio increment from the SplitMix64 specification.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The raw 64-bit stream value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// An unbiased draw from `[0, bound)` (Lemire-style rejection):
    /// the top `2^64 mod bound` values of the raw stream are refused,
    /// so the modulo never skews toward zero and the same seed gives
    /// the same sequence on every platform.
    pub fn below(&mut self, bound: u32) -> Result<u32> {
        let bound = u64::from(bound);
        if bound == 0 {
            return Err(Error::aruco("draw range is empty"));
        }
        let top = (u64::MAX % bound + 1) % bound;
        loop {
            let draw = self.next_u64();
            if top == 0 || draw <= u64::MAX - top {
                return u32::try_from(draw % bound)
                    .map_err(|_| Error::aruco("draw exceeds the u32 bound"));
            }
        }
    }
}
