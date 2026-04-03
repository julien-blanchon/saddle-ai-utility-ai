use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct PriorityTier(pub u8);

impl PriorityTier {
    pub const CRITICAL: Self = Self(0);
    pub const TACTICAL: Self = Self(10);
    pub const FLAVOR: Self = Self(20);
}

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub enum SelectionStrategy {
    #[default]
    HighestScore,
    WeightedRandom,
    TopNRandom {
        count: usize,
    },
    ThresholdFirst {
        threshold: f32,
    },
    TopBandRandom {
        percent_within_best: f32,
    },
}

pub fn select_index(scores: &[f32], strategy: &SelectionStrategy, seed: u64) -> Option<usize> {
    let positive = positive_indices(scores);
    if positive.is_empty() {
        return None;
    }

    match strategy {
        SelectionStrategy::HighestScore => positive.into_iter().max_by(|left, right| {
            scores[*left]
                .total_cmp(&scores[*right])
                .then_with(|| right.cmp(left))
        }),
        SelectionStrategy::WeightedRandom => weighted_pick(scores, &positive, seed),
        SelectionStrategy::TopNRandom { count } => {
            let mut ranked = positive;
            ranked.sort_by(|left, right| {
                scores[*right]
                    .total_cmp(&scores[*left])
                    .then_with(|| left.cmp(right))
            });
            let top_n = ranked.into_iter().take((*count).max(1)).collect::<Vec<_>>();
            uniform_pick(&top_n, seed)
        }
        SelectionStrategy::ThresholdFirst { threshold } => {
            scores.iter().enumerate().find_map(|(index, score)| {
                (*score >= *threshold && score.is_finite()).then_some(index)
            })
        }
        SelectionStrategy::TopBandRandom {
            percent_within_best,
        } => {
            let best = positive
                .iter()
                .copied()
                .map(|index| scores[index])
                .fold(0.0, f32::max);
            let tolerance = best * percent_within_best.clamp(0.0, 1.0);
            let band = positive
                .into_iter()
                .filter(|index| scores[*index] + tolerance >= best)
                .collect::<Vec<_>>();
            weighted_pick(scores, &band, seed)
        }
    }
}

fn positive_indices(scores: &[f32]) -> Vec<usize> {
    scores
        .iter()
        .enumerate()
        .filter_map(|(index, score)| (score.is_finite() && *score > 0.0).then_some(index))
        .collect::<Vec<_>>()
}

fn uniform_pick(indices: &[usize], seed: u64) -> Option<usize> {
    if indices.is_empty() {
        return None;
    }
    let pick = (mix_u64(seed) as usize) % indices.len();
    indices.get(pick).copied()
}

fn weighted_pick(scores: &[f32], indices: &[usize], seed: u64) -> Option<usize> {
    if indices.is_empty() {
        return None;
    }

    let total = indices
        .iter()
        .copied()
        .map(|index| scores[index])
        .sum::<f32>();
    if total <= 0.0 || !total.is_finite() {
        return None;
    }

    let mut remaining = unit_from_seed(seed) * total;
    for index in indices.iter().copied() {
        remaining -= scores[index];
        if remaining <= 0.0 {
            return Some(index);
        }
    }

    indices.last().copied()
}

pub(crate) fn unit_from_seed(seed: u64) -> f32 {
    let mixed = mix_u64(seed);
    let mantissa = (mixed >> 40) as u32;
    mantissa as f32 / ((1u32 << 24) as f32)
}

pub(crate) fn mix_u64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    state = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^ (state >> 31)
}
