use bevy::prelude::*;

use crate::curves::ResponseCurve;
use crate::scoring::{CompositionPolicy, CompositionStrategy};
use crate::selection::{PriorityTier, SelectionStrategy};
use crate::tracing::{ActionHistoryEntry, ConsiderationTrace, DecisionTrace, TargetCandidateTrace};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub struct TargetKey(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum EvaluationMode {
    #[default]
    Interval,
    Manual,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum ConsiderationCost {
    #[default]
    Cheap,
    SharedCache,
    Expensive,
    TargetDependent,
}

impl ConsiderationCost {
    pub fn order(self) -> u8 {
        match self {
            Self::Cheap => 0,
            Self::SharedCache => 1,
            Self::Expensive => 2,
            Self::TargetDependent => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum TargetRequirement {
    #[default]
    Optional,
    Required,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum TargetScoreFold {
    #[default]
    Multiply,
    Minimum,
    Additive {
        weight: f32,
    },
    Ignore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum ActionLifecycle {
    #[default]
    Idle,
    Requested,
    Executing,
    Success,
    Failure,
    Cancelled,
}

impl ActionLifecycle {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failure | Self::Cancelled)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum ActionSuppressionReason {
    #[default]
    Disabled,
    Cooldown,
    NoTarget,
    BelowMinimumScore,
    ZeroScore,
    InvalidInput,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum ActionChangeReason {
    #[default]
    InitialSelection,
    BetterScore,
    ActionFinished,
    Cancelled,
    ExplicitRequest,
    HysteresisHold,
    CommitmentHold,
    NonInterruptible,
    NoBetterChoice,
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct UtilityAgent {
    pub enabled: bool,
    pub selection_strategy: SelectionStrategy,
    pub selection_seed: u64,
    pub trace_capacity: usize,
}

impl Default for UtilityAgent {
    fn default() -> Self {
        Self {
            enabled: true,
            selection_strategy: SelectionStrategy::HighestScore,
            selection_seed: 1,
            trace_capacity: 8,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct EvaluationPolicy {
    pub mode: EvaluationMode,
    pub base_interval_seconds: f32,
    pub jitter_fraction: f32,
    pub lod_scale: f32,
    pub pending_request: bool,
    pub last_evaluation_at_seconds: f32,
    pub next_evaluation_at_seconds: f32,
    pub evaluation_count: u64,
}

impl Default for EvaluationPolicy {
    fn default() -> Self {
        Self {
            mode: EvaluationMode::Interval,
            base_interval_seconds: 0.25,
            jitter_fraction: 0.0,
            lod_scale: 1.0,
            pending_request: true,
            last_evaluation_at_seconds: 0.0,
            next_evaluation_at_seconds: 0.0,
            evaluation_count: 0,
        }
    }
}

impl EvaluationPolicy {
    pub fn interval(seconds: f32) -> Self {
        Self {
            base_interval_seconds: seconds.max(0.0),
            ..default()
        }
    }

    pub fn manual() -> Self {
        Self {
            mode: EvaluationMode::Manual,
            ..default()
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct DecisionMomentum {
    pub active_action_bonus: f32,
    pub hysteresis_band: f32,
    pub momentum_decay_per_second: f32,
}

impl Default for DecisionMomentum {
    fn default() -> Self {
        Self {
            active_action_bonus: 0.1,
            hysteresis_band: 0.05,
            momentum_decay_per_second: 0.0,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct ActiveAction {
    pub entity: Option<Entity>,
    pub label: Option<String>,
    pub started_at_seconds: f32,
    pub last_changed_at_seconds: f32,
    pub switch_count: u32,
}

impl Default for ActiveAction {
    fn default() -> Self {
        Self {
            entity: None,
            label: None,
            started_at_seconds: 0.0,
            last_changed_at_seconds: 0.0,
            switch_count: 0,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct UtilityAction {
    pub label: String,
    pub priority: PriorityTier,
    pub priority_threshold: f32,
    pub enabled: bool,
    pub interruptible: bool,
    pub minimum_commitment_seconds: f32,
    pub weight: f32,
    pub minimum_score: f32,
    pub composition: CompositionPolicy,
    pub lifecycle: ActionLifecycle,
    pub target_requirement: TargetRequirement,
    pub target_selection: SelectionStrategy,
    pub target_score_fold: TargetScoreFold,
    pub is_fallback: bool,
}

impl Default for UtilityAction {
    fn default() -> Self {
        Self {
            label: "action".into(),
            priority: PriorityTier::TACTICAL,
            priority_threshold: 0.0,
            enabled: true,
            interruptible: true,
            minimum_commitment_seconds: 0.0,
            weight: 1.0,
            minimum_score: 0.0,
            composition: CompositionPolicy::default(),
            lifecycle: ActionLifecycle::Idle,
            target_requirement: TargetRequirement::Optional,
            target_selection: SelectionStrategy::HighestScore,
            target_score_fold: TargetScoreFold::Multiply,
            is_fallback: false,
        }
    }
}

impl UtilityAction {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..default()
        }
    }

    pub fn with_priority(mut self, priority: PriorityTier, threshold: f32) -> Self {
        self.priority = priority;
        self.priority_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_composition(mut self, strategy: CompositionStrategy) -> Self {
        self.composition.strategy = strategy;
        self
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct ActionScore {
    pub base_score: f32,
    pub target_score: f32,
    pub final_score: f32,
    pub momentum_score: f32,
    pub suppression: Option<ActionSuppressionReason>,
    pub last_evaluated_at_seconds: f32,
    pub considerations: Vec<ConsiderationTrace>,
    pub target_candidates: Vec<TargetCandidateTrace>,
}

impl Default for ActionScore {
    fn default() -> Self {
        Self {
            base_score: 0.0,
            target_score: 0.0,
            final_score: 0.0,
            momentum_score: 0.0,
            suppression: None,
            last_evaluated_at_seconds: 0.0,
            considerations: Vec::new(),
            target_candidates: Vec::new(),
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct ActionCooldown {
    pub duration_seconds: f32,
    pub remaining_seconds: f32,
    pub restart_on_success: bool,
    pub restart_on_failure: bool,
    pub restart_on_cancel: bool,
}

impl Default for ActionCooldown {
    fn default() -> Self {
        Self {
            duration_seconds: 0.0,
            remaining_seconds: 0.0,
            restart_on_success: true,
            restart_on_failure: true,
            restart_on_cancel: true,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct ActionTarget {
    pub entity: Option<Entity>,
    pub key: Option<TargetKey>,
    pub label: Option<String>,
    pub score: f32,
}

impl Default for ActionTarget {
    fn default() -> Self {
        Self {
            entity: None,
            key: None,
            label: None,
            score: 0.0,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct UtilityTargetCandidate {
    pub label: String,
    pub entity: Option<Entity>,
    pub key: TargetKey,
    pub enabled: bool,
    pub weight: f32,
    pub last_score: f32,
}

impl Default for UtilityTargetCandidate {
    fn default() -> Self {
        Self {
            label: "target".into(),
            entity: None,
            key: TargetKey(0),
            enabled: true,
            weight: 1.0,
            last_score: 0.0,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct UtilityConsideration {
    pub label: String,
    #[reflect(ignore)]
    pub curve: ResponseCurve,
    pub weight: f32,
    pub enabled: bool,
    pub cost: ConsiderationCost,
}

impl Default for UtilityConsideration {
    fn default() -> Self {
        Self {
            label: "consideration".into(),
            curve: ResponseCurve::Linear,
            weight: 1.0,
            enabled: true,
            cost: ConsiderationCost::Cheap,
        }
    }
}

impl UtilityConsideration {
    pub fn new(label: impl Into<String>, curve: ResponseCurve) -> Self {
        Self {
            label: label.into(),
            curve,
            ..default()
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct ConsiderationInput {
    pub value: Option<f32>,
    pub enabled: bool,
}

impl Default for ConsiderationInput {
    fn default() -> Self {
        Self {
            value: Some(0.0),
            enabled: true,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct DecisionTraceBuffer {
    pub last: Option<DecisionTrace>,
    pub history: Vec<ActionHistoryEntry>,
    pub capacity: usize,
}

impl Default for DecisionTraceBuffer {
    fn default() -> Self {
        Self {
            last: None,
            history: Vec::new(),
            capacity: 8,
        }
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct UtilityAiBudget {
    pub max_agents_per_update: usize,
}

impl Default for UtilityAiBudget {
    fn default() -> Self {
        Self {
            max_agents_per_update: 128,
        }
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct UtilityAiStats {
    pub evaluated_agents: usize,
    pub scored_actions: usize,
    pub scored_targets: usize,
    pub skipped_due_to_budget: usize,
    pub last_evaluation_time_micros: u64,
    pub peak_evaluation_time_micros: u64,
    pub average_evaluation_time_micros: f32,
}

impl Default for UtilityAiStats {
    fn default() -> Self {
        Self {
            evaluated_agents: 0,
            scored_actions: 0,
            scored_targets: 0,
            skipped_due_to_budget: 0,
            last_evaluation_time_micros: 0,
            peak_evaluation_time_micros: 0,
            average_evaluation_time_micros: 0.0,
        }
    }
}
