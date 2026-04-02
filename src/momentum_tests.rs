use super::*;

#[test]
fn active_action_bonus_increases_score() {
    let momentum = DecisionMomentum {
        active_action_bonus: 0.2,
        ..default()
    };
    let boosted = apply_active_bonus(0.4, true, 0.0, &momentum);
    assert!(boosted > 0.4);
}

#[test]
fn hysteresis_band_holds_small_improvements() {
    let momentum = DecisionMomentum {
        hysteresis_band: 0.1,
        ..default()
    };
    assert!(within_hysteresis_band(0.6, 0.68, &momentum));
    assert!(!within_hysteresis_band(0.6, 0.8, &momentum));
}

#[test]
fn inactive_action_keeps_score_unchanged() {
    let momentum = DecisionMomentum::default();
    assert_eq!(apply_active_bonus(0.4, false, 3.0, &momentum), 0.4);
}

#[test]
fn momentum_decay_reduces_bonus_over_time() {
    let momentum = DecisionMomentum {
        active_action_bonus: 0.2,
        momentum_decay_per_second: 2.0,
        ..default()
    };

    let early = apply_active_bonus(0.4, true, 0.0, &momentum);
    let late = apply_active_bonus(0.4, true, 2.0, &momentum);
    assert!(early > late);
    assert!(late > 0.4);
}
