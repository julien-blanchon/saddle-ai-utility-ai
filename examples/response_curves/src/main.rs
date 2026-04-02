use saddle_ai_utility_ai_example_support as support;

use bevy::prelude::*;
use saddle_ai_utility_ai::ResponseCurve;
use support::{configure_2d_example, ui_text_node};

#[derive(Component)]
struct OverlayText;

#[derive(Resource)]
struct CurveGallery(Vec<(String, Color, ResponseCurve, f32)>);

fn main() {
    let mut app = App::new();
    configure_2d_example(&mut app, "utility_ai response curves", 6.0);
    app.insert_resource(CurveGallery(vec![
        (
            "linear".into(),
            Color::srgb(0.90, 0.40, 0.34),
            ResponseCurve::Linear,
            130.0,
        ),
        (
            "logistic".into(),
            Color::srgb(0.22, 0.78, 0.92),
            ResponseCurve::Logistic {
                midpoint: 0.5,
                steepness: 10.0,
            },
            40.0,
        ),
        (
            "gaussian".into(),
            Color::srgb(0.92, 0.78, 0.28),
            ResponseCurve::Gaussian {
                mean: 0.5,
                deviation: 0.16,
            },
            -50.0,
        ),
        (
            "inverse logistic".into(),
            Color::srgb(0.55, 0.86, 0.35),
            ResponseCurve::InverseLogistic {
                midpoint: 0.45,
                steepness: 8.0,
            },
            -140.0,
        ),
    ]));
    app.add_systems(Startup, setup);
    app.add_systems(Update, draw_curves);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Overlay"),
        OverlayText,
        Text::new(
            "utility_ai response_curves\nEach line maps normalized input [0, 1] to output [0, 1].",
        ),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        Node {
            width: px(540.0),
            ..ui_text_node(18.0, 18.0)
        },
    ));
}

fn draw_curves(mut gizmos: Gizmos, gallery: Res<CurveGallery>) {
    for (label, color, curve, offset_y) in &gallery.0 {
        gizmos.line_2d(
            Vec2::new(-480.0, *offset_y),
            Vec2::new(-120.0, *offset_y),
            Color::srgb(0.25, 0.28, 0.33),
        );
        gizmos.line_2d(
            Vec2::new(-480.0, *offset_y),
            Vec2::new(-480.0, *offset_y + 180.0),
            Color::srgb(0.25, 0.28, 0.33),
        );

        let mut previous = None;
        for step in 0..=80 {
            let input = step as f32 / 80.0;
            let x = -480.0 + input * 360.0;
            let y = *offset_y + curve.sample(input) * 180.0;
            let point = Vec2::new(x, y);
            if let Some(previous) = previous {
                gizmos.line_2d(previous, point, *color);
            }
            previous = Some(point);
        }

        gizmos.line_2d(
            Vec2::new(-105.0, *offset_y + 90.0),
            Vec2::new(-90.0, *offset_y + 90.0),
            *color,
        );
        gizmos.line_2d(
            Vec2::new(-90.0, *offset_y + 90.0),
            Vec2::new(-85.0 + label.len() as f32, *offset_y + 90.0),
            *color,
        );
    }
}
