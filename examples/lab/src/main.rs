#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use std::f32::consts::PI;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_pane::prelude::*;
use saddle_ai_utility_ai::{
    ActionChanged, ActionCompleted, ActionEvaluationRequested, ActionLifecycle, ActiveAction,
    CompositionPolicy, ConsiderationInput, DecisionMomentum, DecisionTraceBuffer, EvaluationPolicy,
    PriorityTier, ResponseCurve, TargetKey, TargetRequirement, TargetScoreFold, UtilityAction,
    UtilityAgent, UtilityAiBudget, UtilityAiPlugin, UtilityAiStats, UtilityAiSystems,
    UtilityConsideration, UtilityTargetCandidate,
};

const DEFAULT_BRP_PORT: u16 = 15_739;
const STRESS_COLUMNS: usize = 10;
const STRESS_ROWS: usize = 8;

#[derive(Component)]
struct OverlayText;

#[derive(Component)]
struct FlipAgent;

#[derive(Component)]
struct TargetAgent;

#[derive(Component)]
struct PriorityAgent;

#[derive(Component)]
struct StressAgent;

#[derive(Component)]
struct TargetMarker;

#[derive(Component, Clone, Copy, Debug)]
struct OscillationDriver {
    base: f32,
    amplitude: f32,
    speed: f32,
    phase: f32,
}

#[derive(Resource, Reflect, Clone, Debug, Default)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub flip_active: String,
    pub flip_switch_count: u32,
    pub flip_last_reason: String,
    pub target_active: String,
    pub target_selected_target: String,
    pub target_selected_score: f32,
    pub priority_active: String,
    pub priority_emergency_score: f32,
    pub priority_tactical_score: f32,
    pub priority_tactical_suppressed: bool,
    pub stress_agents_total: usize,
    pub stress_active_agents: usize,
    pub stress_peak_skipped_due_to_budget: usize,
    pub stress_peak_eval_micros: u64,
    pub stress_peak_scored_actions: usize,
    pub action_changed_messages: u32,
    pub action_completed_messages: u32,
    pub total_switches: u32,
    pub last_change_reason: String,
}

#[derive(Resource, Clone, Pane)]
#[pane(title = "Utility Lab")]
struct UtilityLabPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(slider, min = 1.0, max = 256.0, step = 1.0)]
    max_agents_per_update: usize,
    #[pane(slider, min = 0.02, max = 0.5, step = 0.01)]
    evaluation_interval_seconds: f32,
    #[pane(slider, min = 0.0, max = 1.0, step = 0.05)]
    jitter_fraction: f32,
    #[pane(slider, min = 0.0, max = 0.3, step = 0.01)]
    active_action_bonus: f32,
    #[pane(slider, min = 0.0, max = 0.2, step = 0.01)]
    hysteresis_band: f32,
}

impl Default for UtilityLabPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            max_agents_per_update: 18,
            evaluation_interval_seconds: 0.09,
            jitter_fraction: 0.25,
            active_action_bonus: 0.08,
            hysteresis_band: 0.06,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.03, 0.035, 0.05)));
    app.insert_resource(UtilityAiBudget {
        max_agents_per_update: 18,
    });
    app.init_resource::<LabDiagnostics>();
    app.init_resource::<UtilityLabPane>();
    app.register_type::<LabDiagnostics>();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "utility_ai crate-local lab".into(),
            resolution: (1560, 920).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    ));
    app.register_pane::<UtilityLabPane>();
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_port(lab_brp_port()));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::UtilityAiLabE2EPlugin);
    app.add_plugins(UtilityAiPlugin::always_on(Update));
    app.add_systems(Startup, setup);
    app.add_systems(Update, sync_pane_to_runtime);
    app.add_systems(Update, drive_inputs.before(UtilityAiSystems::GatherInputs));
    app.add_systems(
        Update,
        promote_requested_actions.after(UtilityAiSystems::Transition),
    );
    app.add_systems(
        Update,
        record_action_messages.after(UtilityAiSystems::Transition),
    );
    app.add_systems(
        Update,
        update_agent_visuals.after(UtilityAiSystems::Transition),
    );
    app.add_systems(
        Update,
        update_diagnostics.after(UtilityAiSystems::DebugRender),
    );
    app.add_systems(Update, update_overlay.after(update_diagnostics));
    app.add_systems(Update, draw_debug_gizmos.after(update_diagnostics));
    app.run();
}

fn sync_pane_to_runtime(
    pane: Res<UtilityLabPane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut budget: ResMut<UtilityAiBudget>,
    mut policies: Query<&mut EvaluationPolicy, With<UtilityAgent>>,
    mut momentum: Query<&mut DecisionMomentum, With<UtilityAgent>>,
) {
    if !pane.is_changed() {
        return;
    }

    virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    budget.max_agents_per_update = pane.max_agents_per_update.max(1);

    for mut policy in &mut policies {
        policy.base_interval_seconds = pane.evaluation_interval_seconds.max(0.01);
        policy.jitter_fraction = pane.jitter_fraction.clamp(0.0, 1.0);
    }

    for mut entry in &mut momentum {
        entry.active_action_bonus = pane.active_action_bonus.max(0.0);
        entry.hysteresis_band = pane.hysteresis_band.max(0.0);
    }
}

#[cfg(feature = "dev")]
fn lab_brp_port() -> u16 {
    std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_BRP_PORT)
}

fn setup(mut commands: Commands) {
    commands.spawn((Name::new("Lab Camera"), Camera2d));

    commands.spawn((
        Name::new("Momentum Panel"),
        Sprite::from_color(Color::srgb(0.08, 0.11, 0.17), Vec2::new(320.0, 260.0)),
        Transform::from_xyz(-460.0, 220.0, -5.0),
    ));
    commands.spawn((
        Name::new("Target Panel"),
        Sprite::from_color(Color::srgb(0.08, 0.10, 0.13), Vec2::new(320.0, 260.0)),
        Transform::from_xyz(0.0, 220.0, -5.0),
    ));
    commands.spawn((
        Name::new("Priority Panel"),
        Sprite::from_color(Color::srgb(0.13, 0.08, 0.10), Vec2::new(320.0, 260.0)),
        Transform::from_xyz(460.0, 220.0, -5.0),
    ));
    commands.spawn((
        Name::new("Stress Panel"),
        Sprite::from_color(Color::srgb(0.06, 0.08, 0.10), Vec2::new(1240.0, 320.0)),
        Transform::from_xyz(-40.0, -170.0, -5.0),
    ));

    spawn_static_labels(&mut commands);
    spawn_flip_agent(&mut commands);
    spawn_target_agent(&mut commands);
    spawn_priority_agent(&mut commands);
    spawn_stress_swarm(&mut commands);

    commands.spawn((
        Name::new("Overlay"),
        OverlayText,
        Text::new("utility_ai lab"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            right: px(18.0),
            top: px(18.0),
            width: px(420.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.01, 0.02, 0.03, 0.86)),
    ));
}

fn spawn_static_labels(commands: &mut Commands) {
    for (name, label, left, top) in [
        ("Momentum Label", "momentum / hysteresis", 54.0, 34.0),
        ("Target Label", "target scoring", 468.0, 34.0),
        ("Priority Label", "priority tiers", 892.0, 34.0),
        ("Stress Label", "budgeted crowd", 54.0, 392.0),
    ] {
        commands.spawn((
            Name::new(name),
            Text::new(label),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.94, 0.98)),
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                ..default()
            },
        ));
    }
}

fn spawn_flip_agent(commands: &mut Commands) {
    let agent = commands
        .spawn((
            Name::new("Flip Agent"),
            FlipAgent,
            UtilityAgent {
                trace_capacity: 16,
                ..default()
            },
            EvaluationPolicy {
                base_interval_seconds: 0.07,
                pending_request: true,
                ..default()
            },
            DecisionMomentum {
                active_action_bonus: 0.12,
                hysteresis_band: 0.11,
                momentum_decay_per_second: 0.0,
            },
            Sprite::from_color(Color::srgb(0.22, 0.66, 0.92), Vec2::new(42.0, 42.0)),
            Transform::from_xyz(-460.0, 210.0, 1.0),
        ))
        .id();

    commands.entity(agent).with_children(|agent| {
        agent
            .spawn((
                Name::new("Advance Action"),
                UtilityAction {
                    minimum_commitment_seconds: 0.45,
                    ..UtilityAction::new("advance")
                },
            ))
            .with_children(|action| {
                action.spawn((
                    Name::new("Advance Pressure"),
                    UtilityConsideration::new(
                        "pressure",
                        ResponseCurve::Logistic {
                            midpoint: 0.5,
                            steepness: 7.0,
                        },
                    ),
                    ConsiderationInput::default(),
                    OscillationDriver {
                        base: 0.52,
                        amplitude: 0.18,
                        speed: 1.1,
                        phase: 0.0,
                    },
                ));
            });

        agent
            .spawn((
                Name::new("Hold Action"),
                UtilityAction {
                    minimum_commitment_seconds: 0.45,
                    ..UtilityAction::new("hold")
                },
            ))
            .with_children(|action| {
                action.spawn((
                    Name::new("Hold Pressure"),
                    UtilityConsideration::new(
                        "stability",
                        ResponseCurve::Logistic {
                            midpoint: 0.5,
                            steepness: 7.0,
                        },
                    ),
                    ConsiderationInput::default(),
                    OscillationDriver {
                        base: 0.52,
                        amplitude: 0.18,
                        speed: 1.1,
                        phase: PI,
                    },
                ));
            });
    });
}

fn spawn_target_agent(commands: &mut Commands) {
    let relay_alpha = commands
        .spawn((
            Name::new("Relay Alpha"),
            TargetMarker,
            Sprite::from_color(Color::srgb(0.35, 0.48, 0.62), Vec2::new(26.0, 26.0)),
            Transform::from_xyz(-82.0, 250.0, 1.0),
        ))
        .id();
    let relay_beta = commands
        .spawn((
            Name::new("Relay Beta"),
            TargetMarker,
            Sprite::from_color(Color::srgb(0.35, 0.48, 0.62), Vec2::new(26.0, 26.0)),
            Transform::from_xyz(14.0, 288.0, 1.0),
        ))
        .id();
    let relay_gamma = commands
        .spawn((
            Name::new("Relay Gamma"),
            TargetMarker,
            Sprite::from_color(Color::srgb(0.35, 0.48, 0.62), Vec2::new(26.0, 26.0)),
            Transform::from_xyz(92.0, 210.0, 1.0),
        ))
        .id();

    let agent = commands
        .spawn((
            Name::new("Target Agent"),
            TargetAgent,
            UtilityAgent::default(),
            EvaluationPolicy {
                base_interval_seconds: 0.08,
                pending_request: true,
                ..default()
            },
            Sprite::from_color(Color::srgb(0.95, 0.70, 0.22), Vec2::new(42.0, 42.0)),
            Transform::from_xyz(0.0, 210.0, 1.0),
        ))
        .id();

    commands.entity(agent).with_children(|agent| {
        agent
            .spawn((
                Name::new("Engage Action"),
                UtilityAction {
                    priority: PriorityTier::TACTICAL,
                    target_requirement: TargetRequirement::Required,
                    target_score_fold: TargetScoreFold::Multiply,
                    composition: CompositionPolicy {
                        empty_score: 0.2,
                        ..default()
                    },
                    ..UtilityAction::new("engage")
                },
            ))
            .with_children(|action| {
                action.spawn((
                    Name::new("Opportunity"),
                    UtilityConsideration::new("opportunity", ResponseCurve::SmoothStep),
                    ConsiderationInput {
                        value: Some(0.94),
                        enabled: true,
                    },
                ));

                spawn_target_candidate(
                    action,
                    "Relay Alpha Candidate",
                    "relay_alpha",
                    relay_alpha,
                    11,
                    0.38,
                );
                spawn_target_candidate(
                    action,
                    "Relay Beta Candidate",
                    "relay_beta",
                    relay_beta,
                    22,
                    0.93,
                );
                spawn_target_candidate(
                    action,
                    "Relay Gamma Candidate",
                    "relay_gamma",
                    relay_gamma,
                    33,
                    0.61,
                );
            });

        agent.spawn((
            Name::new("Observe Fallback"),
            UtilityAction {
                is_fallback: true,
                minimum_score: 0.0,
                ..UtilityAction::new("observe")
            },
        ));
    });
}

fn spawn_target_candidate(
    action: &mut ChildSpawnerCommands<'_>,
    entity_name: &str,
    label: &str,
    target_entity: Entity,
    key: u64,
    value: f32,
) {
    action
        .spawn((
            Name::new(entity_name.to_string()),
            UtilityTargetCandidate {
                label: label.into(),
                entity: Some(target_entity),
                key: TargetKey(key),
                ..default()
            },
        ))
        .with_children(|candidate| {
            candidate.spawn((
                Name::new(format!("{entity_name} Utility")),
                UtilityConsideration::new(
                    "desirability",
                    ResponseCurve::Gaussian {
                        mean: 0.85,
                        deviation: 0.28,
                    },
                ),
                ConsiderationInput {
                    value: Some(value),
                    enabled: true,
                },
            ));
        });
}

fn spawn_priority_agent(commands: &mut Commands) {
    let agent = commands
        .spawn((
            Name::new("Priority Agent"),
            PriorityAgent,
            UtilityAgent::default(),
            EvaluationPolicy {
                base_interval_seconds: 0.09,
                pending_request: true,
                ..default()
            },
            Sprite::from_color(Color::srgb(0.78, 0.28, 0.32), Vec2::new(44.0, 44.0)),
            Transform::from_xyz(460.0, 210.0, 1.0),
        ))
        .id();

    commands.entity(agent).with_children(|agent| {
        agent
            .spawn((
                Name::new("Panic Action"),
                UtilityAction::new("panic").with_priority(PriorityTier::CRITICAL, 0.75),
            ))
            .with_children(|action| {
                action.spawn((
                    Name::new("Danger Consideration"),
                    UtilityConsideration::new(
                        "danger",
                        ResponseCurve::Logistic {
                            midpoint: 0.45,
                            steepness: 9.0,
                        },
                    ),
                    ConsiderationInput {
                        value: Some(0.86),
                        enabled: true,
                    },
                ));
            });

        agent
            .spawn((
                Name::new("Patrol Action"),
                UtilityAction::new("patrol").with_priority(PriorityTier::TACTICAL, 0.0),
            ))
            .with_children(|action| {
                action.spawn((
                    Name::new("Route Confidence"),
                    UtilityConsideration::new("route", ResponseCurve::SmoothStep),
                    ConsiderationInput {
                        value: Some(0.97),
                        enabled: true,
                    },
                ));
            });
    });
}

fn spawn_stress_swarm(commands: &mut Commands) {
    for row in 0..STRESS_ROWS {
        for column in 0..STRESS_COLUMNS {
            let index = row * STRESS_COLUMNS + column;
            let x = -590.0 + column as f32 * 118.0;
            let y = -78.0 - row as f32 * 30.0;
            let phase = index as f32 * 0.37;

            let agent = commands
                .spawn((
                    Name::new(format!("Stress Agent {index:02}")),
                    StressAgent,
                    UtilityAgent {
                        selection_seed: 11 + index as u64,
                        ..default()
                    },
                    EvaluationPolicy {
                        base_interval_seconds: 0.11 + (row as f32 * 0.01),
                        jitter_fraction: 0.55,
                        pending_request: true,
                        ..default()
                    },
                    DecisionMomentum {
                        active_action_bonus: 0.04,
                        hysteresis_band: 0.03,
                        momentum_decay_per_second: 0.0,
                    },
                    Sprite::from_color(Color::srgb(0.38, 0.48, 0.60), Vec2::new(14.0, 14.0)),
                    Transform::from_xyz(x, y, 1.0),
                ))
                .id();

            commands.entity(agent).with_children(|agent| {
                spawn_stress_action(
                    agent,
                    "press",
                    ResponseCurve::Logistic {
                        midpoint: 0.55,
                        steepness: 7.5,
                    },
                    OscillationDriver {
                        base: 0.52,
                        amplitude: 0.42,
                        speed: 0.42,
                        phase,
                    },
                );
                spawn_stress_action(
                    agent,
                    "recover",
                    ResponseCurve::Gaussian {
                        mean: 0.24,
                        deviation: 0.20,
                    },
                    OscillationDriver {
                        base: 0.28,
                        amplitude: 0.26,
                        speed: 0.31,
                        phase: phase + 1.1,
                    },
                );
                agent
                    .spawn((Name::new("Idle Action"), UtilityAction::new("idle")))
                    .with_children(|action| {
                        action.spawn((
                            Name::new("Idle Baseline"),
                            UtilityConsideration::new("baseline", ResponseCurve::Linear),
                            ConsiderationInput {
                                value: Some(0.22),
                                enabled: true,
                            },
                        ));
                    });
            });
        }
    }
}

fn spawn_stress_action(
    agent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    curve: ResponseCurve,
    driver: OscillationDriver,
) {
    agent
        .spawn((
            Name::new(format!("{label} action")),
            UtilityAction::new(label),
        ))
        .with_children(|action| {
            action.spawn((
                Name::new(format!("{label} utility")),
                UtilityConsideration::new(label, curve),
                ConsiderationInput::default(),
                driver,
            ));
        });
}

fn drive_inputs(time: Res<Time>, mut inputs: Query<(&OscillationDriver, &mut ConsiderationInput)>) {
    let now = time.elapsed_secs();

    for (driver, mut input) in &mut inputs {
        let value = driver.base + driver.amplitude * ((now * driver.speed) + driver.phase).sin();
        input.value = Some(value.clamp(0.0, 1.0));
    }
}

fn promote_requested_actions(mut actions: Query<&mut UtilityAction>) {
    for mut action in &mut actions {
        if action.lifecycle == ActionLifecycle::Requested {
            action.lifecycle = ActionLifecycle::Executing;
        }
    }
}

fn record_action_messages(
    mut changed: MessageReader<ActionChanged>,
    mut completed: MessageReader<ActionCompleted>,
    mut requests: MessageReader<ActionEvaluationRequested>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    for event in changed.read() {
        diagnostics.action_changed_messages = diagnostics.action_changed_messages.saturating_add(1);
        diagnostics.last_change_reason = format!("{:?}", event.reason);
    }

    for _ in completed.read() {
        diagnostics.action_completed_messages =
            diagnostics.action_completed_messages.saturating_add(1);
    }

    for _ in requests.read() {}
}

fn update_agent_visuals(
    mut agent_sprites: Query<
        (
            &ActiveAction,
            &mut Sprite,
            Option<&FlipAgent>,
            Option<&TargetAgent>,
            Option<&PriorityAgent>,
            Option<&StressAgent>,
        ),
        (With<UtilityAgent>, Without<TargetMarker>),
    >,
    target_agent: Single<&DecisionTraceBuffer, With<TargetAgent>>,
    mut target_markers: Query<(Entity, &mut Sprite), (With<TargetMarker>, Without<UtilityAgent>)>,
) {
    for (active, mut sprite, flip, target, priority, stress) in &mut agent_sprites {
        let active_label = active.label.as_deref().unwrap_or("none");
        sprite.color = if flip.is_some() {
            match active_label {
                "advance" => Color::srgb(0.95, 0.56, 0.26),
                "hold" => Color::srgb(0.22, 0.66, 0.92),
                _ => Color::srgb(0.42, 0.50, 0.58),
            }
        } else if target.is_some() {
            match active_label {
                "engage" => Color::srgb(0.96, 0.78, 0.25),
                "observe" => Color::srgb(0.44, 0.58, 0.68),
                _ => Color::srgb(0.42, 0.50, 0.58),
            }
        } else if priority.is_some() {
            match active_label {
                "panic" => Color::srgb(0.92, 0.24, 0.28),
                "patrol" => Color::srgb(0.34, 0.76, 0.44),
                _ => Color::srgb(0.42, 0.50, 0.58),
            }
        } else if stress.is_some() {
            match active_label {
                "press" => Color::srgb(0.26, 0.78, 0.88),
                "recover" => Color::srgb(0.94, 0.58, 0.24),
                "idle" => Color::srgb(0.46, 0.52, 0.60),
                _ => Color::srgb(0.38, 0.48, 0.60),
            }
        } else {
            sprite.color
        };
    }

    let winning_marker = target_agent
        .last
        .as_ref()
        .and_then(|trace| trace.winning_target.as_ref())
        .and_then(|target| target.entity);

    for (entity, mut sprite) in &mut target_markers {
        sprite.color = if Some(entity) == winning_marker {
            Color::srgb(0.97, 0.87, 0.36)
        } else {
            Color::srgb(0.35, 0.48, 0.62)
        };
    }
}

fn update_diagnostics(
    mut diagnostics: ResMut<LabDiagnostics>,
    stats: Res<UtilityAiStats>,
    flip_agent: Single<(&ActiveAction, &DecisionTraceBuffer), With<FlipAgent>>,
    target_agent: Single<(&ActiveAction, &DecisionTraceBuffer), With<TargetAgent>>,
    priority_agent: Single<(&ActiveAction, &DecisionTraceBuffer), With<PriorityAgent>>,
    all_agents: Query<&ActiveAction, With<UtilityAgent>>,
    stress_agents: Query<&ActiveAction, With<StressAgent>>,
) {
    diagnostics.flip_active = flip_agent.0.label.clone().unwrap_or_else(|| "none".into());
    diagnostics.flip_switch_count = flip_agent.0.switch_count;
    diagnostics.flip_last_reason = flip_agent
        .1
        .last
        .as_ref()
        .and_then(|trace| trace.switch_reason)
        .map(|reason| format!("{reason:?}"))
        .unwrap_or_else(|| "none".into());

    diagnostics.target_active = target_agent
        .0
        .label
        .clone()
        .unwrap_or_else(|| "none".into());
    diagnostics.target_selected_target = target_agent
        .1
        .last
        .as_ref()
        .and_then(|trace| trace.winning_target.as_ref())
        .map(|target| target.label.clone())
        .unwrap_or_else(|| "none".into());
    diagnostics.target_selected_score = target_agent
        .1
        .last
        .as_ref()
        .and_then(|trace| trace.winning_target.as_ref())
        .map_or(0.0, |target| target.score);

    diagnostics.priority_active = priority_agent
        .0
        .label
        .clone()
        .unwrap_or_else(|| "none".into());
    diagnostics.priority_emergency_score = action_score(priority_agent.1, "panic");
    diagnostics.priority_tactical_score = action_score(priority_agent.1, "patrol");
    diagnostics.priority_tactical_suppressed = diagnostics.priority_active == "panic"
        && diagnostics.priority_emergency_score >= 0.75
        && diagnostics.priority_tactical_score >= 0.9;

    diagnostics.stress_agents_total = stress_agents.iter().len();
    diagnostics.stress_active_agents = stress_agents
        .iter()
        .filter(|active| active.entity.is_some())
        .count();
    diagnostics.stress_peak_skipped_due_to_budget = diagnostics
        .stress_peak_skipped_due_to_budget
        .max(stats.skipped_due_to_budget);
    diagnostics.stress_peak_eval_micros = diagnostics
        .stress_peak_eval_micros
        .max(stats.last_evaluation_time_micros);
    diagnostics.stress_peak_scored_actions = diagnostics
        .stress_peak_scored_actions
        .max(stats.scored_actions);
    diagnostics.total_switches = all_agents.iter().map(|active| active.switch_count).sum();
}

fn update_overlay(
    diagnostics: Res<LabDiagnostics>,
    stats: Res<UtilityAiStats>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    if !diagnostics.is_changed() && !stats.is_changed() {
        return;
    }

    **overlay = format!(
        "utility_ai lab\n\
flip agent\n\
active: {}\n\
switches: {}\n\
last reason: {}\n\n\
target agent\n\
active: {}\n\
winning target: {} ({:.2})\n\n\
priority agent\n\
active: {}\n\
panic score: {:.2}\n\
patrol score: {:.2}\n\
tactical suppressed: {}\n\n\
stress swarm\n\
active / total: {} / {}\n\
last eval agents/actions/targets: {} / {} / {}\n\
skipped due to budget peak: {}\n\
peak eval time us: {}\n\
total action changes: {}\n\
last change reason: {}",
        diagnostics.flip_active,
        diagnostics.flip_switch_count,
        diagnostics.flip_last_reason,
        diagnostics.target_active,
        diagnostics.target_selected_target,
        diagnostics.target_selected_score,
        diagnostics.priority_active,
        diagnostics.priority_emergency_score,
        diagnostics.priority_tactical_score,
        diagnostics.priority_tactical_suppressed,
        diagnostics.stress_active_agents,
        diagnostics.stress_agents_total,
        stats.evaluated_agents,
        stats.scored_actions,
        stats.scored_targets,
        diagnostics.stress_peak_skipped_due_to_budget,
        diagnostics.stress_peak_eval_micros,
        diagnostics.action_changed_messages,
        diagnostics.last_change_reason,
    )
    .into();
}

fn draw_debug_gizmos(
    mut gizmos: Gizmos,
    target_agent: Single<(&Transform, &DecisionTraceBuffer), With<TargetAgent>>,
    targets: Query<&Transform, With<TargetMarker>>,
    flip_agent: Single<&Transform, With<FlipAgent>>,
    priority_agent: Single<&Transform, With<PriorityAgent>>,
) {
    if let Some(target_entity) = target_agent
        .1
        .last
        .as_ref()
        .and_then(|trace| trace.winning_target.as_ref())
        .and_then(|target| target.entity)
    {
        if let Ok(target_transform) = targets.get(target_entity) {
            gizmos.line_2d(
                target_agent.0.translation.truncate(),
                target_transform.translation.truncate(),
                Color::srgb(0.96, 0.82, 0.26),
            );
        }
    }

    gizmos.line_2d(
        flip_agent.translation.truncate() + Vec2::new(-54.0, 48.0),
        flip_agent.translation.truncate() + Vec2::new(54.0, 48.0),
        Color::srgb(0.24, 0.54, 0.84),
    );
    gizmos.line_2d(
        priority_agent.translation.truncate() + Vec2::new(-54.0, 48.0),
        priority_agent.translation.truncate() + Vec2::new(54.0, 48.0),
        Color::srgb(0.84, 0.24, 0.28),
    );
}

fn action_score(buffer: &DecisionTraceBuffer, label: &str) -> f32 {
    buffer
        .last
        .as_ref()
        .and_then(|trace| trace.actions.iter().find(|action| action.label == label))
        .map_or(0.0, |trace| trace.momentum_score)
}
