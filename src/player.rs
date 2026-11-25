use crate::level::*;
use crate::ui::Banner;
use crate::GameState;
use bevy::prelude::*;

pub const HALF: Vec3 = Vec3::new(0.35, 0.5, 0.35);
const RUN: f32 = 8.0;
const GRAVITY: f32 = 30.0;
const JUMP_V: f32 = 12.0;
const DOUBLE_V: f32 = 10.0;
const COYOTE: f32 = 0.1;
const BUFFER: f32 = 0.12;
pub const COIN_COUNT: usize = 12;

#[derive(Component, Default)]
pub struct Player {
    pub vel: Vec3,
    pub grounded: bool,
    pub jump: JumpState,
    pub hearts: u8,
    pub coins: u32,
    pub facing: f32,
}

#[derive(Component)]
pub struct BlobShadow;

/// Written in Update (input + camera), read in FixedUpdate.
#[derive(Resource, Default)]
pub struct Controls {
    pub mv: Vec2,
    pub jump_pressed: bool, // latched until a fixed step consumes it
    pub jump_held: bool,
    pub yaw: f32,
    pub pov: bool,
}

#[derive(Resource, Default)]
pub struct ResetLevel(pub bool);

#[derive(Default)]
pub struct JumpState {
    coyote: f32,
    buffer: f32,
    air_jumps: u8,
}

impl JumpState {
    /// Returns the jump velocity if a jump starts on this step.
    pub fn step(&mut self, dt: f32, grounded: bool, pressed: bool) -> Option<f32> {
        if grounded {
            self.coyote = COYOTE;
            self.air_jumps = 1;
        } else {
            self.coyote -= dt;
        }
        self.buffer = if pressed { BUFFER } else { self.buffer - dt };
        if self.buffer <= 0.0 {
            return None;
        }
        if self.coyote > 0.0 {
            (self.coyote, self.buffer) = (0.0, 0.0);
            Some(JUMP_V)
        } else if pressed && self.air_jumps > 0 {
            self.air_jumps -= 1;
            self.buffer = 0.0;
            Some(DOUBLE_V)
        } else {
            None
        }
    }
}

pub fn spawn_player(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let eye = meshes.add(Sphere::new(0.08));
    let white = mats.add(Color::WHITE);
    commands.spawn((
        Player { hearts: 3, ..default() },
        Sim { prev: SPAWN, pos: SPAWN },
        Transform::from_translation(SPAWN),
        Mesh3d(meshes.add(Capsule3d::new(0.35, 0.3))),
        MeshMaterial3d(mats.add(Color::srgb(1.0, 0.45, 0.1))),
        children![
            (Mesh3d(eye.clone()), MeshMaterial3d(white.clone()), Transform::from_xyz(-0.13, 0.2, -0.3)),
            (Mesh3d(eye), MeshMaterial3d(white), Transform::from_xyz(0.13, 0.2, -0.3)),
        ],
    ));
    commands.spawn((
        BlobShadow,
        Mesh3d(meshes.add(Circle::new(0.4))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.45),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

pub fn read_input(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut ctl: ResMut<Controls>) {
    let key = |k: KeyCode| if keys.pressed(k) { 1.0f32 } else { 0.0 };
    let mut mv = Vec2::new(
        key(KeyCode::KeyD).max(key(KeyCode::ArrowRight)) - key(KeyCode::KeyA).max(key(KeyCode::ArrowLeft)),
        key(KeyCode::KeyW).max(key(KeyCode::ArrowUp)) - key(KeyCode::KeyS).max(key(KeyCode::ArrowDown)),
    );
    let mut pressed = keys.just_pressed(KeyCode::Space);
    let mut held = keys.pressed(KeyCode::Space);
    for pad in &pads {
        mv += pad.left_stick();
        pressed |= pad.just_pressed(GamepadButton::South);
        held |= pad.pressed(GamepadButton::South);
    }
    ctl.mv = mv.clamp_length_max(1.0);
    ctl.jump_pressed |= pressed;
    ctl.jump_held = held;
}

pub fn player_step(time: Res<Time>, mut ctl: ResMut<Controls>, solids: Res<Solids>, player: Single<(&mut Player, &mut Sim)>) {
    let dt = time.delta_secs();
    let (mut p, mut sim) = player.into_inner();
    let pressed = std::mem::take(&mut ctl.jump_pressed) && !ctl.pov;
    let mv = if ctl.pov { Vec2::ZERO } else { ctl.mv };

    let rot = Quat::from_rotation_y(ctl.yaw);
    let wish = (rot * Vec3::X * mv.x + rot * Vec3::NEG_Z * mv.y) * RUN;
    let accel = if p.grounded { 60.0 } else { 25.0 };
    let h = Vec2::new(p.vel.x, p.vel.z);
    let h = h + (Vec2::new(wish.x, wish.z) - h).clamp_length_max(accel * dt);
    (p.vel.x, p.vel.z) = (h.x, h.y);
    if mv.length() > 0.1 {
        p.facing = (-wish.x).atan2(-wish.z);
    }

    let grounded = p.grounded;
    if let Some(v) = p.jump.step(dt, grounded, pressed) {
        p.vel.y = v;
    }
    // Faster fall and short hop when jump is released: the standard "tight jump" feel.
    let g = GRAVITY * if p.vel.y < 0.0 { 1.6 } else if !ctl.jump_held { 2.5 } else { 1.0 };
    p.vel.y = (p.vel.y - g * dt).max(-30.0);

    let (body, hit, grounded) = move_and_collide(Aabb { c: sim.pos, h: HALF }, p.vel * dt, &solids.0);
    if hit.x { p.vel.x = 0.0 }
    if hit.y { p.vel.y = 0.0 }
    if hit.z { p.vel.z = 0.0 }
    p.grounded = grounded;
    sim.prev = sim.pos;
    sim.pos = body.c;
}

pub fn walker_step(time: Res<Time>, mut q: Query<(&mut Walker, &mut Sim)>) {
    for (mut w, mut sim) in &mut q {
        w.t += time.delta_secs() * 0.25;
        let f = w.t % 2.0;
        sim.prev = sim.pos;
        sim.pos = w.a.lerp(w.b, if f > 1.0 { 2.0 - f } else { f });
    }
}

pub fn interactions(
    player: Single<(&mut Player, &mut Sim), Without<Walker>>,
    mut walkers: Query<(&mut Walker, &Sim, &mut Visibility), Without<Coin>>,
    mut coins: Query<(&Transform, &mut Visibility), (With<Coin>, Without<Walker>)>,
    hazards: Query<&Hazard>,
    goal: Single<&Goal>,
    mut banner: ResMut<Banner>,
    mut reset: ResMut<ResetLevel>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (mut p, mut sim) = player.into_inner();
    let body = Aabb { c: sim.pos, h: HALF };
    let mut hurt = sim.pos.y < -20.0 || hazards.iter().any(|h| h.0.overlaps(&body));

    for (mut w, wsim, mut vis) in &mut walkers {
        if w.alive && body.overlaps(&Aabb { c: wsim.pos, h: WALKER_HALF }) {
            if p.vel.y < 0.0 && sim.pos.y > wsim.pos.y + 0.3 {
                w.alive = false;
                *vis = Visibility::Hidden;
                p.vel.y = 10.0;
            } else {
                hurt = true;
            }
        }
    }
    for (t, mut vis) in &mut coins {
        if *vis != Visibility::Hidden && t.translation.distance(sim.pos) < 0.8 {
            *vis = Visibility::Hidden;
            p.coins += 1;
        }
    }
    if goal.0.overlaps(&body) {
        banner.show(format!("Level complete! Coins {}/{COIN_COUNT}\nPress Enter / (A) to play again", p.coins), f32::INFINITY);
        next.set(GameState::Won);
    } else if hurt {
        p.hearts -= 1;
        if p.hearts == 0 {
            banner.show("Out of hearts! Restarting...".into(), 2.0);
            reset.0 = true;
        } else {
            banner.show("Ouch!".into(), 1.0);
        }
        (sim.prev, sim.pos, p.vel) = (SPAWN, SPAWN, Vec3::ZERO);
    }
}

pub fn do_reset(
    mut reset: ResMut<ResetLevel>,
    player: Single<(&mut Player, &mut Sim), Without<Walker>>,
    mut walkers: Query<(&mut Walker, &mut Visibility)>,
    mut coins: Query<&mut Visibility, (With<Coin>, Without<Walker>)>,
) {
    if !std::mem::take(&mut reset.0) {
        return;
    }
    let (mut p, mut sim) = player.into_inner();
    *p = Player { hearts: 3, ..default() };
    (sim.prev, sim.pos) = (SPAWN, SPAWN);
    for (mut w, mut vis) in &mut walkers {
        (w.alive, w.t) = (true, 0.0);
        *vis = Visibility::Inherited;
    }
    for mut vis in &mut coins {
        *vis = Visibility::Inherited;
    }
}

/// Smooths fixed-step motion at any frame rate, and updates the blob shadow.
pub fn interpolate(
    fixed: Res<Time<Fixed>>,
    time: Res<Time>,
    solids: Res<Solids>,
    mut q: Query<(&Sim, &mut Transform, Option<&Player>)>,
    mut spin: Query<&mut Transform, (Or<(With<Coin>, With<Goal>)>, Without<Sim>)>,
    shadow: Single<(&mut Transform, &mut Visibility), (With<BlobShadow>, Without<Sim>, Without<Coin>, Without<Goal>)>,
) {
    let a = fixed.overstep_fraction();
    let mut player_pos = None;
    for (sim, mut t, player) in &mut q {
        t.translation = sim.prev.lerp(sim.pos, a);
        if let Some(p) = player {
            t.rotation = t.rotation.slerp(Quat::from_rotation_y(p.facing), 0.3);
            player_pos = Some(t.translation);
        }
    }
    for mut t in &mut spin {
        t.rotate_y(time.delta_secs() * 3.0);
    }
    let (mut st, mut vis) = shadow.into_inner();
    let hit = player_pos.and_then(|p| {
        let feet = p - Vec3::Y * HALF.y;
        solids.0.iter().filter_map(|s| s.ray(feet, Vec3::NEG_Y)).reduce(f32::min).map(|d| feet - Vec3::Y * (d - 0.02))
    });
    *vis = if hit.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    if let Some(h) = hit {
        st.translation = h;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f32 = 1.0 / 64.0;

    fn left_ground_for(steps: usize) -> JumpState {
        let mut j = JumpState::default();
        j.step(DT, true, false);
        for _ in 0..steps {
            j.step(DT, false, false);
        }
        j
    }

    #[test]
    fn coyote_jump_shortly_after_leaving_ledge() {
        assert_eq!(left_ground_for(3).step(DT, false, true), Some(JUMP_V));
    }

    #[test]
    fn late_press_uses_double_jump_once() {
        let mut j = left_ground_for(13);
        assert_eq!(j.step(DT, false, true), Some(DOUBLE_V));
        j.step(DT, false, false);
        assert_eq!(j.step(DT, false, true), None);
    }

    #[test]
    fn buffered_press_jumps_on_landing() {
        let mut j = left_ground_for(20);
        j.step(DT, false, true); // double jump
        j.step(DT, false, false);
        assert_eq!(j.step(DT, false, true), None); // no air jumps left; buffered
        j.step(DT, false, false);
        assert_eq!(j.step(DT, true, false), Some(JUMP_V));
    }
}
