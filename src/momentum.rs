use crate::components::DecisionMomentum;

pub fn apply_active_bonus(
    score: f32,
    is_active: bool,
    active_for_seconds: f32,
    momentum: &DecisionMomentum,
) -> f32 {
    if !is_active {
        return score.clamp(0.0, 1.0);
    }

    let decay = if momentum.momentum_decay_per_second <= 0.0 {
        1.0
    } else {
        (-momentum.momentum_decay_per_second * active_for_seconds.max(0.0)).exp()
    };

    (score + momentum.active_action_bonus.max(0.0) * decay).clamp(0.0, 1.0)
}

pub fn within_hysteresis_band(
    current_score: f32,
    candidate_score: f32,
    momentum: &DecisionMomentum,
) -> bool {
    candidate_score <= current_score + momentum.hysteresis_band.max(0.0)
}
