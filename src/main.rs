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

struct Boids;

impl Plugin for Boids {
    fn build(&self, app: &mut App) {
        app.insert_resource(FixedTime::new_from_secs(BOID_TIMESTEP));
        app.insert_resource(Generator(StdRng::seed_from_u64(55)));
        app.add_startup_system(setup);
        app.add_startup_system(spawn_boids);
        app.add_system(move_boids.in_schedule(CoreSchedule::FixedUpdate));
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
