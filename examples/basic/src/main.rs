use saddle_ai_utility_ai_example_support as support;

use bevy::prelude::*;
use saddle_ai_utility_ai::{
    ActiveAction, ConsiderationInput, DecisionTraceBuffer, EvaluationPolicy, PriorityTier,
    ResponseCurve, UtilityAction, UtilityAgent, UtilityAiPlugin, UtilityConsideration,
};
use support::{configure_2d_example, ui_text_node};

#[derive(Component)]
struct OverlayText;

#[derive(Component)]
struct AdvanceNeed;

#[derive(Component)]
struct RecoverNeed;

#[derive(Component)]
struct IdleNeed;

fn main() {
    let mut app = App::new();
    configure_2d_example(&mut app, "utility_ai basic", 7.0);
    app.add_plugins(UtilityAiPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, (animate_inputs, update_overlay));
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Overlay"),
        OverlayText,
        Text::new(String::new()),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        Node {
            width: px(560.0),
            ..ui_text_node(18.0, 18.0)
        },
    ));

    commands
        .spawn((
            Name::new("Scout"),
            UtilityAgent::default(),
            EvaluationPolicy::interval(0.12),
        ))
        .with_children(|agent| {
            agent
                .spawn((
                    Name::new("Advance"),
                    UtilityAction::new("advance").with_priority(PriorityTier::TACTICAL, 0.4),
                ))
                .with_children(|action| {
                    action.spawn((
                        Name::new("Advance Need"),
                        AdvanceNeed,
                        UtilityConsideration::new("opportunity", ResponseCurve::SmoothStep),
                        ConsiderationInput::default(),
                    ));
                });

            agent
                .spawn((
                    Name::new("Recover"),
                    UtilityAction::new("recover").with_priority(PriorityTier::TACTICAL, 0.35),
                ))
                .with_children(|action| {
                    action.spawn((
                        Name::new("Recover Need"),
                        RecoverNeed,
                        UtilityConsideration::new(
                            "fatigue",
                            ResponseCurve::Logistic {
                                midpoint: 0.55,
                                steepness: 10.0,
                            },
                        ),
                        ConsiderationInput::default(),
                    ));
                });

            agent
                .spawn((
                    Name::new("Wait"),
                    UtilityAction {
                        label: "wait".into(),
                        is_fallback: true,
                        priority: PriorityTier::FLAVOR,
                        ..default()
                    },
                ))
                .with_children(|action| {
                    action.spawn((
                        Name::new("Idle Need"),
                        IdleNeed,
                        UtilityConsideration::new("idle", ResponseCurve::Linear),
                        ConsiderationInput::default(),
                    ));
                });
        });
}

fn animate_inputs(
    time: Res<Time>,
    mut advance: Query<&mut ConsiderationInput, With<AdvanceNeed>>,
    mut recover: Query<&mut ConsiderationInput, (With<RecoverNeed>, Without<AdvanceNeed>)>,
    mut idle: Query<
        &mut ConsiderationInput,
        (With<IdleNeed>, Without<AdvanceNeed>, Without<RecoverNeed>),
    >,
) {
    let seconds = time.elapsed_secs();
    let advance_value = ((seconds * 1.2).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let recover_value = (((seconds * 0.6) + 1.8).sin() * 0.5 + 0.5).clamp(0.0, 1.0);

    for mut input in &mut advance {
        input.value = Some(advance_value);
    }
    for mut input in &mut recover {
        input.value = Some(recover_value);
    }
    for mut input in &mut idle {
        input.value = Some(0.25);
    }
}

fn update_overlay(
    mut text: Single<&mut Text, With<OverlayText>>,
    agent: Single<(&ActiveAction, &DecisionTraceBuffer), With<UtilityAgent>>,
) {
    let (active, traces) = *agent;
    let mut lines = vec![
        "utility_ai basic".to_string(),
        "Scores oscillate between advance, recover, and a low fallback wait action.".to_string(),
        format!(
            "Active action: {}",
            active.label.clone().unwrap_or_else(|| "none".into())
        ),
    ];

    if let Some(trace) = &traces.last {
        lines.push(String::new());
        for action in &trace.actions {
            lines.push(format!(
                "{}  score {:.2}  momentum {:.2}  suppression {:?}",
                action.label, action.final_score, action.momentum_score, action.suppression
            ));
        }
    }

    text.0 = lines.join("\n");
}
