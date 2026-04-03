use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub enum CompositionStrategy {
    Multiplicative,
    #[default]
    GeometricMean,
    Additive,
    Minimum,
    CompensatedProduct {
        compensation_factor: f32,
    },
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct CompositionPolicy {
    pub strategy: CompositionStrategy,
    pub floor: f32,
    pub ceiling: f32,
    pub urgency_multiplier: f32,
    pub empty_score: f32,
}

impl Default for CompositionPolicy {
    fn default() -> Self {
        Self {
            strategy: CompositionStrategy::GeometricMean,
            floor: 0.0,
            ceiling: 1.0,
            urgency_multiplier: 1.0,
            empty_score: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct ConsiderationOperand {
    pub score: f32,
    pub weight: f32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct CompositionOutcome {
    pub score: f32,
    pub evaluated_count: usize,
    pub zero_hit: bool,
}

pub fn compose_scores(
    operands: &[ConsiderationOperand],
    policy: &CompositionPolicy,
) -> CompositionOutcome {
    let enabled = operands
        .iter()
        .filter(|operand| operand.enabled)
        .map(|operand| weighted_score(operand.score, operand.weight))
        .collect::<Vec<_>>();

    if enabled.is_empty() {
        return CompositionOutcome {
            score: policy.empty_score.clamp(policy.floor, policy.ceiling),
            evaluated_count: 0,
            zero_hit: false,
        };
    }

    let zero_hit = enabled.iter().any(|score| *score <= 0.0);
    let raw_score = match &policy.strategy {
        CompositionStrategy::Multiplicative => enabled.iter().product::<f32>(),
        CompositionStrategy::GeometricMean => {
            let product = enabled.iter().product::<f32>();
            if product <= 0.0 {
                0.0
            } else {
                product.powf(1.0 / enabled.len() as f32)
            }
        }
        CompositionStrategy::Additive => {
            let total_weight = operands
                .iter()
                .filter(|operand| operand.enabled)
                .map(|operand| operand.weight.max(0.0))
                .sum::<f32>();
            if total_weight <= 0.0 {
                enabled.iter().copied().sum::<f32>() / enabled.len() as f32
            } else {
                operands
                    .iter()
                    .filter(|operand| operand.enabled)
                    .map(|operand| operand.score.clamp(0.0, 1.0) * operand.weight.max(0.0))
                    .sum::<f32>()
                    / total_weight
            }
        }
        CompositionStrategy::Minimum => enabled.into_iter().fold(1.0, f32::min),
        CompositionStrategy::CompensatedProduct {
            compensation_factor,
        } => compensated_product(&enabled, *compensation_factor),
    };

    let score = (raw_score * policy.urgency_multiplier.max(0.0)).clamp(
        policy.floor.min(policy.ceiling),
        policy.floor.max(policy.ceiling),
    );

    CompositionOutcome {
        score,
        evaluated_count: operands.iter().filter(|operand| operand.enabled).count(),
        zero_hit,
    }
}

pub fn weighted_score(score: f32, weight: f32) -> f32 {
    let clamped_score = if score.is_finite() {
        score.clamp(0.0, 1.0)
    } else {
        0.0
    };
    clamped_score.powf(weight.max(0.0))
}

fn compensated_product(scores: &[f32], compensation_factor: f32) -> f32 {
    if scores.is_empty() {
        return 0.0;
    }
    let mean = scores.iter().copied().sum::<f32>() / scores.len() as f32;
    let compensation = compensation_factor.clamp(0.0, 1.0);

    scores.iter().copied().fold(1.0, |acc, score| {
        let make_up = (1.0 - score) * compensation * (1.0 - mean);
        acc * (score + make_up).clamp(0.0, 1.0)
    })
}
