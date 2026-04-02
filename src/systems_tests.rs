use std::time::Duration;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::*;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestUpdate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestDeactivate;

fn test_app() -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.init_schedule(TestUpdate);
    app.init_schedule(TestDeactivate);
    app.add_plugins(UtilityAiPlugin::new(TestUpdate, TestDeactivate, TestUpdate));
    app
}

fn run_test_schedule(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(16));
    app.world_mut().run_schedule(TestUpdate);
}

fn drain_messages<T: Message>(app: &mut App) -> Vec<T> {
    app.world_mut()
        .resource_mut::<Messages<T>>()
        .drain()
        .collect()
}

fn consideration(world: &mut World, label: &str, curve: ResponseCurve, value: f32) -> Entity {
    world
        .spawn((
            Name::new(label.to_string()),
            UtilityConsideration::new(label, curve),
            ConsiderationInput {
                value: Some(value),
                enabled: true,
            },
        ))
        .id()
}

#[test]
fn plugin_registers_core_resources_and_messages() {
    let app = test_app();

    assert!(app.world().contains_resource::<UtilityAiBudget>());
    assert!(app.world().contains_resource::<UtilityAiStats>());
    assert!(app.world().contains_resource::<Messages<ActionChanged>>());
    assert!(app.world().contains_resource::<Messages<ActionCompleted>>());
    assert!(
        app.world()
            .contains_resource::<Messages<ActionEvaluationRequested>>()
    );
}

#[test]
fn custom_schedule_bootstraps_and_selects_initial_action() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((Name::new("Agent"), UtilityAgent::default()))
        .id();
    let action = app
        .world_mut()
        .spawn((Name::new("Advance"), UtilityAction::new("advance")))
        .id();
    let input = consideration(app.world_mut(), "Need", ResponseCurve::Linear, 0.8);
    app.world_mut().entity_mut(action).add_children(&[input]);
    app.world_mut().entity_mut(agent).add_children(&[action]);

    run_test_schedule(&mut app);

    assert!(app.world().get::<EvaluationPolicy>(agent).is_some());
    assert!(app.world().get::<ActionScore>(action).is_some());
    assert_eq!(
        app.world()
            .get::<ActiveAction>(agent)
            .unwrap()
            .label
            .as_deref(),
        Some("advance")
    );
}

#[test]
fn zero_action_agents_evaluate_without_panicking() {
    let mut app = test_app();
    let agent = app
        .world_mut()
        .spawn((Name::new("Empty Agent"), UtilityAgent::default()))
        .id();

    run_test_schedule(&mut app);

    let active = app.world().get::<ActiveAction>(agent).unwrap();
    assert!(active.entity.is_none());
    assert!(app.world().get::<DecisionTraceBuffer>(agent).is_some());
}

#[test]
fn explicit_reevaluation_request_drives_manual_agents() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((
            Name::new("Manual Agent"),
            UtilityAgent::default(),
            EvaluationPolicy::manual(),
        ))
        .id();
    let low = app
        .world_mut()
        .spawn((Name::new("Low"), UtilityAction::new("low")))
        .id();
    let high = app
        .world_mut()
        .spawn((Name::new("High"), UtilityAction::new("high")))
        .id();
    let low_input = consideration(app.world_mut(), "Low Score", ResponseCurve::Linear, 0.8);
    let high_input = consideration(app.world_mut(), "High Score", ResponseCurve::Linear, 0.2);
    app.world_mut().entity_mut(low).add_children(&[low_input]);
    app.world_mut().entity_mut(high).add_children(&[high_input]);
    app.world_mut().entity_mut(agent).add_children(&[low, high]);

    run_test_schedule(&mut app);
    assert_eq!(
        app.world()
            .get::<ActiveAction>(agent)
            .unwrap()
            .label
            .as_deref(),
        Some("low")
    );

    app.world_mut()
        .get_mut::<ConsiderationInput>(high_input)
        .unwrap()
        .value = Some(1.0);
    app.world_mut()
        .get_mut::<ConsiderationInput>(low_input)
        .unwrap()
        .value = Some(0.1);
    app.world_mut()
        .resource_mut::<Messages<ActionEvaluationRequested>>()
        .write(ActionEvaluationRequested::new(agent, "manual refresh"));

    run_test_schedule(&mut app);

    assert_eq!(
        app.world()
            .get::<ActiveAction>(agent)
            .unwrap()
            .label
            .as_deref(),
        Some("high")
    );
    assert!(
        app.world()
            .get::<DecisionTraceBuffer>(agent)
            .unwrap()
            .last
            .as_ref()
            .is_some_and(|trace| trace.requested)
    );
}

#[test]
fn target_scoring_selects_best_target_candidate() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((Name::new("Target Agent"), UtilityAgent::default()))
        .id();
    let action = app
        .world_mut()
        .spawn((
            Name::new("Engage"),
            UtilityAction::new("engage").with_priority(PriorityTier::TACTICAL, 0.0),
        ))
        .id();

    let base = consideration(app.world_mut(), "Opportunity", ResponseCurve::Linear, 1.0);
    let target_a = app
        .world_mut()
        .spawn((
            Name::new("Target A"),
            UtilityTargetCandidate {
                label: "target_a".into(),
                key: TargetKey(11),
                ..default()
            },
        ))
        .id();
    let target_b = app
        .world_mut()
        .spawn((
            Name::new("Target B"),
            UtilityTargetCandidate {
                label: "target_b".into(),
                key: TargetKey(22),
                ..default()
            },
        ))
        .id();
    let target_a_score = consideration(app.world_mut(), "A Score", ResponseCurve::Linear, 0.2);
    let target_b_score = consideration(app.world_mut(), "B Score", ResponseCurve::Linear, 0.9);
    app.world_mut()
        .entity_mut(target_a)
        .add_children(&[target_a_score]);
    app.world_mut()
        .entity_mut(target_b)
        .add_children(&[target_b_score]);
    app.world_mut()
        .entity_mut(action)
        .add_children(&[base, target_a, target_b]);
    app.world_mut().entity_mut(agent).add_children(&[action]);

    run_test_schedule(&mut app);

    let selected = app.world().get::<ActionTarget>(action).unwrap();
    assert_eq!(selected.key, Some(TargetKey(22)));
    assert_eq!(selected.label.as_deref(), Some("target_b"));

    let trace = app
        .world()
        .get::<DecisionTraceBuffer>(agent)
        .unwrap()
        .last
        .as_ref()
        .unwrap();
    let winning_target = trace.winning_target.as_ref().unwrap();
    assert_eq!(winning_target.label, "target_b");
    assert_eq!(winning_target.considerations.len(), 1);
    assert_eq!(winning_target.considerations[0].label, "B Score");
}

#[test]
fn action_changed_message_emits_on_switch() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((Name::new("Switch Agent"), UtilityAgent::default()))
        .id();
    let action_a = app
        .world_mut()
        .spawn((Name::new("Action A"), UtilityAction::new("a")))
        .id();
    let action_b = app
        .world_mut()
        .spawn((Name::new("Action B"), UtilityAction::new("b")))
        .id();
    let input_a = consideration(app.world_mut(), "A", ResponseCurve::Linear, 0.9);
    let input_b = consideration(app.world_mut(), "B", ResponseCurve::Linear, 0.1);
    app.world_mut()
        .entity_mut(action_a)
        .add_children(&[input_a]);
    app.world_mut()
        .entity_mut(action_b)
        .add_children(&[input_b]);
    app.world_mut()
        .entity_mut(agent)
        .add_children(&[action_a, action_b]);

    run_test_schedule(&mut app);
    drain_messages::<ActionChanged>(&mut app);

    app.world_mut()
        .get_mut::<ConsiderationInput>(input_b)
        .unwrap()
        .value = Some(1.0);
    app.world_mut()
        .get_mut::<ConsiderationInput>(input_a)
        .unwrap()
        .value = Some(0.1);
    app.world_mut()
        .resource_mut::<Messages<ActionEvaluationRequested>>()
        .write(ActionEvaluationRequested::new(agent, "switch"));

    run_test_schedule(&mut app);

    let changed = drain_messages::<ActionChanged>(&mut app);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].previous_label.as_deref(), Some("a"));
    assert_eq!(changed[0].next_label.as_deref(), Some("b"));
}

#[test]
fn multiplicative_actions_short_circuit_after_zero_score() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((Name::new("Short Circuit Agent"), UtilityAgent::default()))
        .id();
    let action = app
        .world_mut()
        .spawn((
            Name::new("Multiply"),
            UtilityAction::new("multiply")
                .with_composition(crate::scoring::CompositionStrategy::Multiplicative),
        ))
        .id();
    let zero = app
        .world_mut()
        .spawn((
            Name::new("Zero"),
            UtilityConsideration {
                label: "zero".into(),
                cost: ConsiderationCost::Cheap,
                curve: ResponseCurve::Linear,
                ..default()
            },
            ConsiderationInput {
                value: Some(0.0),
                enabled: true,
            },
        ))
        .id();
    let invalid_expensive = app
        .world_mut()
        .spawn((
            Name::new("Invalid Expensive"),
            UtilityConsideration {
                label: "invalid".into(),
                cost: ConsiderationCost::Expensive,
                curve: ResponseCurve::Linear,
                ..default()
            },
            ConsiderationInput {
                value: Some(f32::NAN),
                enabled: true,
            },
        ))
        .id();

    app.world_mut()
        .entity_mut(action)
        .add_children(&[zero, invalid_expensive]);
    app.world_mut().entity_mut(agent).add_children(&[action]);

    run_test_schedule(&mut app);

    let score = app.world().get::<ActionScore>(action).unwrap();
    assert_eq!(score.suppression, Some(ActionSuppressionReason::ZeroScore));
    assert_eq!(score.considerations.len(), 1);
    assert_eq!(score.considerations[0].label, "zero");
}

#[test]
fn disabled_fallback_is_not_selected() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((Name::new("Fallback Agent"), UtilityAgent::default()))
        .id();
    let fallback = app
        .world_mut()
        .spawn((
            Name::new("Fallback"),
            UtilityAction {
                enabled: false,
                is_fallback: true,
                ..UtilityAction::new("fallback")
            },
        ))
        .id();
    let fallback_input = consideration(app.world_mut(), "fallback", ResponseCurve::Linear, 0.5);
    app.world_mut()
        .entity_mut(fallback)
        .add_children(&[fallback_input]);
    app.world_mut().entity_mut(agent).add_children(&[fallback]);

    run_test_schedule(&mut app);

    assert!(
        app.world()
            .get::<ActiveAction>(agent)
            .unwrap()
            .entity
            .is_none()
    );
}

#[test]
fn completed_action_emits_message_and_restarts_cooldown() {
    let mut app = test_app();

    let agent = app
        .world_mut()
        .spawn((Name::new("Completion Agent"), UtilityAgent::default()))
        .id();
    let action = app
        .world_mut()
        .spawn((
            Name::new("Execute"),
            UtilityAction::new("execute"),
            ActionCooldown {
                duration_seconds: 0.75,
                restart_on_success: true,
                ..default()
            },
        ))
        .id();
    let input = consideration(app.world_mut(), "Need", ResponseCurve::Linear, 1.0);
    app.world_mut().entity_mut(action).add_children(&[input]);
    app.world_mut().entity_mut(agent).add_children(&[action]);

    run_test_schedule(&mut app);
    drain_messages::<ActionChanged>(&mut app);

    app.world_mut()
        .get_mut::<UtilityAction>(action)
        .unwrap()
        .lifecycle = ActionLifecycle::Success;
    app.world_mut()
        .get_mut::<ConsiderationInput>(input)
        .unwrap()
        .value = Some(0.0);
    app.world_mut()
        .resource_mut::<Messages<ActionEvaluationRequested>>()
        .write(ActionEvaluationRequested::new(agent, "complete"));

    run_test_schedule(&mut app);

    let completed = drain_messages::<ActionCompleted>(&mut app);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].label, "execute");
    assert_eq!(completed[0].lifecycle, ActionLifecycle::Success);

    let active = app.world().get::<ActiveAction>(agent).unwrap();
    assert!(active.entity.is_none());
    assert!(active.label.is_none());

    let action_state = app.world().get::<UtilityAction>(action).unwrap();
    assert_eq!(action_state.lifecycle, ActionLifecycle::Idle);

    let cooldown = app.world().get::<ActionCooldown>(action).unwrap();
    assert_eq!(cooldown.remaining_seconds, 0.75);
}

#[test]
fn frame_budget_spills_due_agents_across_updates() {
    let mut app = test_app();
    app.world_mut()
        .resource_mut::<UtilityAiBudget>()
        .max_agents_per_update = 1;

    for index in 0..2 {
        let agent = app
            .world_mut()
            .spawn((Name::new(format!("Agent {index}")), UtilityAgent::default()))
            .id();
        let action = app
            .world_mut()
            .spawn((
                Name::new(format!("Action {index}")),
                UtilityAction::new(format!("action_{index}")),
            ))
            .id();
        let input = consideration(app.world_mut(), "Need", ResponseCurve::Linear, 1.0);
        app.world_mut().entity_mut(action).add_children(&[input]);
        app.world_mut().entity_mut(agent).add_children(&[action]);
    }

    run_test_schedule(&mut app);
    assert_eq!(app.world().resource::<UtilityAiStats>().evaluated_agents, 1);
    assert_eq!(
        app.world()
            .resource::<UtilityAiStats>()
            .skipped_due_to_budget,
        1
    );

    run_test_schedule(&mut app);
    assert_eq!(app.world().resource::<UtilityAiStats>().evaluated_agents, 1);

    let active_count = {
        let world = app.world_mut();
        let mut query = world.query::<&ActiveAction>();
        query
            .iter(world)
            .filter(|active| active.entity.is_some())
            .count()
    };
    assert_eq!(active_count, 2);
}
