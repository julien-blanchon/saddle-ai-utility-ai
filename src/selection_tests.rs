use super::*;

#[test]
fn highest_score_is_deterministic() {
    let index = select_index(&[0.2, 0.8, 0.5], &SelectionStrategy::HighestScore, 7);
    assert_eq!(index, Some(1));
}

#[test]
fn threshold_first_uses_order() {
    let index = select_index(
        &[0.3, 0.7, 0.9],
        &SelectionStrategy::ThresholdFirst { threshold: 0.65 },
        7,
    );
    assert_eq!(index, Some(1));
}

#[test]
fn weighted_random_excludes_zero_scores() {
    for seed in 0..16 {
        let index = select_index(&[0.0, 0.0, 0.9], &SelectionStrategy::WeightedRandom, seed);
        assert_eq!(index, Some(2));
    }
}

#[test]
fn top_band_random_stays_near_best() {
    for seed in 0..32 {
        let index = select_index(
            &[0.95, 0.92, 0.4],
            &SelectionStrategy::TopBandRandom {
                percent_within_best: 0.05,
            },
            seed,
        )
        .unwrap();
        assert!(index == 0 || index == 1);
    }
}

#[test]
fn weighted_random_is_seed_repeatable() {
    let left = select_index(&[0.2, 0.5, 0.3], &SelectionStrategy::WeightedRandom, 99);
    let right = select_index(&[0.2, 0.5, 0.3], &SelectionStrategy::WeightedRandom, 99);
    assert_eq!(left, right);
}

#[test]
fn top_n_random_never_returns_outside_top_n() {
    for seed in 0..32 {
        let index = select_index(
            &[0.9, 0.8, 0.2, 0.1],
            &SelectionStrategy::TopNRandom { count: 2 },
            seed,
        )
        .unwrap();
        assert!(index == 0 || index == 1);
    }
}

#[test]
fn no_strategy_returns_out_of_range_index() {
    let strategies = [
        SelectionStrategy::HighestScore,
        SelectionStrategy::WeightedRandom,
        SelectionStrategy::TopNRandom { count: 3 },
        SelectionStrategy::ThresholdFirst { threshold: 0.1 },
        SelectionStrategy::TopBandRandom {
            percent_within_best: 0.2,
        },
    ];

    for strategy in strategies {
        for seed in 0..16 {
            let index = select_index(&[0.4, 0.6, 0.5], &strategy, seed).unwrap();
            assert!(index < 3);
        }
    }
}
