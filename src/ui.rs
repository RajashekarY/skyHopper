use crate::player::{Player, ResetLevel, COIN_COUNT};
use crate::prefs::{Prefs, Quality};
use crate::GameState;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};

#[derive(Component, Clone, Copy)]
pub enum Btn {
    Quality(Quality),
    DontAsk,
    AskOnLaunch,
    Vsync,
    Start,
    Resume,
    Quit,
}

/// The "Don't ask me again" checkbox; only becomes a pref when Start is pressed.
#[derive(Resource, Default)]
pub struct DontAsk(bool);

#[derive(Resource, Default)]
pub struct Banner {
    text: String,
    timer: f32,
}

impl Banner {
    pub fn show(&mut self, text: String, secs: f32) {
        (self.text, self.timer) = (text, secs);
    }
}

#[derive(Component)]
pub struct HudText;
#[derive(Component)]
pub struct BannerText;
#[derive(Component)]
pub struct FpsText;

fn text(s: &str, size: f32) -> impl Bundle {
    (Text::new(s), TextFont { font_size: size.into(), ..default() })
}

fn button(b: Btn) -> impl Bundle {
    (
        b,
        Button,
        Text::default(),
        TextFont { font_size: 22.0.into(), ..default() },
        Node { padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)), width: Val::Px(560.0), ..default() },
        BackgroundColor(Color::BLACK),
    )
}

fn panel(state: GameState) -> impl Bundle {
    (
        DespawnOnExit(state),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.05, 0.7)),
    )
}

pub fn spawn_mode_select(mut commands: Commands) {
    commands.spawn((panel(GameState::ModeSelect), children![
        text("SKYHOP", 56.0),
        text("Choose a run mode for this computer", 24.0),
        button(Btn::Quality(Quality::Low)),
        button(Btn::Quality(Quality::Medium)),
        button(Btn::Quality(Quality::High)),
        button(Btn::DontAsk),
        button(Btn::Start),
        text("Keys: 1 / 2 / 3 pick mode   D toggle checkbox   Enter start   (gamepad: D-pad, A)", 16.0),
        text("You can change this any time: Esc > Preferences", 16.0),
    ]));
}

pub fn spawn_pause(mut commands: Commands) {
    commands.spawn((panel(GameState::Paused), children![
        text("Paused - Preferences", 40.0),
        button(Btn::Quality(Quality::Low)),
        button(Btn::Quality(Quality::Medium)),
        button(Btn::Quality(Quality::High)),
        button(Btn::AskOnLaunch),
        button(Btn::Vsync),
        button(Btn::Resume),
        button(Btn::Quit),
        text("Move: WASD / left stick   Jump (double jump): Space / A   Camera: mouse / right stick", 16.0),
        text("Hold right mouse / LT: look through Pip's eyes   F3: FPS   Esc / Start: pause", 16.0),
    ]));
}

pub fn spawn_hud(mut commands: Commands) {
    commands.spawn((HudText, text("", 24.0), Node { position_type: PositionType::Absolute, left: Val::Px(12.0), top: Val::Px(8.0), ..default() }));
    commands.spawn((FpsText, text("", 18.0), Node { position_type: PositionType::Absolute, right: Val::Px(12.0), top: Val::Px(8.0), ..default() }));
    commands.spawn((
        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), top: Val::Percent(30.0), justify_content: JustifyContent::Center, ..default() },
        children![(BannerText, text("", 36.0), TextLayout::justify(Justify::Center))],
    ));
}

fn label(b: Btn, prefs: &Prefs, dont_ask: bool) -> (String, bool) {
    let check = |on: bool| if on { "[x]" } else { "[ ]" };
    match b {
        Btn::Quality(q) => (
            match q {
                Quality::Low => "Low     - integrated GPUs, max FPS",
                Quality::Medium => "Medium  - shadows + FXAA",
                Quality::High => "High    - MSAA, bloom, far view",
            }
            .into(),
            prefs.quality == q,
        ),
        Btn::DontAsk => (format!("{} Don't ask me again", check(dont_ask)), dont_ask),
        Btn::AskOnLaunch => (format!("{} Ask for run mode on launch", check(prefs.ask_on_launch)), prefs.ask_on_launch),
        Btn::Vsync => (format!("{} VSync (off = maximum FPS)", check(prefs.vsync)), prefs.vsync),
        Btn::Start => ("Start".into(), false),
        Btn::Resume => ("Resume".into(), false),
        Btn::Quit => ("Quit".into(), false),
    }
}

pub fn refresh_buttons(prefs: Res<Prefs>, dont_ask: Res<DontAsk>, mut q: Query<(&Btn, &mut Text, &mut BackgroundColor)>) {
    for (&b, mut t, mut bg) in &mut q {
        let (s, active) = label(b, &prefs, dont_ask.0);
        t.set_if_neq(Text(s));
        let c = if active { Color::srgb(0.15, 0.5, 0.25) } else { Color::srgb(0.15, 0.15, 0.2) };
        bg.set_if_neq(BackgroundColor(c));
    }
}

pub fn on_activate(
    ev: On<Activate>,
    q: Query<&Btn>,
    mut prefs: ResMut<Prefs>,
    mut dont_ask: ResMut<DontAsk>,
    mut next: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(&b) = q.get(ev.entity) else { return };
    match b {
        Btn::Quality(q) => prefs.quality = q,
        Btn::DontAsk => dont_ask.0 = !dont_ask.0,
        Btn::AskOnLaunch => prefs.ask_on_launch = !prefs.ask_on_launch,
        Btn::Vsync => prefs.vsync = !prefs.vsync,
        Btn::Start => {
            prefs.ask_on_launch = !dont_ask.0;
            next.set(GameState::Playing);
        }
        Btn::Resume => next.set(GameState::Playing),
        Btn::Quit => {
            exit.write(AppExit::Success);
        }
    }
}

/// Keyboard / gamepad shortcuts for both menus.
pub fn menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    state: Res<State<GameState>>,
    mut prefs: ResMut<Prefs>,
    mut dont_ask: ResMut<DontAsk>,
    mut next: ResMut<NextState<GameState>>,
) {
    let pad = |b| pads.iter().any(|p| p.just_pressed(b));
    let idx = Quality::ALL.iter().position(|&q| q == prefs.quality).unwrap_or(1);
    for (k, q) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3].into_iter().zip(Quality::ALL) {
        if keys.just_pressed(k) {
            prefs.quality = q;
        }
    }
    if pad(GamepadButton::DPadLeft) {
        prefs.quality = Quality::ALL[idx.saturating_sub(1)];
    }
    if pad(GamepadButton::DPadRight) {
        prefs.quality = Quality::ALL[(idx + 1).min(2)];
    }
    let in_mode_select = *state.get() == GameState::ModeSelect;
    if in_mode_select && (keys.just_pressed(KeyCode::KeyD) || pad(GamepadButton::West)) {
        dont_ask.0 = !dont_ask.0;
    }
    if keys.just_pressed(KeyCode::Enter) || pad(GamepadButton::South) {
        if in_mode_select {
            prefs.ask_on_launch = !dont_ask.0;
        }
        next.set(GameState::Playing);
    }
}

pub fn pause_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) || pads.iter().any(|p| p.just_pressed(GamepadButton::Start)) {
        match state.get() {
            GameState::Playing => next.set(GameState::Paused),
            GameState::Paused => next.set(GameState::Playing),
            _ => {}
        }
    }
}

pub fn won_input(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut reset: ResMut<ResetLevel>, mut banner: ResMut<Banner>, mut next: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Enter) || pads.iter().any(|p| p.just_pressed(GamepadButton::South)) {
        reset.0 = true;
        banner.timer = 0.0;
        next.set(GameState::Playing);
    }
}

pub fn update_hud(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    diag: Res<DiagnosticsStore>,
    prefs: Res<Prefs>,
    mut banner: ResMut<Banner>,
    player: Single<&Player>,
    mut hud: Single<&mut Text, With<HudText>>,
    mut fps: Single<(&mut Text, &mut Visibility), (With<FpsText>, Without<HudText>)>,
    mut btext: Single<&mut Text, (With<BannerText>, Without<HudText>, Without<FpsText>)>,
) {
    hud.set_if_neq(Text(format!("Coins {}/{COIN_COUNT}   Hearts {}", player.coins, player.hearts)));
    banner.timer -= time.delta_secs();
    btext.set_if_neq(Text(if banner.timer > 0.0 { banner.text.clone() } else { String::new() }));

    let (ft, fv) = &mut *fps;
    if keys.just_pressed(KeyCode::F3) {
        **fv = if **fv == Visibility::Hidden { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Some(v) = diag.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()) {
        ft.set_if_neq(Text(format!("{v:.0} FPS  ({})", prefs.quality.name())));
    }
}
