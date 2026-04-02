use bevy::app::AppExit;
use bevy::prelude::*;

#[derive(Resource, Clone, Copy)]
pub struct ExampleLifetime {
    pub duration_seconds: f32,
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
    app.add_systems(Startup, spawn_camera);
    app.add_systems(Update, auto_exit);
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Name::new("Camera"), Camera2d));
}

fn auto_exit(
    time: Res<Time>,
    lifetime: Res<ExampleLifetime>,
    mut exit: MessageWriter<AppExit>,
) {
    if time.elapsed_secs() >= lifetime.duration_seconds {
        exit.write(AppExit::Success);
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
