use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub c: Vec3,
    pub h: Vec3, // half extents
}

impl Aabb {
    pub fn new(c: Vec3, size: Vec3) -> Self {
        Self { c, h: size / 2.0 }
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        let d = (self.c - o.c).abs();
        let r = self.h + o.h;
        d.x < r.x - 1e-4 && d.y < r.y - 1e-4 && d.z < r.z - 1e-4
    }

    /// Ray/box slab test; returns distance along `dir` (unit) to the first hit.
    pub fn ray(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let inv = dir.recip();
        let t1 = (self.c - self.h - origin) * inv;
        let t2 = (self.c + self.h - origin) * inv;
        let near = t1.min(t2).max_element();
        let far = t1.max(t2).min_element();
        (far >= near.max(0.0)).then_some(near.max(0.0))
    }
}

/// Moves `body` by `delta` one axis at a time, pushing it out of `solids`.
/// Returns (new body, hit per axis, grounded).
pub fn move_and_collide(mut body: Aabb, delta: Vec3, solids: &[Aabb]) -> (Aabb, BVec3, bool) {
    let mut hit = BVec3::FALSE;
    let mut grounded = false;
    for axis in [0, 2, 1] {
        body.c[axis] += delta[axis];
        for s in solids {
            if body.overlaps(s) {
                let edge = s.h[axis] + body.h[axis];
                body.c[axis] = if delta[axis] > 0.0 { s.c[axis] - edge } else { s.c[axis] + edge };
                hit.set(axis, true);
                grounded |= axis == 1 && delta.y <= 0.0;
            }
        }
    }
    (body, hit, grounded)
}

#[derive(Resource, Default)]
pub struct Solids(pub Vec<Aabb>);

#[derive(Component)]
pub struct Hazard(pub Aabb);
#[derive(Component)]
pub struct Goal(pub Aabb);
#[derive(Component)]
pub struct Coin;
/// Index decides which quality tiers show it (see `apply_prefs`).
#[derive(Component)]
pub struct Decor(pub usize);
#[derive(Component)]
pub struct Walker {
    pub a: Vec3,
    pub b: Vec3,
    pub t: f32,
    pub alive: bool,
}
/// Fixed-step position; rendered Transform is interpolated between prev and pos.
#[derive(Component, Clone, Copy)]
pub struct Sim {
    pub prev: Vec3,
    pub pos: Vec3,
}

pub const SPAWN: Vec3 = Vec3::new(0.0, 1.0, 3.0);
pub const WALKER_HALF: Vec3 = Vec3::splat(0.5);

const GROUND: &[([f32; 3], [f32; 3])] = &[
    ([0.0, -1.0, 0.0], [14.0, 2.0, 14.0]),
    ([0.0, 0.0, -11.0], [5.0, 2.0, 5.0]),
    ([4.0, 1.0, -17.0], [4.0, 2.0, 4.0]),
    ([-1.0, 2.0, -22.0], [4.0, 2.0, 4.0]),
    ([0.0, -1.0, -36.0], [16.0, 2.0, 16.0]),
    ([0.0, 1.25, -48.0], [6.0, 4.5, 4.0]), // needs the double jump
    ([0.0, 4.0, -55.0], [3.0, 1.0, 3.0]),
    ([5.0, 5.0, -60.0], [3.0, 1.0, 3.0]),
    ([0.0, 6.0, -66.0], [3.0, 1.0, 3.0]),
    ([0.0, 6.0, -75.0], [8.0, 1.0, 8.0]),
];
const SPIKES: &[([f32; 3], [f32; 3])] = &[([0.0, 0.25, -32.0], [6.0, 0.5, 1.5])];
const COINS: &[[f32; 3]] = &[
    [0.0, 1.0, -3.0], [0.0, 2.0, -11.0], [4.0, 3.0, -17.0], [-1.0, 4.0, -22.0],
    [0.0, 3.0, -26.0], [-5.0, 1.0, -36.0], [5.0, 1.0, -36.0], [0.0, 1.0, -42.0],
    [0.0, 4.5, -48.0], [0.0, 5.5, -55.0], [5.0, 6.5, -60.0], [0.0, 7.5, -66.0],
];
const TREES: &[[f32; 2]] = &[
    [-5.0, 5.0], [5.0, 5.0], [-5.5, -4.0], [5.5, -4.0], [-6.0, 0.0], [6.0, 1.0],
    [-6.5, -30.0], [6.5, -30.0], [-6.5, -42.0], [6.5, -42.0], [-3.0, -43.0], [3.0, -29.0],
];

pub fn spawn_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mut mat = |c: Color| mats.add(StandardMaterial { base_color: c, perceptual_roughness: 0.9, ..default() });
    let grass = mat(Color::srgb(0.35, 0.75, 0.3));
    let spike = mat(Color::srgb(0.6, 0.15, 0.2));
    let gold = mat(Color::srgb(1.0, 0.8, 0.1));
    let trunk = mat(Color::srgb(0.45, 0.3, 0.15));
    let leaves = mat(Color::srgb(0.15, 0.55, 0.25));
    let walker = mat(Color::srgb(0.5, 0.2, 0.6));

    let mut solids = Vec::new();
    for &(c, s) in GROUND {
        let b = Aabb::new(c.into(), s.into());
        solids.push(b);
        commands.spawn((Mesh3d(meshes.add(Cuboid::from_size(b.h * 2.0))), MeshMaterial3d(grass.clone()), Transform::from_translation(b.c)));
    }
    commands.insert_resource(Solids(solids));

    for &(c, s) in SPIKES {
        let b = Aabb::new(c.into(), s.into());
        commands.spawn((Hazard(b), Mesh3d(meshes.add(Cuboid::from_size(b.h * 2.0))), MeshMaterial3d(spike.clone()), Transform::from_translation(b.c)));
    }

    let coin = meshes.add(Cylinder::new(0.3, 0.08));
    for &p in COINS {
        commands.spawn((Coin, Mesh3d(coin.clone()), MeshMaterial3d(gold.clone()), Transform::from_translation(p.into()).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
    }

    let goal = Aabb::new(Vec3::new(0.0, 7.5, -77.0), Vec3::splat(1.0));
    commands.spawn((Goal(goal), Mesh3d(meshes.add(Cuboid::from_length(1.0))), MeshMaterial3d(gold.clone()), Transform::from_translation(goal.c)));

    let trunk_mesh = meshes.add(Cuboid::new(0.4, 2.0, 0.4));
    let leaf_mesh = meshes.add(Cuboid::from_length(1.6));
    for (i, &[x, z]) in TREES.iter().enumerate() {
        commands.spawn((Decor(i), Transform::from_xyz(x, 1.0, z), Visibility::default(), children![
            (Mesh3d(trunk_mesh.clone()), MeshMaterial3d(trunk.clone())),
            (Mesh3d(leaf_mesh.clone()), MeshMaterial3d(leaves.clone()), Transform::from_xyz(0.0, 1.5, 0.0)),
        ]));
    }

    let (a, b) = (Vec3::new(-6.0, 0.5, -39.0), Vec3::new(6.0, 0.5, -39.0));
    commands.spawn((
        Walker { a, b, t: 0.0, alive: true },
        Sim { prev: a, pos: a },
        Mesh3d(meshes.add(Cuboid::from_size(WALKER_HALF * 2.0))),
        MeshMaterial3d(walker),
        Transform::from_translation(a),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor() -> Aabb {
        Aabb::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(10.0, 2.0, 10.0))
    }

    #[test]
    fn lands_on_floor() {
        let body = Aabb { c: Vec3::new(0.0, 0.55, 0.0), h: Vec3::splat(0.5) };
        let (b, hit, grounded) = move_and_collide(body, Vec3::new(0.0, -0.2, 0.0), &[floor()]);
        assert!(grounded && hit.y);
        assert!((b.c.y - 0.5).abs() < 1e-5);
    }

    #[test]
    fn stopped_by_wall_but_slides() {
        let wall = Aabb::new(Vec3::new(2.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 10.0));
        let body = Aabb { c: Vec3::new(0.9, 0.5, 0.0), h: Vec3::splat(0.5) };
        let (b, hit, _) = move_and_collide(body, Vec3::new(0.3, 0.0, 0.3), &[floor(), wall]);
        assert!(hit.x && !hit.z);
        assert!((b.c.x - 1.0).abs() < 1e-5 && (b.c.z - 0.3).abs() < 1e-5);
    }

    #[test]
    fn ray_hits_box() {
        assert_eq!(floor().ray(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y), Some(5.0));
        assert_eq!(floor().ray(Vec3::new(0.0, 5.0, 0.0), Vec3::Y), None);
    }
}
