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
struct CadenceDriver(usize);

fn main() {
    let mut app = App::new();
    configure_2d_example(&mut app, "utility_ai realtime cadence", 7.0);
    app.insert_resource(UtilityAiBudget {
        max_agents_per_update: 3,
    });
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
            width: px(720.0),
            ..ui_text_node(18.0, 18.0)
        },
    ));

    for index in 0..12 {
        commands
            .spawn((
                Name::new(format!("Cadence Agent {index}")),
                UtilityAgent {
                    selection_seed: 100 + index as u64,
                    ..default()
                },
                EvaluationPolicy {
                    base_interval_seconds: if index < 6 { 0.08 } else { 0.25 },
                    jitter_fraction: 0.35,
                    ..default()
                },
            ))
            .with_children(|agent| {
                agent
                    .spawn((Name::new("Pulse"), UtilityAction::new("pulse")))
                    .with_children(|action| {
                        action.spawn((
                            Name::new("Pulse Input"),
                            CadenceDriver(index),
                            UtilityConsideration::new("cadence", ResponseCurve::Linear),
                            ConsiderationInput::default(),
                        ));
                    });
            });
    }
}

fn animate_inputs(time: Res<Time>, mut inputs: Query<(&CadenceDriver, &mut ConsiderationInput)>) {
    let seconds = time.elapsed_secs();
    for (driver, mut input) in &mut inputs {
        let phase = driver.0 as f32 * 0.25;
        input.value = Some(((seconds * 1.4 + phase).sin() * 0.5 + 0.5).clamp(0.0, 1.0));
    }
}

fn update_overlay(
    mut text: Single<&mut Text, With<OverlayText>>,
    stats: Res<UtilityAiStats>,
    policies: Query<&EvaluationPolicy, With<UtilityAgent>>,
) {
    let mut next_times = policies
        .iter()
        .map(|policy| policy.next_evaluation_at_seconds)
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    next_times.sort_by(|left, right| left.total_cmp(right));

    text.0 = format!(
        "utility_ai realtime_cadence\n\
         Budget spreads due agents across frames.\n\n\
         evaluated this frame: {}\n\
         scored actions: {}\n\
         skipped due to budget: {}\n\
         eval time: {}us (avg {:.0}us, peak {}us)\n\
         next evaluation timestamps: {:?}",
        stats.evaluated_agents,
        stats.scored_actions,
        stats.skipped_due_to_budget,
        stats.last_evaluation_time_micros,
        stats.average_evaluation_time_micros,
        stats.peak_evaluation_time_micros,
        &next_times[..next_times.len().min(6)],
    );
}
