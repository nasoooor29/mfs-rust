use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use maze_runner::protocol::{encode, ClientMessage, InputState};

use crate::resources::{Connection, Controls, ViewState};

pub fn update_mouse_capture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = window.get_single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

pub fn update_view_input(
    mut motion: EventReader<MouseMotion>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    controls: Res<Controls>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut view: ResMut<ViewState>,
) {
    let mouse_delta = motion
        .read()
        .fold(Vec2::ZERO, |sum, event| sum + event.delta);
    let captured = window
        .get_single()
        .is_ok_and(|window| !window.cursor.visible);
    view.mouse_turn = if captured {
        (mouse_delta.x * 0.025).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    let keyboard_pitch =
        f32::from(keys.pressed(controls.look_up)) - f32::from(keys.pressed(controls.look_down));
    let mouse_pitch = if captured {
        -mouse_delta.y * 0.0025
    } else {
        0.0
    };
    view.pitch =
        (view.pitch + keyboard_pitch * time.delta_seconds() * 1.5 + mouse_pitch).clamp(-1.35, 1.35);
}

pub fn send_input(
    keys: Res<ButtonInput<KeyCode>>,
    controls: Res<Controls>,
    view: Res<ViewState>,
    mut connection: ResMut<Connection>,
) {
    let axis =
        |positive, negative| f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative));
    let input = InputState {
        forward: axis(controls.forward, controls.backward),
        strafe: axis(controls.right, controls.left),
        turn: (axis(controls.turn_right, controls.turn_left) + view.mouse_turn).clamp(-1.0, 1.0),
        pitch: axis(controls.look_up, controls.look_down),
        shoot: keys.pressed(controls.shoot),
    };
    connection.sequence = connection.sequence.wrapping_add(1);
    if let Ok(bytes) = encode(&ClientMessage::Input {
        sequence: connection.sequence,
        input,
    }) {
        let _ = connection.socket.send(&bytes);
    }
}
