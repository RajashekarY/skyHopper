use crate::level::Solids;
use crate::player::{Controls, Player};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

#[derive(Resource)]
pub struct Orbit {
    yaw: f32,
    pitch: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: -0.35 }
    }
}

const DIST: f32 = 7.0;

pub fn orbit_camera(
    time: Res<Time>,
    mouse: Res<AccumulatedMouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    solids: Res<Solids>,
    mut orbit: ResMut<Orbit>,
    mut ctl: ResMut<Controls>,
    player: Single<(&Transform, &mut Visibility), With<Player>>,
    mut cam: Single<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    let mut look = mouse.delta * 0.003;
    let mut pov = buttons.pressed(MouseButton::Right);
    for pad in &pads {
        look += pad.right_stick() * Vec2::new(1.0, -1.0) * 2.5 * time.delta_secs();
        pov |= pad.pressed(GamepadButton::LeftTrigger2);
    }
    orbit.yaw -= look.x;
    orbit.pitch = (orbit.pitch - look.y).clamp(-1.3, if pov { 1.3 } else { 0.5 });
    (ctl.yaw, ctl.pov) = (orbit.yaw, pov);

    let rot = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let (pt, mut pvis) = player.into_inner();
    // POV mode: look through Pip's eyes; Pip is hidden so the body doesn't block the view.
    *pvis = if pov { Visibility::Hidden } else { Visibility::Inherited };
    let focus = pt.translation + Vec3::Y * if pov { 0.25 } else { 0.8 };
    let back = rot * Vec3::Z;
    let dist = if pov {
        0.0
    } else {
        // Pull the camera in front of any wall between it and Pip.
        let hit = solids.0.iter().filter_map(|s| s.ray(focus, back)).fold(DIST, f32::min);
        (hit - 0.3).max(0.5)
    };
    cam.translation = focus + back * dist;
    cam.rotation = rot;
}

pub fn grab_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
}

pub fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}
