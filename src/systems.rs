use std::time::Instant;

use bevy::prelude::*;

use crate::components::{
    ActionChangeReason, ActionCooldown, ActionLifecycle, ActionScore, ActionSuppressionReason,
    ActionTarget, ActiveAction, ConsiderationInput, DecisionMomentum, DecisionTraceBuffer,
    EvaluationMode, EvaluationPolicy, TargetRequirement, TargetScoreFold, UtilityAction,
    UtilityAgent, UtilityAiBudget, UtilityAiStats, UtilityConsideration, UtilityTargetCandidate,
};
use crate::curves::CurveEvaluation;
use crate::messages::{ActionChanged, ActionCompleted, ActionEvaluationRequested};
use crate::momentum::{apply_active_bonus, within_hysteresis_band};
use crate::scoring::{ConsiderationOperand, compose_scores};
use crate::selection::{select_index, unit_from_seed};
use crate::tracing::{
    ActionHistoryEntry, ActionTrace, ConsiderationTrace, DecisionTrace, TargetCandidateTrace,
};

#[derive(Resource, Default)]
pub(crate) struct EvaluationBatch {
    pub due_agents: Vec<Entity>,
    pub skipped_due_to_budget: usize,
    pub scored_actions: usize,
    pub scored_targets: usize,
    pub started_at: Option<Instant>,
}

#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PendingSelection {
    pub selected_action: Option<Entity>,
    pub selected_score: f32,
}

#[allow(clippy::type_complexity)]
pub(crate) fn bootstrap_agents(
    mut commands: Commands,
    agents: Query<
        (
            Entity,
            &UtilityAgent,
            Option<&EvaluationPolicy>,
            Option<&ActiveAction>,
            Option<&DecisionMomentum>,
            Option<&DecisionTraceBuffer>,
        ),
        Added<UtilityAgent>,
    >,
) {
    for (entity, agent, policy, active, momentum, trace) in &agents {
        let mut entity_commands = commands.entity(entity);
        if policy.is_none() {
            entity_commands.insert(EvaluationPolicy::default());
        }
        if active.is_none() {
            entity_commands.insert(ActiveAction::default());
        }
        if momentum.is_none() {
            entity_commands.insert(DecisionMomentum::default());
        }
        if trace.is_none() {
            entity_commands.insert(DecisionTraceBuffer {
                capacity: agent.trace_capacity,
                ..default()
            });
        }
    }
}

pub(crate) fn bootstrap_actions(
    mut commands: Commands,
    actions: Query<
        (
            Entity,
            Option<&ActionScore>,
            Option<&ActionCooldown>,
            Option<&ActionTarget>,
        ),
        Added<UtilityAction>,
    >,
) {
    for (entity, score, cooldown, target) in &actions {
        let mut entity_commands = commands.entity(entity);
        if score.is_none() {
            entity_commands.insert(ActionScore::default());
        }
        if cooldown.is_none() {
            entity_commands.insert(ActionCooldown::default());
        }
        if target.is_none() {
            entity_commands.insert(ActionTarget::default());
        }
    }
}

pub(crate) fn apply_evaluation_requests(
    mut requests: MessageReader<ActionEvaluationRequested>,
    mut policies: Query<&mut EvaluationPolicy>,
) {
    for request in requests.read() {
        if let Ok(mut policy) = policies.get_mut(request.agent) {
            policy.pending_request = true;
        }
    }
}

pub(crate) fn tick_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut ActionCooldown>) {
    let delta = time.delta_secs();
    if delta <= 0.0 {
        return;
    }

    for mut cooldown in &mut cooldowns {
        cooldown.remaining_seconds = (cooldown.remaining_seconds - delta).max(0.0);
    }
}

pub(crate) fn collect_due_agents(
    time: Res<Time>,
    budget: Res<UtilityAiBudget>,
    mut batch: ResMut<EvaluationBatch>,
    agents: Query<(Entity, &UtilityAgent, &EvaluationPolicy)>,
) {
    batch.due_agents.clear();
    batch.skipped_due_to_budget = 0;
    batch.scored_actions = 0;
    batch.scored_targets = 0;
    batch.started_at = None;

    let now = time.elapsed_secs();
    let mut due = agents
        .iter()
        .filter_map(|(entity, agent, policy)| {
            if !agent.enabled {
                return None;
            }

            let is_due = policy.pending_request
                || (policy.mode == EvaluationMode::Interval
                    && now >= policy.next_evaluation_at_seconds);
            is_due.then_some((entity, policy.next_evaluation_at_seconds))
        })
        .collect::<Vec<_>>();

    due.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
    });

    batch.skipped_due_to_budget = due.len().saturating_sub(budget.max_agents_per_update);
    batch.due_agents.extend(
        due.into_iter()
            .take(budget.max_agents_per_update)
            .map(|(entity, _)| entity),
    );
}

pub(crate) fn score_due_agents(
    time: Res<Time>,
    mut batch: ResMut<EvaluationBatch>,
    agents: Query<(&DecisionMomentum, &ActiveAction), With<UtilityAgent>>,
    agent_children: Query<&Children, With<UtilityAgent>>,
    mut actions: Query<(
        Entity,
        &UtilityAction,
        &mut ActionScore,
        &mut ActionTarget,
        &ActionCooldown,
        Option<&Children>,
    )>,
    mut targets: Query<(&mut UtilityTargetCandidate, Option<&Children>)>,
    considerations: Query<(&UtilityConsideration, &ConsiderationInput)>,
) {
    if batch.due_agents.is_empty() {
        return;
    }
    if batch.started_at.is_none() {
        batch.started_at = Some(Instant::now());
    }

    let now = time.elapsed_secs();
    let due_agents = batch.due_agents.clone();

    for agent_entity in due_agents {
        let Ok((momentum, active)) = agents.get(agent_entity) else {
            continue;
        };
        let Ok(children) = agent_children.get(agent_entity) else {
            continue;
        };
        let active_for_seconds = if active.entity.is_some() {
            (now - active.started_at_seconds).max(0.0)
        } else {
            0.0
        };

        for action_entity in children.iter() {
            let Ok((_, action, mut score, mut target, cooldown, action_children)) =
                actions.get_mut(action_entity)
            else {
                continue;
            };

            let (consideration_traces, base_score) =
                score_considerations(action_children, &considerations, &action.composition);

            let target_seed = agent_entity.to_bits() ^ action_entity.to_bits();
            let (target_traces, selected_target, target_score) = score_targets(
                action_children,
                &mut targets,
                &considerations,
                &action.composition,
                &action.target_selection,
                target_seed,
            );

            let folded_score =
                fold_target_score(base_score, target_score, action.target_score_fold);
            let final_score = (folded_score * action.weight.max(0.0)).clamp(0.0, 1.0);
            let suppression = action_suppression_reason(
                action,
                cooldown,
                !target_traces.is_empty(),
                selected_target.is_none(),
                final_score,
            );
            let momentum_score = if suppression.is_none() {
                apply_active_bonus(
                    final_score,
                    active.entity == Some(action_entity),
                    active_for_seconds,
                    momentum,
                )
            } else {
                0.0
            };

            score.base_score = base_score;
            score.target_score = target_score;
            score.final_score = final_score;
            score.momentum_score = momentum_score;
            score.suppression = suppression;
            score.last_evaluated_at_seconds = now;
            score.considerations = consideration_traces;
            score.target_candidates = target_traces;

            if let Some(target_trace) = selected_target {
                target.entity = target_trace.entity;
                target.key = Some(target_trace.key);
                target.label = Some(target_trace.label.clone());
                target.score = target_trace.score;
            } else {
                *target = ActionTarget::default();
            }

            batch.scored_actions += 1;
            batch.scored_targets += score.target_candidates.len();
        }
    }
}

pub(crate) fn select_due_agents(
    mut commands: Commands,
    batch: Res<EvaluationBatch>,
    agents: Query<(&UtilityAgent, &EvaluationPolicy)>,
    agent_children: Query<&Children, With<UtilityAgent>>,
    actions: Query<(&UtilityAction, &ActionScore)>,
) {
    for agent_entity in batch.due_agents.iter().copied() {
        let Ok((agent, policy)) = agents.get(agent_entity) else {
            continue;
        };
        let Ok(children) = agent_children.get(agent_entity) else {
            continue;
        };

        let mut eligible = Vec::new();
        let mut fallback = None;

        for action_entity in children.iter() {
            let Ok((action, score)) = actions.get(action_entity) else {
                continue;
            };

            if action.is_fallback && fallback.is_none() && score.suppression.is_none() {
                fallback = Some(action_entity);
            }

            if score.suppression.is_none() && score.momentum_score > 0.0 {
                eligible.push((
                    action_entity,
                    action.priority.0,
                    action.priority_threshold,
                    score.momentum_score,
                ));
            }
        }

        eligible.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
        });

        let mut pool = Vec::new();
        for tier in eligible.iter().map(|entry| entry.1).collect::<Vec<_>>() {
            let tier_entries = eligible
                .iter()
                .copied()
                .filter(|entry| entry.1 == tier)
                .collect::<Vec<_>>();
            if tier_entries.is_empty() {
                continue;
            }
            if tier_entries
                .iter()
                .any(|entry| entry.3 >= entry.2.clamp(0.0, 1.0))
            {
                pool = tier_entries;
                break;
            }
        }

        if pool.is_empty() {
            pool = eligible;
        }

        let selected_action = if pool.is_empty() {
            fallback
        } else {
            let scores = pool.iter().map(|entry| entry.3).collect::<Vec<_>>();
            let seed = agent.selection_seed ^ policy.evaluation_count;
            select_index(&scores, &agent.selection_strategy, seed)
                .and_then(|index| pool.get(index).map(|entry| entry.0))
        };

        let selected_score = selected_action
            .and_then(|entity| {
                actions
                    .get(entity)
                    .ok()
                    .map(|(_, score)| score.momentum_score)
            })
            .unwrap_or(0.0);

        commands.entity(agent_entity).insert(PendingSelection {
            selected_action,
            selected_score,
        });
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn transition_due_agents(
    mut commands: Commands,
    time: Res<Time>,
    batch: Res<EvaluationBatch>,
    mut stats: ResMut<UtilityAiStats>,
    mut changed_writer: MessageWriter<ActionChanged>,
    mut completed_writer: MessageWriter<ActionCompleted>,
    mut agents: Query<(
        &UtilityAgent,
        &mut EvaluationPolicy,
        &mut ActiveAction,
        &mut DecisionTraceBuffer,
        &DecisionMomentum,
    )>,
    agent_children: Query<&Children, With<UtilityAgent>>,
    pending: Query<&PendingSelection>,
    mut action_set: ParamSet<(
        Query<(&mut UtilityAction, &mut ActionCooldown)>,
        Query<(&UtilityAction, &ActionScore, &ActionTarget)>,
    )>,
) {
    let now = time.elapsed_secs();

    let due_agents = batch.due_agents.clone();

    for agent_entity in due_agents {
        let Ok((agent, mut policy, mut active, mut trace_buffer, momentum)) =
            agents.get_mut(agent_entity)
        else {
            continue;
        };
        let Ok(children) = agent_children.get(agent_entity) else {
            continue;
        };

        trace_buffer.capacity = agent.trace_capacity;
        let pending = pending.get(agent_entity).ok();
        let requested = policy.pending_request;

        let previous_entity = active.entity;
        let previous_label = active.label.clone();
        let mut switch_reason = None;

        if let Some(current_entity) = active.entity {
            if let Ok((current_action, _, _)) = action_set.p1().get(current_entity) {
                if current_action.lifecycle.is_terminal() {
                    completed_writer.write(ActionCompleted {
                        agent: agent_entity,
                        action: current_entity,
                        label: current_action.label.clone(),
                        lifecycle: current_action.lifecycle,
                    });

                    if let Ok((mut action, mut cooldown)) = action_set.p0().get_mut(current_entity)
                    {
                        maybe_restart_cooldown(&mut cooldown, action.lifecycle);
                        action.lifecycle = ActionLifecycle::Idle;
                    }

                    active.entity = None;
                    active.label = None;
                    switch_reason = Some(ActionChangeReason::ActionFinished);
                }
            }
        }

        let selected_entity = pending.and_then(|pending| pending.selected_action);
        let selected_score = pending.map_or(0.0, |pending| pending.selected_score);
        let current_score = active
            .entity
            .and_then(|entity| {
                action_set
                    .p1()
                    .get(entity)
                    .ok()
                    .map(|(_, score, _)| score.momentum_score)
            })
            .unwrap_or(0.0);

        let mut next_entity = active.entity;
        let mut next_label = active.label.clone();

        match (active.entity, selected_entity) {
            (Some(current), Some(selected)) if current == selected => {}
            (Some(current), Some(selected)) => {
                let (current_interruptible, current_minimum_commitment_seconds) = {
                    let action_read = action_set.p1();
                    let Ok((current_action, _, _)) = action_read.get(current) else {
                        continue;
                    };
                    (
                        current_action.interruptible,
                        current_action.minimum_commitment_seconds,
                    )
                };
                let active_for_seconds = (now - active.started_at_seconds).max(0.0);
                let should_hold = if !current_interruptible {
                    switch_reason = Some(ActionChangeReason::NonInterruptible);
                    true
                } else if active_for_seconds < current_minimum_commitment_seconds {
                    switch_reason = Some(ActionChangeReason::CommitmentHold);
                    true
                } else if within_hysteresis_band(current_score, selected_score, momentum) {
                    switch_reason = Some(ActionChangeReason::HysteresisHold);
                    true
                } else {
                    false
                };

                if !should_hold {
                    if let Ok((mut action, mut cooldown)) = action_set.p0().get_mut(current) {
                        action.lifecycle = ActionLifecycle::Cancelled;
                        maybe_restart_cooldown(&mut cooldown, ActionLifecycle::Cancelled);
                    }
                    request_action(&mut action_set.p0(), selected);

                    next_entity = Some(selected);
                    next_label = action_set
                        .p1()
                        .get(selected)
                        .ok()
                        .map(|(action, _, _)| action.label.clone());
                    switch_reason = Some(ActionChangeReason::BetterScore);
                }
            }
            (Some(_), None) => {
                switch_reason.get_or_insert(ActionChangeReason::NoBetterChoice);
            }
            (None, Some(selected)) => {
                request_action(&mut action_set.p0(), selected);
                next_entity = Some(selected);
                next_label = action_set
                    .p1()
                    .get(selected)
                    .ok()
                    .map(|(action, _, _)| action.label.clone());
                switch_reason.get_or_insert(ActionChangeReason::InitialSelection);
            }
            (None, None) => {}
        }

        if next_entity != previous_entity {
            active.entity = next_entity;
            active.label = next_label.clone();
            active.started_at_seconds = now;
            active.last_changed_at_seconds = now;
            active.switch_count = active.switch_count.saturating_add(1);

            let reason = switch_reason.unwrap_or(ActionChangeReason::InitialSelection);
            changed_writer.write(ActionChanged {
                agent: agent_entity,
                previous_action: previous_entity,
                next_action: next_entity,
                previous_label: previous_label.clone(),
                next_label: next_label.clone(),
                reason,
            });

            push_history(
                &mut trace_buffer,
                previous_label,
                next_label.clone(),
                reason,
                now,
            );
        }

        policy.pending_request = false;
        policy.last_evaluation_at_seconds = now;
        policy.evaluation_count = policy.evaluation_count.saturating_add(1);
        policy.next_evaluation_at_seconds = match policy.mode {
            EvaluationMode::Manual => f32::INFINITY,
            EvaluationMode::Interval => {
                now + jittered_interval(
                    policy.base_interval_seconds,
                    policy.jitter_fraction,
                    policy.lod_scale,
                    agent.selection_seed ^ policy.evaluation_count,
                )
            }
        };

        trace_buffer.last = Some(build_trace(
            children,
            &action_set.p1(),
            &trace_buffer.history,
            next_entity,
            switch_reason,
            now,
            policy.next_evaluation_at_seconds,
            requested,
            batch.skipped_due_to_budget > 0,
        ));

        commands.entity(agent_entity).remove::<PendingSelection>();
    }

    let elapsed_micros = batch
        .started_at
        .map(|started_at| started_at.elapsed().as_micros() as u64)
        .unwrap_or(0);
    stats.evaluated_agents = batch.due_agents.len();
    stats.scored_actions = batch.scored_actions;
    stats.scored_targets = batch.scored_targets;
    stats.skipped_due_to_budget = batch.skipped_due_to_budget;
    stats.last_evaluation_time_micros = elapsed_micros;
    stats.peak_evaluation_time_micros = stats.peak_evaluation_time_micros.max(elapsed_micros);
    stats.average_evaluation_time_micros =
        (stats.average_evaluation_time_micros * 0.8) + (elapsed_micros as f32 * 0.2);
}

pub(crate) fn refresh_debug_buffers(mut buffers: Query<(&UtilityAgent, &mut DecisionTraceBuffer)>) {
    for (agent, mut buffer) in &mut buffers {
        buffer.capacity = agent.trace_capacity;
        trim_history(&mut buffer);
    }
}

fn score_considerations(
    children: Option<&Children>,
    considerations: &Query<(&UtilityConsideration, &ConsiderationInput)>,
    composition: &crate::scoring::CompositionPolicy,
) -> (Vec<ConsiderationTrace>, f32) {
    let Some(children) = children else {
        return (Vec::new(), composition.empty_score);
    };

    let mut entries = children
        .iter()
        .enumerate()
        .filter_map(|(order, entity)| {
            considerations
                .get(entity)
                .ok()
                .map(|(consideration, input)| (order, consideration.clone(), input.clone()))
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        left.1
            .cost
            .order()
            .cmp(&right.1.cost.order())
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut operands = Vec::with_capacity(entries.len());
    let mut traces = Vec::with_capacity(entries.len());

    for (_, consideration, input) in entries {
        let (trace, operand) = evaluate_consideration(consideration, input);
        traces.push(trace);
        operands.push(operand);

        if should_short_circuit_considerations(composition, operands.last()) {
            break;
        }
    }

    let outcome = compose_scores(&operands, composition);
    (traces, outcome.score)
}

fn score_targets(
    children: Option<&Children>,
    targets: &mut Query<(&mut UtilityTargetCandidate, Option<&Children>)>,
    considerations: &Query<(&UtilityConsideration, &ConsiderationInput)>,
    composition: &crate::scoring::CompositionPolicy,
    selection: &crate::selection::SelectionStrategy,
    seed: u64,
) -> (Vec<TargetCandidateTrace>, Option<TargetCandidateTrace>, f32) {
    let Some(children) = children else {
        return (Vec::new(), None, 1.0);
    };

    let mut traces = Vec::new();
    let mut scores = Vec::new();

    for child in children.iter() {
        let Ok((mut target, target_children)) = targets.get_mut(child) else {
            continue;
        };

        let (consideration_traces, raw_score) =
            score_considerations(target_children, considerations, composition);
        let score = if consideration_traces.is_empty() {
            target.weight.clamp(0.0, 1.0)
        } else {
            (raw_score * target.weight.max(0.0)).clamp(0.0, 1.0)
        };
        target.last_score = score;
        scores.push(score);
        traces.push(TargetCandidateTrace {
            label: target.label.clone(),
            entity: target.entity,
            key: target.key,
            score,
            enabled: target.enabled,
            considerations: consideration_traces,
        });
    }

    if traces.is_empty() {
        return (traces, None, 1.0);
    }

    let selected = select_index(&scores, selection, seed)
        .and_then(|index| traces.get(index).cloned())
        .filter(|trace| trace.enabled && trace.score > 0.0);
    let selected_score = selected.as_ref().map_or(0.0, |trace| trace.score);
    (traces, selected, selected_score)
}

fn evaluate_consideration(
    consideration: UtilityConsideration,
    input: ConsiderationInput,
) -> (ConsiderationTrace, ConsiderationOperand) {
    if !consideration.enabled || !input.enabled {
        return (
            ConsiderationTrace {
                label: consideration.label,
                enabled: false,
                ..default()
            },
            ConsiderationOperand {
                enabled: false,
                ..default()
            },
        );
    }

    let Some(raw_input) = input.value else {
        return (
            ConsiderationTrace {
                label: consideration.label,
                input: None,
                enabled: false,
                ..default()
            },
            ConsiderationOperand {
                enabled: false,
                ..default()
            },
        );
    };

    let CurveEvaluation {
        output,
        invalid_input,
        ..
    } = consideration.curve.evaluate(raw_input);
    let weighted = crate::scoring::weighted_score(output, consideration.weight);

    (
        ConsiderationTrace {
            label: consideration.label,
            input: Some(raw_input.clamp(0.0, 1.0)),
            output,
            weighted_score: weighted,
            enabled: true,
            invalid_input,
        },
        ConsiderationOperand {
            score: output,
            weight: consideration.weight,
            enabled: !invalid_input,
        },
    )
}

fn should_short_circuit_considerations(
    composition: &crate::scoring::CompositionPolicy,
    operand: Option<&ConsiderationOperand>,
) -> bool {
    matches!(
        composition.strategy,
        crate::scoring::CompositionStrategy::Multiplicative
    ) && operand.is_some_and(|operand| operand.enabled && operand.score <= 0.0)
}

fn action_suppression_reason(
    action: &UtilityAction,
    cooldown: &ActionCooldown,
    has_target_candidates: bool,
    missing_selected_target: bool,
    score: f32,
) -> Option<ActionSuppressionReason> {
    if !action.enabled {
        Some(ActionSuppressionReason::Disabled)
    } else if cooldown.remaining_seconds > 0.0 {
        Some(ActionSuppressionReason::Cooldown)
    } else if action.target_requirement == TargetRequirement::Required
        && (!has_target_candidates || missing_selected_target)
    {
        Some(ActionSuppressionReason::NoTarget)
    } else if !score.is_finite() {
        Some(ActionSuppressionReason::InvalidInput)
    } else if score <= 0.0 && !action.is_fallback {
        Some(ActionSuppressionReason::ZeroScore)
    } else if score < action.minimum_score {
        Some(ActionSuppressionReason::BelowMinimumScore)
    } else {
        None
    }
}

fn fold_target_score(base_score: f32, target_score: f32, fold: TargetScoreFold) -> f32 {
    match fold {
        TargetScoreFold::Multiply => {
            if target_score <= 0.0 {
                base_score * target_score.max(0.0)
            } else {
                base_score * target_score
            }
        }
        TargetScoreFold::Minimum => base_score.min(target_score),
        TargetScoreFold::Additive { weight } => ((base_score + target_score * weight.max(0.0))
            / (1.0 + weight.max(0.0)))
        .clamp(0.0, 1.0),
        TargetScoreFold::Ignore => base_score,
    }
}

fn maybe_restart_cooldown(cooldown: &mut ActionCooldown, lifecycle: ActionLifecycle) {
    let should_restart = match lifecycle {
        ActionLifecycle::Success => cooldown.restart_on_success,
        ActionLifecycle::Failure => cooldown.restart_on_failure,
        ActionLifecycle::Cancelled => cooldown.restart_on_cancel,
        _ => false,
    };
    if should_restart {
        cooldown.remaining_seconds = cooldown.duration_seconds.max(0.0);
    }
}

fn request_action(actions: &mut Query<(&mut UtilityAction, &mut ActionCooldown)>, entity: Entity) {
    if let Ok((mut action, _)) = actions.get_mut(entity) {
        action.lifecycle = ActionLifecycle::Requested;
    }
}

fn jittered_interval(base_interval: f32, jitter_fraction: f32, lod_scale: f32, seed: u64) -> f32 {
    let base = (base_interval.max(0.0) * lod_scale.max(0.0)).max(0.0);
    if base <= 0.0 {
        return 0.0;
    }
    let jitter = base * jitter_fraction.clamp(0.0, 1.0);
    let centered = (unit_from_seed(seed) * 2.0) - 1.0;
    (base + centered * jitter).max(0.0)
}

#[allow(clippy::too_many_arguments)]
fn build_trace(
    children: &Children,
    actions: &Query<(&UtilityAction, &ActionScore, &ActionTarget)>,
    history: &[ActionHistoryEntry],
    active_entity: Option<Entity>,
    switch_reason: Option<ActionChangeReason>,
    now: f32,
    next_evaluation_at_seconds: f32,
    requested: bool,
    due_to_budget: bool,
) -> DecisionTrace {
    let action_traces = children
        .iter()
        .filter_map(|entity| {
            actions.get(entity).ok().map(|(action, score, target)| {
                let selected_target = build_selected_target_trace(score, target);
                ActionTrace {
                    label: action.label.clone(),
                    priority: action.priority,
                    lifecycle: action.lifecycle,
                    base_score: score.base_score,
                    target_score: score.target_score,
                    final_score: score.final_score,
                    momentum_score: score.momentum_score,
                    suppression: score.suppression,
                    considerations: score.considerations.clone(),
                    target_candidates: score.target_candidates.clone(),
                    selected_target,
                }
            })
        })
        .collect::<Vec<_>>();

    let winning_target = active_entity
        .and_then(|entity| actions.get(entity).ok())
        .and_then(|(_, score, target)| build_selected_target_trace(score, target));

    let winning_action = active_entity
        .and_then(|entity| actions.get(entity).ok())
        .map(|(action, _, _)| action.label.clone());

    DecisionTrace {
        winning_action,
        winning_target,
        evaluated_at_seconds: now,
        next_evaluation_at_seconds,
        requested,
        due_to_budget,
        switch_reason,
        actions: action_traces,
        recent_history: history.to_vec(),
    }
}

fn build_selected_target_trace(
    score: &ActionScore,
    target: &ActionTarget,
) -> Option<TargetCandidateTrace> {
    let key = target.key?;

    score
        .target_candidates
        .iter()
        .find(|candidate| candidate.key == key && candidate.entity == target.entity)
        .cloned()
        .or_else(|| {
            target.label.clone().map(|label| TargetCandidateTrace {
                label,
                entity: target.entity,
                key,
                score: target.score,
                enabled: true,
                considerations: Vec::new(),
            })
        })
}

fn push_history(
    buffer: &mut DecisionTraceBuffer,
    previous_label: Option<String>,
    next_label: Option<String>,
    reason: ActionChangeReason,
    now: f32,
) {
    buffer.history.push(ActionHistoryEntry {
        previous_action: previous_label,
        next_action: next_label,
        reason,
        changed_at_seconds: now,
    });
    trim_history(buffer);
}

fn trim_history(buffer: &mut DecisionTraceBuffer) {
    if buffer.capacity == 0 {
        buffer.history.clear();
        return;
    }
    if buffer.history.len() > buffer.capacity {
        let excess = buffer.history.len() - buffer.capacity;
        buffer.history.drain(0..excess);
    }
}
