use bevy::app::AppExit;
use bevy::prelude::*;
use saddle_ai_utility_ai::{
    DecisionMomentum, EvaluationMode, EvaluationPolicy, UtilityAction, UtilityAgent,
    UtilityAiBudget,
};
use saddle_pane::prelude::*;

#[derive(Resource, Clone, Copy)]
pub struct ExampleLifetime {
    pub duration_seconds: f32,
}

#[derive(Resource, Clone, Pane)]
#[pane(title = "Utility AI Demo")]
pub struct UtilityExamplePane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    pub time_scale: f32,
    #[pane(slider, min = 0.05, max = 2.0, step = 0.05)]
    pub evaluation_interval_seconds: f32,
    #[pane(slider, min = 0.0, max = 0.5, step = 0.01)]
    pub jitter_fraction: f32,
    #[pane(slider, min = 0.0, max = 0.4, step = 0.01)]
    pub active_action_bonus: f32,
    #[pane(slider, min = 0.0, max = 0.3, step = 0.01)]
    pub hysteresis_band: f32,
    #[pane(slider, min = 0.1, max = 2.0, step = 0.05)]
    pub action_weight: f32,
    #[pane(slider, min = 1.0, max = 2048.0, step = 1.0)]
    pub max_agents_per_update: usize,
}

impl Default for UtilityExamplePane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            evaluation_interval_seconds: 0.25,
            jitter_fraction: 0.0,
            active_action_bonus: 0.1,
            hysteresis_band: 0.05,
            action_weight: 1.0,
            max_agents_per_update: 512,
        }
    }
}

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn configure_2d_example(app: &mut App, title: &str, duration_seconds: f32) {
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));
    app.insert_resource(ExampleLifetime { duration_seconds });
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.into(),
            resolution: (1280, 720).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(pane_plugins());
    app.register_pane::<UtilityExamplePane>();
    app.add_systems(Startup, (spawn_backdrop, spawn_camera));
    app.add_systems(Update, (auto_exit, sync_pane_to_runtime));
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Name::new("Camera"), Camera2d));
}

fn spawn_backdrop(mut commands: Commands) {
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.08, 0.10, 0.12), Vec2::new(1600.0, 900.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));
    commands.spawn((
        Name::new("Decision Lane Left"),
        Sprite::from_color(Color::srgba(0.18, 0.25, 0.34, 0.55), Vec2::new(500.0, 620.0)),
        Transform::from_xyz(-270.0, -30.0, -10.0),
    ));
    commands.spawn((
        Name::new("Decision Lane Right"),
        Sprite::from_color(Color::srgba(0.22, 0.18, 0.12, 0.45), Vec2::new(500.0, 620.0)),
        Transform::from_xyz(270.0, -30.0, -10.0),
    ));
    commands.spawn((
        Name::new("Status Strip"),
        Sprite::from_color(Color::srgba(0.95, 0.72, 0.32, 0.14), Vec2::new(1280.0, 96.0)),
        Transform::from_xyz(0.0, -300.0, -5.0),
    ));
}

fn auto_exit(time: Res<Time>, lifetime: Res<ExampleLifetime>, mut exit: MessageWriter<AppExit>) {
    if time.elapsed_secs() >= lifetime.duration_seconds {
        exit.write(AppExit::Success);
    }
}

fn sync_pane_to_runtime(
    pane: Res<UtilityExamplePane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut budget: ResMut<UtilityAiBudget>,
    mut policies: Query<&mut EvaluationPolicy, With<UtilityAgent>>,
    mut momentum: Query<&mut DecisionMomentum, With<UtilityAgent>>,
    mut actions: Query<&mut UtilityAction>,
) {
    if !pane.is_changed() {
        return;
    }

    virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    budget.max_agents_per_update = pane.max_agents_per_update.max(1);

    for mut policy in &mut policies {
        if policy.mode == EvaluationMode::Interval {
            policy.base_interval_seconds = pane.evaluation_interval_seconds.max(0.01);
        }
        policy.jitter_fraction = pane.jitter_fraction.clamp(0.0, 1.0);
    }

    for mut entry in &mut momentum {
        entry.active_action_bonus = pane.active_action_bonus.max(0.0);
        entry.hysteresis_band = pane.hysteresis_band.max(0.0);
    }

    for mut action in &mut actions {
        action.weight = pane.action_weight.max(0.0);
    }
}

pub fn ui_text_node(top: f32, left: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: px(top),
        left: px(left),
        ..default()
    }
}
