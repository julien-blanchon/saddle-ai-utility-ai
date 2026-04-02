#![doc = include_str!("../README.md")]

mod components;
mod curves;
mod messages;
mod momentum;
mod scoring;
mod selection;
mod systems;
mod tracing;

pub use components::{
    ActionChangeReason, ActionCooldown, ActionLifecycle, ActionScore, ActionSuppressionReason,
    ActionTarget, ActiveAction, ConsiderationCost, ConsiderationInput, DecisionMomentum,
    DecisionTraceBuffer, EvaluationMode, EvaluationPolicy, TargetKey, TargetRequirement,
    TargetScoreFold, UtilityAction, UtilityAgent, UtilityAiBudget, UtilityAiStats,
    UtilityConsideration, UtilityTargetCandidate,
};
pub use curves::{CurveEvaluation, ResponseCurve};
pub use messages::{ActionChanged, ActionCompleted, ActionEvaluationRequested};
pub use momentum::{apply_active_bonus, within_hysteresis_band};
pub use scoring::{
    CompositionOutcome, CompositionPolicy, CompositionStrategy, ConsiderationOperand,
    compose_scores, weighted_score,
};
pub use selection::{PriorityTier, SelectionStrategy, select_index};
pub use tracing::{
    ActionHistoryEntry, ActionTrace, ConsiderationTrace, DecisionTrace, TargetCandidateTrace,
};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UtilityAiSystems {
    GatherInputs,
    Score,
    Select,
    Transition,
    DebugRender,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct UtilityAiPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl UtilityAiPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for UtilityAiPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for UtilityAiPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_resource::<UtilityAiBudget>()
            .init_resource::<UtilityAiStats>()
            .init_resource::<systems::EvaluationBatch>()
            .add_message::<ActionChanged>()
            .add_message::<ActionCompleted>()
            .add_message::<ActionEvaluationRequested>()
            .register_type::<ActionChangeReason>()
            .register_type::<ActionCooldown>()
            .register_type::<ActionLifecycle>()
            .register_type::<ActionScore>()
            .register_type::<ActionSuppressionReason>()
            .register_type::<ActionTarget>()
            .register_type::<ActiveAction>()
            .register_type::<ConsiderationCost>()
            .register_type::<ConsiderationInput>()
            .register_type::<ConsiderationOperand>()
            .register_type::<ConsiderationTrace>()
            .register_type::<CurveEvaluation>()
            .register_type::<DecisionMomentum>()
            .register_type::<DecisionTrace>()
            .register_type::<DecisionTraceBuffer>()
            .register_type::<EvaluationMode>()
            .register_type::<EvaluationPolicy>()
            .register_type::<PriorityTier>()
            .register_type::<SelectionStrategy>()
            .register_type::<CompositionOutcome>()
            .register_type::<CompositionPolicy>()
            .register_type::<CompositionStrategy>()
            .register_type::<TargetCandidateTrace>()
            .register_type::<TargetKey>()
            .register_type::<TargetRequirement>()
            .register_type::<TargetScoreFold>()
            .register_type::<UtilityAction>()
            .register_type::<UtilityAgent>()
            .register_type::<UtilityAiBudget>()
            .register_type::<UtilityAiStats>()
            .register_type::<UtilityConsideration>()
            .register_type::<UtilityTargetCandidate>()
            .configure_sets(
                self.update_schedule,
                (
                    UtilityAiSystems::GatherInputs,
                    UtilityAiSystems::Score,
                    UtilityAiSystems::Select,
                    UtilityAiSystems::Transition,
                    UtilityAiSystems::DebugRender,
                )
                    .chain(),
            )
            .add_systems(self.activate_schedule, systems::bootstrap_agents)
            .add_systems(self.activate_schedule, systems::bootstrap_actions)
            .add_systems(
                self.update_schedule,
                (
                    systems::bootstrap_agents,
                    systems::bootstrap_actions,
                    systems::apply_evaluation_requests,
                    systems::tick_cooldowns,
                    systems::collect_due_agents,
                )
                    .chain()
                    .in_set(UtilityAiSystems::GatherInputs),
            )
            .add_systems(
                self.update_schedule,
                systems::score_due_agents.in_set(UtilityAiSystems::Score),
            )
            .add_systems(
                self.update_schedule,
                systems::select_due_agents.in_set(UtilityAiSystems::Select),
            )
            .add_systems(
                self.update_schedule,
                systems::transition_due_agents.in_set(UtilityAiSystems::Transition),
            )
            .add_systems(
                self.update_schedule,
                systems::refresh_debug_buffers.in_set(UtilityAiSystems::DebugRender),
            );
    }
}

#[cfg(test)]
#[path = "curves_tests.rs"]
mod curves_tests;

#[cfg(test)]
#[path = "momentum_tests.rs"]
mod momentum_tests;

#[cfg(test)]
#[path = "scoring_tests.rs"]
mod scoring_tests;

#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;

#[cfg(test)]
#[path = "systems_tests.rs"]
mod systems_tests;

#[cfg(test)]
#[path = "tracing_tests.rs"]
mod tracing_tests;
