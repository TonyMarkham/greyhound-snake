use crate::dictionary::ArucoDictionary;
use crate::marker::Marker;
use crate::rotation::rotations;
use crate::select::{Selection, filter_self_symmetric, select};

fn pool() -> Vec<Marker> {
    filter_self_symmetric(ArucoDictionary::Dict4x4_1000).expect("dictionary ids are dense")
}

fn ids(markers: &[Marker]) -> Vec<u32> {
    markers.iter().map(|marker| marker.id).collect()
}

#[test]
fn the_pool_holds_every_dictionary_marker() {
    // Zero self-symmetric markers in the predefined dicts (test
    // constant, also pinned in tests::dictionary) -> the pool is the
    // full 1000.
    assert_eq!(pool().len(), 1000);
}

#[test]
fn first_n_takes_the_lowest_ids_in_order() {
    let picked = select(&pool(), 12, Selection::FirstN).expect("12 <= 1000");
    assert_eq!(ids(&picked), (0..12).collect::<Vec<u32>>());
}

#[test]
fn count_zero_is_an_error() {
    let error = select(&pool(), 0, Selection::FirstN).expect_err("0 is rejected");
    assert!(error.to_string().contains("at least one"), "{error}");
}

#[test]
fn count_beyond_the_pool_is_an_error() {
    let error = select(&pool(), 1001, Selection::FirstN).expect_err("1001 > 1000 pool");
    assert!(error.to_string().contains("pool holds 1000"), "{error}");
}

#[test]
fn seeded_selection_is_reproducible() {
    let first = select(&pool(), 24, Selection::Seeded { seed: 1234 }).expect("24 <= 1000");
    let second = select(&pool(), 24, Selection::Seeded { seed: 1234 }).expect("24 <= 1000");
    assert_eq!(ids(&first), ids(&second));
}

#[test]
fn different_seeds_give_different_orders() {
    let a = select(&pool(), 24, Selection::Seeded { seed: 1 }).expect("24 <= 1000");
    let b = select(&pool(), 24, Selection::Seeded { seed: 2 }).expect("24 <= 1000");
    assert_ne!(ids(&a), ids(&b));
}

#[test]
fn every_selection_is_rotation_unique() {
    // The angular-uniqueness property, asserted per selection: no
    // rotation of one picked marker equals any rotation of another.
    for picked in [
        select(&pool(), 50, Selection::FirstN).expect("50 <= 1000"),
        select(&pool(), 50, Selection::Seeded { seed: 5 }).expect("50 <= 1000"),
        select(
            &pool(),
            50,
            Selection::MaxDistance {
                seed: 5,
                min_distance: 0,
            },
        )
        .expect("floor 0 is always reachable"),
    ] {
        for (index, marker) in picked.iter().enumerate() {
            let ours = rotations(marker.bits);
            for other in picked.iter().skip(index + 1) {
                for rotated in ours {
                    assert_ne!(
                        rotated, other.bits,
                        "rotation collision between {} and {}",
                        marker.id, other.id
                    );
                }
            }
        }
    }
}

#[test]
fn max_distance_reaches_a_six_bit_floor_for_twelve() {
    // Measured with the Rust greedy (SplitMix64 Fisher-Yates) over the
    // transcribed table via the CLI: for 12 picks at a 6-bit floor,
    // seed 6 succeeds; seeds 1-5, 7, 8 reach 5 and error. The floor is
    // seed-dependent by design — retry other seeds or lower the floor.
    let picked = select(
        &pool(),
        12,
        Selection::MaxDistance {
            seed: 6,
            min_distance: 6,
        },
    )
    .expect("seed 6 reaches a 6-bit floor for 12 markers");
    let mut minimum = u32::MAX;
    for (index, marker) in picked.iter().enumerate() {
        for other in picked.iter().skip(index + 1) {
            minimum = minimum.min(crate::hamming::min_rotation_hamming(
                marker.bits,
                other.bits,
            ));
        }
    }
    assert!(minimum >= 6, "achieved minimum distance {minimum}");
    assert_eq!(ids(&picked).len(), 12);
    assert!(ids(&picked).iter().all(|id| *id < 1000));
}

#[test]
fn a_seed_that_misses_the_floor_errors_instead_of_shrinking() {
    // Seed 42's greedy reaches 5 for 12 picks; asking for 6 must
    // error naming the floor and the achieved value — never emit a
    // weaker set silently.
    let error = select(
        &pool(),
        12,
        Selection::MaxDistance {
            seed: 42,
            min_distance: 6,
        },
    )
    .expect_err("seed 42 does not reach 6 for 12 picks");
    assert!(error.to_string().contains("minimum distance 6"), "{error}");
    assert!(error.to_string().contains("reached 5"), "{error}");
}

#[test]
fn an_unreachable_floor_is_an_error() {
    // Floor 16 would need two 4x4 fields differing in every bit at
    // every relative rotation — impossible, and refused fast (2
    // picks, one scoring pass).
    let error = select(
        &pool(),
        2,
        Selection::MaxDistance {
            seed: 1,
            min_distance: 16,
        },
    )
    .expect_err("floor 16 is unreachable");
    assert!(error.to_string().contains("minimum distance 16"), "{error}");
}

#[test]
fn the_paper_tau_is_out_of_greedy_reach() {
    // tau = 10 is the generator's target, not what the predefined
    // table offers beyond small counts; a large pick must refuse.
    let error = select(
        &pool(),
        100,
        Selection::MaxDistance {
            seed: 0,
            min_distance: 10,
        },
    )
    .expect_err("100 markers at tau is unreachable");
    assert!(error.to_string().contains("10"), "{error}");
}

#[test]
fn filter_keeps_the_pool_in_id_order() {
    let pool = pool();
    assert_eq!(ids(&pool), (0..1000).collect::<Vec<u32>>());
}

#[test]
fn empty_pool_cannot_select() {
    let error = select(&[], 1, Selection::FirstN).expect_err("pool is empty");
    assert!(error.to_string().contains("pool holds 0"), "{error}");
}
