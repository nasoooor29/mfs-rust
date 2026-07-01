use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

#[derive(Component)]
pub struct FpsText;

#[derive(Component)]
pub struct ScoreText;

pub fn spawn_ui(commands: &mut Commands) {
    commands.spawn((
        TextBundle::from_sections([TextSection::new(
            "FPS: --",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )])
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(8.0),
            ..default()
        }),
        FpsText,
    ));
    commands.spawn((
        TextBundle::from_section(
            "Waiting for snapshot",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(34.0),
            ..default()
        }),
        ScoreText,
    ));
    commands.spawn(
        TextBundle::from_section(
            "WASD move  |  mouse/arrow keys look  |  SPACE shoot  |  ESC release mouse",
            TextStyle {
                font_size: 16.0,
                color: Color::srgb(0.75, 0.8, 0.82),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(8.0),
            ..default()
        }),
    );
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(4.0),
            height: Val::Px(4.0),
            ..default()
        },
        background_color: Color::WHITE.into(),
        transform: Transform::from_xyz(-2.0, -2.0, 0.0),
        z_index: ZIndex::Global(20),
        ..default()
    });
}

pub fn update_ui(diagnostics: Res<DiagnosticsStore>, mut text: Query<&mut Text, With<FpsText>>) {
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        if let Ok(mut text) = text.get_single_mut() {
            text.sections[0].value = format!("FPS: {fps:.0}");
        }
    }
}
