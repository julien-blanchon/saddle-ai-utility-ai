use bevy::prelude::*;

use crate::components::{ActionChangeReason, ActionLifecycle, ActionSuppressionReason, TargetKey};
use crate::selection::PriorityTier;

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct ConsiderationTrace {
    pub label: String,
    pub input: Option<f32>,
    pub output: f32,
    pub weighted_score: f32,
    pub enabled: bool,
    pub invalid_input: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct TargetCandidateTrace {
    pub label: String,
    pub entity: Option<Entity>,
    pub key: TargetKey,
    pub score: f32,
    pub enabled: bool,
    pub considerations: Vec<ConsiderationTrace>,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct ActionTrace {
    pub label: String,
    pub priority: PriorityTier,
    pub lifecycle: ActionLifecycle,
    pub base_score: f32,
    pub target_score: f32,
    pub final_score: f32,
    pub momentum_score: f32,
    pub suppression: Option<ActionSuppressionReason>,
    pub considerations: Vec<ConsiderationTrace>,
    pub target_candidates: Vec<TargetCandidateTrace>,
    pub selected_target: Option<TargetCandidateTrace>,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct ActionHistoryEntry {
    pub previous_action: Option<String>,
    pub next_action: Option<String>,
    pub reason: ActionChangeReason,
    pub changed_at_seconds: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct DecisionTrace {
    pub winning_action: Option<String>,
    pub winning_target: Option<TargetCandidateTrace>,
    pub evaluated_at_seconds: f32,
    pub next_evaluation_at_seconds: f32,
    pub requested: bool,
    pub due_to_budget: bool,
    pub switch_reason: Option<ActionChangeReason>,
    pub actions: Vec<ActionTrace>,
    pub recent_history: Vec<ActionHistoryEntry>,
}
