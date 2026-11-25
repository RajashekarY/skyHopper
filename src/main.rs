// Bevy systems take many query params; these lints fight the ECS style.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod camera;
mod level;
mod player;
mod prefs;
mod ui;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::camera::Hdr;
use bevy::window::{PresentMode, PrimaryWindow};
use prefs::{Prefs, Quality};

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GameState {
    ModeSelect,
    Playing,
    Paused,
    Won,
}

const SKY: Color = Color::srgb(0.55, 0.78, 0.95);

fn main() {
    let prefs = prefs::load();
    let start = if prefs.ask_on_launch { GameState::ModeSelect } else { GameState::Playing };
    App::new()
        // wgpu already tries every backend (Vulkan/DX12/Metal/GL) and honours WGPU_BACKEND.
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window { title: "Skyhop".into(), ..default() }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .insert_resource(prefs)
        .insert_resource(ClearColor(SKY))
        .insert_resource(GlobalAmbientLight { brightness: 400.0, ..default() })
        .init_resource::<player::Controls>()
        .init_resource::<player::ResetLevel>()
        .init_resource::<camera::Orbit>()
        .init_resource::<ui::Banner>()
        .init_resource::<ui::DontAsk>()
        .insert_state(start)
        .add_systems(Startup, (spawn_world, level::spawn_level, player::spawn_player, ui::spawn_hud))
        .add_systems(OnEnter(GameState::ModeSelect), ui::spawn_mode_select)
        .add_systems(OnEnter(GameState::Paused), ui::spawn_pause)
        .add_systems(OnEnter(GameState::Playing), camera::grab_cursor)
        .add_systems(OnExit(GameState::Playing), camera::release_cursor)
        .add_systems(
            FixedUpdate,
            (player::do_reset, player::walker_step, player::player_step, player::interactions)
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                apply_prefs.run_if(resource_changed::<Prefs>),
                ui::refresh_buttons,
                ui::update_hud,
                ui::pause_toggle,
                ui::menu_keys.run_if(|s: Res<State<GameState>>| matches!(s.get(), GameState::ModeSelect | GameState::Paused)),
                ui::won_input.run_if(in_state(GameState::Won)),
                (player::read_input, player::interpolate, camera::orbit_camera).chain().run_if(in_state(GameState::Playing)),
            ),
        )
        .add_observer(ui::on_activate)
        .run();
}

fn spawn_world(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 4.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, ..default() },
        Transform::from_xyz(4.0, 10.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Turns render features on/off for the chosen run mode, and saves the prefs.
fn apply_prefs(
    prefs: Res<Prefs>,
    mut commands: Commands,
    cam: Single<(Entity, &mut Projection), With<Camera3d>>,
    light: Single<(Entity, &mut DirectionalLight)>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut decor: Query<(&level::Decor, &mut Visibility)>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    prefs::save(&prefs);
    window.present_mode = if prefs.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };

    let q = prefs.quality;
    let (far, fog_start) = match q {
        Quality::Low => (80.0, 35.0),
        Quality::Medium => (150.0, 70.0),
        Quality::High => (300.0, 120.0),
    };
    let (cam, mut proj) = cam.into_inner();
    if let Projection::Perspective(p) = &mut *proj {
        p.far = far;
    }
    let mut c = commands.entity(cam);
    c.insert(DistanceFog { color: SKY, falloff: FogFalloff::Linear { start: fog_start, end: far }, ..default() });
    match q {
        Quality::Low => c.insert((Msaa::Off, Fxaa { enabled: false, ..default() })).remove::<(Hdr, Bloom)>(),
        Quality::Medium => c.insert((Msaa::Off, Fxaa::default())).remove::<(Hdr, Bloom)>(),
        Quality::High => c.insert((Msaa::Sample4, Fxaa { enabled: false, ..default() }, Hdr, Bloom::NATURAL)),
    };

    let (light, mut dl) = light.into_inner();
    dl.shadow_maps_enabled = q != Quality::Low;
    let (size, cascades, dist) = if q == Quality::High { (2048, 3, 120.0) } else { (1024, 1, 40.0) };
    shadow_map.size = size;
    commands.entity(light).insert(CascadeShadowConfigBuilder { num_cascades: cascades, maximum_distance: dist, ..default() }.build());

    for (d, mut vis) in &mut decor {
        let show = match q {
            Quality::Low => false,
            Quality::Medium => d.0 % 2 == 0,
            Quality::High => true,
        };
        *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
}
