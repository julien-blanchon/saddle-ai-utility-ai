use saddle_ai_utility_ai_example_support as support;

use bevy::prelude::*;
use saddle_ai_utility_ai::{
    ActionTarget, ConsiderationInput, DecisionTraceBuffer, EvaluationPolicy, ResponseCurve,
    SelectionStrategy, TargetKey, TargetRequirement, UtilityAction, UtilityAgent, UtilityAiPlugin,
    UtilityConsideration, UtilityTargetCandidate,
};
use support::{configure_2d_example, ui_text_node};

#[derive(Component)]
struct OverlayText;

#[derive(Component, Clone, Copy)]
struct TargetDriver(u64);

#[derive(Resource)]
struct TargetLayout(Vec<(TargetKey, Vec2)>);

fn main() {
    let mut app = App::new();
    configure_2d_example(&mut app, "utility_ai target scoring", 7.0);
    app.insert_resource(TargetLayout(vec![
        (TargetKey(10), Vec2::new(-140.0, -80.0)),
        (TargetKey(20), Vec2::new(40.0, 110.0)),
        (TargetKey(30), Vec2::new(180.0, -30.0)),
    ]));
    app.add_plugins(UtilityAiPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, (animate_targets, draw_scene, update_overlay));
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Overlay"),
        OverlayText,
        Text::new(String::new()),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        Node {
            width: px(520.0),
            ..ui_text_node(18.0, 18.0)
        },
    ));

    commands
        .spawn((
            Name::new("Hunter"),
            UtilityAgent::default(),
            EvaluationPolicy::interval(0.12),
        ))
        .with_children(|agent| {
            agent
                .spawn((
                    Name::new("Engage"),
                    UtilityAction {
                        label: "engage".into(),
                        target_requirement: TargetRequirement::Required,
                        target_selection: SelectionStrategy::HighestScore,
                        ..default()
                    },
                ))
                .with_children(|action| {
                    action.spawn((
                        Name::new("Opportunity"),
                        UtilityConsideration::new("opportunity", ResponseCurve::Linear),
                        ConsiderationInput {
                            value: Some(1.0),
                            ..default()
                        },
                    ));

                    for key in [10, 20, 30] {
                        action
                            .spawn((
                                Name::new(format!("Target {key}")),
                                UtilityTargetCandidate {
                                    label: format!("target_{key}"),
                                    key: TargetKey(key),
                                    ..default()
                                },
                            ))
                            .with_children(|target| {
                                target.spawn((
                                    Name::new(format!("Target Driver {key}")),
                                    TargetDriver(key),
                                    UtilityConsideration::new(
                                        format!("score_{key}"),
                                        ResponseCurve::Gaussian {
                                            mean: 0.75,
                                            deviation: 0.18,
                                        },
                                    ),
                                    ConsiderationInput::default(),
                                ));
                            });
                    }
                });
        });
}

fn animate_targets(time: Res<Time>, mut inputs: Query<(&TargetDriver, &mut ConsiderationInput)>) {
    let seconds = time.elapsed_secs();
    for (driver, mut input) in &mut inputs {
        let value = match driver.0 {
            10 => ((seconds * 0.9).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
            20 => (((seconds * 0.9) + 1.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
            _ => (((seconds * 0.9) + 2.4).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
        };
        input.value = Some(value);
    }
}

fn draw_scene(mut gizmos: Gizmos, layout: Res<TargetLayout>, selected: Single<&ActionTarget>) {
    let agent_position = Vec2::ZERO;
    gizmos.circle_2d(agent_position, 18.0, Color::srgb(0.20, 0.72, 0.96));

    for (key, position) in &layout.0 {
        let color = if selected.key == Some(*key) {
            Color::srgb(0.95, 0.82, 0.24)
        } else {
            Color::srgb(0.46, 0.54, 0.64)
        };
        gizmos.circle_2d(*position, 14.0, color);
        if selected.key == Some(*key) {
            gizmos.line_2d(agent_position, *position, color);
        }
    }
}

fn update_overlay(
    mut text: Single<&mut Text, With<OverlayText>>,
    selected: Single<&ActionTarget>,
    traces: Single<&DecisionTraceBuffer>,
) {
    let mut lines = vec![
        "utility_ai target_scoring".to_string(),
        "One action scores three targets and keeps the best candidate.".to_string(),
        format!(
            "Selected target: {}",
            selected.label.clone().unwrap_or_else(|| "none".into())
        ),
    ];

    if let Some(trace) = &traces.last {
        for action in &trace.actions {
            lines.push(format!("{} final {:.2}", action.label, action.final_score));
            for target in &action.target_candidates {
                lines.push(format!("  {} -> {:.2}", target.label, target.score));
            }
        }
    }

    text.0 = lines.join("\n");
}
