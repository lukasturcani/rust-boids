use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::MaterialMesh2dBundle;
use rand::distributions::Standard;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const BOID_TIMESTEP: f32 = 1.0 / 60.0;
const NUM_BOIDS: usize = 200;
const BOX_SIZE: f32 = 100.0;
const MAX_BOID_VELOCITY: f32 = 10.0;
const SEPARATION_RADIUS: f32 = 5.0;
const SEPARATION_COEFFICIENT: f32 = 1.0;
const VISIBLE_RADIUS: f32 = SEPARATION_RADIUS + 5.0;
const ALIGNMENT_COEFFICIENT: f32 = 1.0;

struct Boids;

impl Plugin for Boids {
    fn build(&self, app: &mut App) {
        app.insert_resource(FixedTime::new_from_secs(BOID_TIMESTEP));
        app.insert_resource(Generator(StdRng::seed_from_u64(55)));
        app.add_startup_system(setup);
        app.add_startup_system(spawn_boids);
        app.add_system(clear_separation.in_schedule(CoreSchedule::FixedUpdate));
        app.add_system(clear_alignment.in_schedule(CoreSchedule::FixedUpdate));
        app.add_system(
            calculate_separation
                .after(clear_separation)
                .in_schedule(CoreSchedule::FixedUpdate),
        );
        app.add_system(
            calculate_alignment
                .after(clear_alignment)
                .in_schedule(CoreSchedule::FixedUpdate),
        );
        app.add_system(
            update_boid_velocity
                .after(calculate_separation)
                .after(calculate_alignment)
                .in_schedule(CoreSchedule::FixedUpdate),
        );
        app.add_system(
            move_boids
                .after(update_boid_velocity)
                .in_schedule(CoreSchedule::FixedUpdate),
        );
    }
}

#[derive(Resource)]
struct Generator(pub StdRng);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 0.11,
            ..default()
        },
        ..default()
    });
}

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Separation {
    displacment_sum: Vec2,
}

impl Separation {
    fn new() -> Self {
        Self {
            displacment_sum: Vec2::new(0.0, 0.),
        }
    }
}

#[derive(Component)]
struct Alignment {
    velocity_sum: Vec2,
    num_neighbors: u16,
}

impl Alignment {
    fn new() -> Self {
        Self {
            velocity_sum: Vec2::new(0., 0.),
            num_neighbors: 0,
        }
    }
}

fn spawn_boids(
    mut generator: ResMut<Generator>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let boids: Vec<_> = (&mut generator.0)
        .sample_iter(Standard)
        .map(|[x, y, vx, vy]: [f32; 4]| {
            let velocity = (Vec2::new(vx, vy) - 0.5) * MAX_BOID_VELOCITY;
            (
                MaterialMesh2dBundle {
                    transform: Transform {
                        translation: Vec3::new((x - 0.5) * BOX_SIZE, (y - 0.5) * BOX_SIZE, 0.0),
                        rotation: Quat::from_rotation_z(
                            Vec2::new(0.0, 1.0).angle_between(velocity),
                        ),
                        ..default()
                    },
                    mesh: meshes.add(create_boid_mesh()).into(),
                    material: materials.add(ColorMaterial::from(Color::PURPLE)),
                    ..default()
                },
                Velocity(velocity),
                Separation::new(),
                Alignment::new(),
            )
        })
        .take(NUM_BOIDS)
        .collect();

    commands.spawn_batch(boids);
}

fn move_boids(mut boids: Query<(&mut Transform, &Velocity)>, time_step: Res<FixedTime>) {
    for (mut transform, velocity) in boids.iter_mut() {
        transform.translation.x += velocity.0.x * time_step.period.as_secs_f32();
        transform.translation.y += velocity.0.y * time_step.period.as_secs_f32();
        transform.rotation = Quat::from_rotation_z(Vec2::new(0.0, 1.0).angle_between(velocity.0))
    }
}

fn clear_separation(mut boids: Query<&mut Separation>) {
    for mut separation in boids.iter_mut() {
        separation.displacment_sum.x = 0.;
        separation.displacment_sum.y = 0.;
    }
}

fn clear_alignment(mut boids: Query<&mut Alignment>) {
    for mut alignment in boids.iter_mut() {
        alignment.velocity_sum.x = 0.;
        alignment.velocity_sum.y = 0.;
        alignment.num_neighbors = 0;
    }
}

fn calculate_separation(
    mut boids: Query<(&Transform, &mut Separation)>,
    _time_step: Res<FixedTime>,
) {
    let mut combinations = boids.iter_combinations_mut();
    while let Some([(transform1, mut separation1), (transform2, mut separation2)]) =
        combinations.fetch_next()
    {
        let displacement = transform1.translation - transform2.translation;
        if displacement.length() < SEPARATION_RADIUS {
            separation1.displacment_sum.x += displacement.x;
            separation1.displacment_sum.y += displacement.y;
            separation2.displacment_sum.x -= displacement.x;
            separation2.displacment_sum.y -= displacement.y;
        }
    }
}

fn calculate_alignment(
    mut boids: Query<(&Transform, &Velocity, &mut Alignment)>,
    _time_step: Res<FixedTime>,
) {
    let mut combinations = boids.iter_combinations_mut();
    while let Some(
        [(transform1, velocity1, mut alignment1), (transform2, velocity2, mut alignment2)],
    ) = combinations.fetch_next()
    {
        let displacement = transform1.translation - transform2.translation;
        if displacement.length() > SEPARATION_RADIUS && displacement.length() < VISIBLE_RADIUS {
            alignment1.velocity_sum += velocity2.0;
            alignment2.velocity_sum += velocity1.0;
            alignment1.num_neighbors += 1;
            alignment2.num_neighbors += 1;
        }
    }
}

fn update_boid_velocity(mut boids: Query<(&Separation, &Alignment, &mut Velocity)>) {
    for (separation, alignment, mut velocity) in boids.iter_mut() {
        if alignment.num_neighbors > 0 {
            let velocity_update =
                (alignment.velocity_sum / (alignment.num_neighbors as f32)) - velocity.0;
            velocity.0 += velocity_update * ALIGNMENT_COEFFICIENT;
        }
        velocity.0 += separation.displacment_sum * SEPARATION_COEFFICIENT;
    }
}

fn create_boid_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.5, 2.5, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
    );
    mesh.set_indices(Some(Indices::U32(vec![0, 1, 2])));
    mesh
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(Boids)
        .run();
}
