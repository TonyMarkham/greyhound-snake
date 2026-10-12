use crate::dictionary::ArucoDictionary;
use crate::error::{Error, Result};
use crate::hamming::{TAU_TARGET, min_rotation_hamming, self_distance};
use crate::marker::Marker;
use crate::prng::SplitMix64;

/// How `--count N` picks its markers out of the
/// self-symmetry-filtered pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// The N lowest ids in dictionary order.
    FirstN,
    /// Seeded Fisher-Yates order of the pool; same seed, same set.
    Seeded { seed: u64 },
    /// Seeded greedy max-min: repeatedly take the candidate whose
    /// minimum distance to everything already picked is highest
    /// (ties go to higher self-distance, then to shuffled order).
    /// `min_distance` is a floor — if the greedy pass cannot reach it,
    /// selection errors rather than emitting a weaker set.
    MaxDistance { seed: u64, min_distance: u32 },
}

/// The pool for `--count N`: every dictionary marker that is
/// orientation-resolvable. A marker equal to any of its own rotations
/// is dropped (self-distance 0) — inside the predefined OpenCV 4x4
/// dictionaries this filters nothing (count 0, pinned by tests), but
/// the filter is what makes the rotation-uniqueness guarantee
/// structural rather than incidental.
pub fn filter_self_symmetric(dict: ArucoDictionary) -> Result<Vec<Marker>> {
    let mut pool = Vec::new();
    for marker in dict.markers() {
        let marker = marker?;
        if self_distance(marker.bits) > 0 {
            pool.push(marker);
        }
    }
    Ok(pool)
}

/// Picks `count` markers from the pool. The result keeps the
/// rotation-uniqueness property (no rotation of one picked marker
/// equals any rotation of another) — inside an OpenCV pool this holds
/// automatically (documented minimum distances 4/3/3/2 ≥ 1); the
/// selection tests assert it anyway.
pub fn select(pool: &[Marker], count: u32, selection: Selection) -> Result<Vec<Marker>> {
    if count == 0 {
        return Err(Error::aruco("select needs at least one marker"));
    }
    let count = usize::try_from(count).map_err(|_| Error::aruco("count does not fit usize"))?;
    if count > pool.len() {
        return Err(Error::aruco(format!(
            "asked for {count} markers but the pool holds {}",
            pool.len()
        )));
    }
    match selection {
        Selection::FirstN => Ok(pool[..count].to_vec()),
        Selection::Seeded { seed } => {
            let mut rng = SplitMix64::new(seed);
            let mut picked = shuffled(pool.to_vec(), &mut rng)?;
            picked.truncate(count);
            Ok(picked)
        }
        Selection::MaxDistance { seed, min_distance } => {
            greedy_max_distance(pool, count, seed, min_distance)
        }
    }
}

/// Seeded Fisher-Yates over a copy of the pool. Indices travel the
/// u32 domain (`below`) and convert to `usize` only for the swap —
/// the repo's no-`as`-cast rule, with errors instead of narrowing
/// casts.
fn shuffled(markers: Vec<Marker>, rng: &mut SplitMix64) -> Result<Vec<Marker>> {
    let mut out = markers;
    let len = u32::try_from(out.len()).map_err(|_| Error::aruco("pool exceeds u32 markers"))?;
    for i in (1..len).rev() {
        let j = rng.below(i + 1)?;
        let i = usize::try_from(i).map_err(|_| Error::aruco("index out of range"))?;
        let j = usize::try_from(j).map_err(|_| Error::aruco("index out of range"))?;
        out.swap(i, j);
    }
    Ok(out)
}

/// Greedy max-min selection: start from the first shuffled candidate,
/// then repeatedly take the candidate maximizing its minimum
/// min-rotation-Hamming distance to the selected set (ties: higher
/// self-distance, then shuffled order). Fails if the achieved minimum
/// pairwise distance stays under `min_distance`.
fn greedy_max_distance(
    pool: &[Marker],
    count: usize,
    seed: u64,
    min_distance: u32,
) -> Result<Vec<Marker>> {
    let mut rng = SplitMix64::new(seed);
    let mut remaining = shuffled(pool.to_vec(), &mut rng)?;
    let mut selected: Vec<Marker> = vec![remaining.remove(0)];
    while selected.len() < count {
        let mut best: Option<(usize, u32, u32)> = None;
        for (index, candidate) in remaining.iter().enumerate() {
            let mut score = u32::MAX;
            for chosen in &selected {
                score = score.min(min_rotation_hamming(candidate.bits, chosen.bits));
            }
            let distinctness = self_distance(candidate.bits);
            let better = match best {
                None => true,
                Some((_, best_score, best_distinctness)) => {
                    score > best_score || (score == best_score && distinctness > best_distinctness)
                }
            };
            if better {
                best = Some((index, score, distinctness));
            }
        }
        match best {
            Some((index, _, _)) => selected.push(remaining.remove(index)),
            None => {
                // Unreachable while `count <= pool.len()` — the loop
                // body runs only when `selected.len() < count`, which
                // leaves candidates. Defensive, not dead: the
                // workspace denies panic/expect in library code.
                return Err(Error::aruco("the candidate pool ran dry mid-selection"));
            }
        }
    }
    let mut achieved = u32::MAX;
    for (index, marker) in selected.iter().enumerate() {
        for other in selected.iter().skip(index + 1) {
            achieved = achieved.min(min_rotation_hamming(marker.bits, other.bits));
        }
    }
    if achieved < min_distance {
        return Err(Error::aruco(format!(
            "cannot pick {count} markers with minimum distance {min_distance}; \
             the greedy pass reached {achieved} (paper target tau = {TAU_TARGET})"
        )));
    }
    Ok(selected)
}
