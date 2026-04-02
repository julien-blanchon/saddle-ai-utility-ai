use super::*;

#[test]
fn multiplicative_scoring_zero_kills() {
    let outcome = compose_scores(
        &[
            ConsiderationOperand {
                score: 1.0,
                weight: 1.0,
                enabled: true,
            },
            ConsiderationOperand {
                score: 0.0,
                weight: 1.0,
                enabled: true,
            },
        ],
        &CompositionPolicy {
            strategy: CompositionStrategy::Multiplicative,
            ..default()
        },
    );
    assert_eq!(outcome.score, 0.0);
    assert!(outcome.zero_hit);
}

#[test]
fn geometric_mean_normalizes_across_count() {
    let left = compose_scores(
        &[ConsiderationOperand {
            score: 0.25,
            weight: 1.0,
            enabled: true,
        }],
        &CompositionPolicy::default(),
    );
    let right = compose_scores(
        &[
            ConsiderationOperand {
                score: 0.25,
                weight: 1.0,
                enabled: true,
            },
            ConsiderationOperand {
                score: 0.25,
                weight: 1.0,
                enabled: true,
            },
        ],
        &CompositionPolicy::default(),
    );
    assert!((left.score - right.score).abs() < 0.001);
}

#[test]
fn additive_weighting_respects_weights() {
    let outcome = compose_scores(
        &[
            ConsiderationOperand {
                score: 1.0,
                weight: 3.0,
                enabled: true,
            },
            ConsiderationOperand {
                score: 0.0,
                weight: 1.0,
                enabled: true,
            },
        ],
        &CompositionPolicy {
            strategy: CompositionStrategy::Additive,
            ..default()
        },
    );
    assert!((outcome.score - 0.75).abs() < 0.001);
}

#[test]
fn minimum_strategy_returns_smallest_score() {
    let outcome = compose_scores(
        &[
            ConsiderationOperand {
                score: 0.8,
                weight: 1.0,
                enabled: true,
            },
            ConsiderationOperand {
                score: 0.3,
                weight: 1.0,
                enabled: true,
            },
        ],
        &CompositionPolicy {
            strategy: CompositionStrategy::Minimum,
            ..default()
        },
    );
    assert!((outcome.score - 0.3).abs() < 0.001);
}

#[test]
fn compensated_product_stays_in_range() {
    let outcome = compose_scores(
        &[
            ConsiderationOperand {
                score: 0.2,
                weight: 1.0,
                enabled: true,
            },
            ConsiderationOperand {
                score: 0.9,
                weight: 1.0,
                enabled: true,
            },
        ],
        &CompositionPolicy {
            strategy: CompositionStrategy::CompensatedProduct {
                compensation_factor: 0.5,
            },
            ..default()
        },
    );
    assert!((0.0..=1.0).contains(&outcome.score));
}

#[test]
fn empty_operands_use_clamped_empty_score() {
    let outcome = compose_scores(
        &[],
        &CompositionPolicy {
            empty_score: 0.9,
            floor: 0.0,
            ceiling: 0.6,
            ..default()
        },
    );

    assert_eq!(outcome.score, 0.6);
    assert_eq!(outcome.evaluated_count, 0);
}

#[test]
fn weighted_score_sanitizes_invalid_inputs() {
    assert_eq!(weighted_score(f32::NAN, 2.0), 0.0);
    assert_eq!(weighted_score(0.5, -2.0), 1.0);
}

#[test]
fn geometric_mean_never_exceeds_arithmetic_mean() {
    let operands = [
        ConsiderationOperand {
            score: 0.2,
            weight: 1.0,
            enabled: true,
        },
        ConsiderationOperand {
            score: 0.8,
            weight: 1.0,
            enabled: true,
        },
        ConsiderationOperand {
            score: 0.5,
            weight: 1.0,
            enabled: true,
        },
    ];

    let outcome = compose_scores(&operands, &CompositionPolicy::default());
    let arithmetic_mean =
        operands.iter().map(|operand| operand.score).sum::<f32>() / operands.len() as f32;
    assert!(outcome.score <= arithmetic_mean + 0.0001);
}

#[test]
fn urgency_multiplier_is_bounded_by_floor_and_ceiling() {
    let outcome = compose_scores(
        &[ConsiderationOperand {
            score: 0.8,
            weight: 1.0,
            enabled: true,
        }],
        &CompositionPolicy {
            urgency_multiplier: 2.0,
            floor: 0.1,
            ceiling: 0.9,
            ..default()
        },
    );

    assert_eq!(outcome.score, 0.9);
}
