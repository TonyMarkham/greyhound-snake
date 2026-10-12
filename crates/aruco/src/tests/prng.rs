use crate::prng::SplitMix64;

#[test]
fn golden_vectors_match_the_reference_implementation() {
    // Computed from the SplitMix64 definition (state += golden; mix)
    // with exact big-integer arithmetic.
    let mut rng = SplitMix64::new(0);
    assert_eq!(rng.next_u64(), 0xe220a8397b1dcdaf);
    assert_eq!(rng.next_u64(), 0x6e789e6aa1b965f4);
    assert_eq!(rng.next_u64(), 0x6c45d188009454f);

    let mut rng = SplitMix64::new(0xdead_beef);
    assert_eq!(rng.next_u64(), 0x4adfb90f68c9eb9b);
}

#[test]
fn the_same_seed_replays_the_same_stream() {
    let mut a = SplitMix64::new(42);
    let mut b = SplitMix64::new(42);
    for _ in 0..100 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn different_seeds_diverge_immediately() {
    let mut a = SplitMix64::new(1);
    let mut b = SplitMix64::new(2);
    assert_ne!(a.next_u64(), b.next_u64());
}

#[test]
fn below_is_unbiased_enough_to_catch_a_broken_rejection_scheme() {
    // 20k draws of below(2): both outcomes appear, neither side is
    // wildly off (a modulo-biased scheme would show ~25% skew here;
    // split point is 10k +- ~200 for this sample size).
    let mut rng = SplitMix64::new(7);
    let mut ones = 0u32;
    for _ in 0..20_000 {
        ones += rng.below(2).expect("bound 2 is valid");
    }
    assert!(ones > 9_000 && ones < 11_000, "ones = {ones}");
}

#[test]
fn below_covers_the_full_range() {
    // Every value of a small bound is reachable within a modest run.
    let mut rng = SplitMix64::new(9);
    let mut seen = [false; 6];
    for _ in 0..200 {
        let draw =
            usize::try_from(rng.below(6).expect("bound 6 is valid")).expect("u32 fits usize");
        seen[draw] = true;
    }
    assert!(seen.iter().all(|hit| *hit), "uncovered: {seen:?}");
}

#[test]
fn below_of_zero_bound_is_an_error() {
    let mut rng = SplitMix64::new(1);
    let error = rng.below(0).expect_err("bound 0 is empty");
    assert!(error.to_string().contains("empty"));
}

#[test]
fn below_of_one_bound_is_always_zero() {
    let mut rng = SplitMix64::new(3);
    for _ in 0..50 {
        assert_eq!(rng.below(1).expect("bound 1 is valid"), 0);
    }
}
