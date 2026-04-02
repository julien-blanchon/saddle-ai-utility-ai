use saddle_ai_utility_ai_example_support as support;

use bevy::prelude::*;
use saddle_ai_utility_ai::{
    ActiveAction, ConsiderationInput, DecisionTraceBuffer, EvaluationPolicy, ResponseCurve,
    SelectionStrategy, UtilityAction, UtilityAgent, UtilityAiPlugin, UtilityConsideration,
};
use support::{configure_2d_example, ui_text_node};

#[derive(Component)]
struct OverlayText;

#[derive(Component, Clone, Copy)]
struct StrategyInput {
    agent_index: usize,
    action_index: usize,
}

fn main() {
    let mut app = App::new();
    configure_2d_example(&mut app, "utility_ai selection strategies", 7.0);
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
            font_size: 18.0,
            ..default()
        },
        Node {
            width: px(760.0),
            ..ui_text_node(18.0, 18.0)
        },
    ));

    let strategies = [
        ("highest", SelectionStrategy::HighestScore),
        ("weighted", SelectionStrategy::WeightedRandom),
        ("top-n", SelectionStrategy::TopNRandom { count: 2 }),
        (
            "top-band",
            SelectionStrategy::TopBandRandom {
                percent_within_best: 0.08,
            },
        ),
    ];

    for (agent_index, (label, strategy)) in strategies.into_iter().enumerate() {
        commands
            .spawn((
                Name::new(format!("Agent {label}")),
                UtilityAgent {
                    selection_strategy: strategy,
                    selection_seed: (agent_index as u64) + 9,
                    ..default()
                },
                EvaluationPolicy::interval(0.12),
            ))
            .with_children(|agent| {
                for action_index in 0..3 {
                    agent
                        .spawn((
                            Name::new(format!("{label} action {action_index}")),
                            UtilityAction::new(format!("action_{action_index}")),
                        ))
                        .with_children(|action| {
                            action.spawn((
                                Name::new(format!("consideration_{action_index}")),
                                StrategyInput {
                                    agent_index,
                                    action_index,
                                },
                                UtilityConsideration::new(
                                    format!("score_{action_index}"),
                                    ResponseCurve::SmoothStep,
                                ),
                                ConsiderationInput::default(),
                            ));
                        });
                }
            });
    }
}

fn animate_inputs(time: Res<Time>, mut inputs: Query<(&StrategyInput, &mut ConsiderationInput)>) {
    let seconds = time.elapsed_secs();
    for (channel, mut input) in &mut inputs {
        let phase = channel.agent_index as f32 * 0.4;
        let value = match channel.action_index {
            0 => ((seconds * 1.1 + phase).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
            1 => (((seconds * 1.1 + phase) + 0.25).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
            _ => 0.55,
        };
        input.value = Some(value);
    }
}

fn update_overlay(
    mut text: Single<&mut Text, With<OverlayText>>,
    agents: Query<(&Name, &ActiveAction, &DecisionTraceBuffer), With<UtilityAgent>>,
) {
    let mut lines = vec![
        "utility_ai selection_strategies".to_string(),
        "The same action set is evaluated four times with different pickers.".to_string(),
        String::new(),
    ];

    for (name, active, trace) in &agents {
        lines.push(format!(
            "{} -> {}",
            name.as_str(),
            active.label.clone().unwrap_or_else(|| "none".into())
        ));
        if let Some(trace) = &trace.last {
            for action in &trace.actions {
                lines.push(format!(
                    "  {} score {:.2} momentum {:.2}",
                    action.label, action.final_score, action.momentum_score
                ));
            }
        }
        lines.push(String::new());
    }

    text.0 = lines.join("\n");
}
