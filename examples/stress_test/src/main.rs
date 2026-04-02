use saddle_ai_utility_ai_example_support as support;

use bevy::prelude::*;
use support::{configure_2d_example, ui_text_node};
use saddle_ai_utility_ai::{
    ConsiderationInput, EvaluationPolicy, UtilityAction, UtilityAgent, UtilityAiBudget,
    UtilityAiPlugin, UtilityAiStats, UtilityConsideration, ResponseCurve,
};

#[derive(Component)]
struct OverlayText;

#[derive(Component, Clone, Copy)]
struct StressInput {
    agent_index: usize,
    action_index: usize,
}

fn main() {
    let mut app = App::new();
    configure_2d_example(&mut app, "utility_ai stress test", 5.0);
    app.insert_resource(UtilityAiBudget {
        max_agents_per_update: 64,
    });
    app.add_plugins(UtilityAiPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, (animate_inputs, update_overlay, log_stats));
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
            width: px(720.0),
            ..ui_text_node(18.0, 18.0)
        },
    ));

    for agent_index in 0..180 {
        commands
            .spawn((
                Name::new(format!("Stress Agent {agent_index}")),
                UtilityAgent {
                    selection_seed: 900 + agent_index as u64,
                    ..default()
                },
                EvaluationPolicy {
                    base_interval_seconds: 0.12,
                    jitter_fraction: 0.25,
                    ..default()
                },
            ))
            .with_children(|agent| {
                for action_index in 0..3 {
                    agent
                        .spawn((
                            Name::new(format!("Action {action_index}")),
                            UtilityAction::new(format!("action_{action_index}")),
                        ))
                        .with_children(|action| {
                            action.spawn((
                                Name::new(format!("Input {action_index}")),
                                StressInput {
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

fn animate_inputs(time: Res<Time>, mut inputs: Query<(&StressInput, &mut ConsiderationInput)>) {
    let seconds = time.elapsed_secs();
    for (driver, mut input) in &mut inputs {
        let phase = driver.agent_index as f32 * 0.03;
        let speed = 0.8 + driver.action_index as f32 * 0.15;
        input.value = Some(((seconds * speed + phase).sin() * 0.5 + 0.5).clamp(0.0, 1.0));
    }
}

fn update_overlay(mut text: Single<&mut Text, With<OverlayText>>, stats: Res<UtilityAiStats>) {
    text.0 = format!(
        "utility_ai stress_test\n\
         180 agents, 3 actions each.\n\n\
         evaluated this frame: {}\n\
         scored actions: {}\n\
         skipped due to budget: {}\n\
         eval time: {}us (avg {:.0}us, peak {}us)",
        stats.evaluated_agents,
        stats.scored_actions,
        stats.skipped_due_to_budget,
        stats.last_evaluation_time_micros,
        stats.average_evaluation_time_micros,
        stats.peak_evaluation_time_micros,
    );
}

fn log_stats(time: Res<Time>, stats: Res<UtilityAiStats>, mut local_second: Local<u32>) {
    let elapsed_whole = time.elapsed_secs() as u32;
    if elapsed_whole != *local_second {
        *local_second = elapsed_whole;
        info!(
            "utility_ai stress: {} agents, {} actions, {}us",
            stats.evaluated_agents,
            stats.scored_actions,
            stats.last_evaluation_time_micros
        );
    }
}
