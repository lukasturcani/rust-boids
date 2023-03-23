use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use rand::distributions::Standard;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

struct Boids;

impl Plugin for Boids {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_startup_system(spawn_boids);
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

const NUM_BOIDS: usize = 200;
const BOX_SIZE: f32 = 100.0;

fn spawn_boids(
    mut generator: ResMut<Generator>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let boid_translations: Vec<_> = (&mut generator.0)
        .sample_iter(Standard)
        .map(|(x, y, angle): (f32, f32, f32)| Transform {
            translation: Vec3::new(
                x * BOX_SIZE - 0.5 * BOX_SIZE,
                y * BOX_SIZE - 0.5 * BOX_SIZE,
                0.0,
            ),
            rotation: Quat::from_rotation_z(angle),
            ..default()
        })
        .map(|transform| MaterialMesh2dBundle {
            transform,
            mesh: meshes
                .add(Mesh::from(shape::RegularPolygon::new(1.0, 3)))
                .into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            ..default()
        })
        .take(NUM_BOIDS)
        .collect();

    commands.spawn_batch(boid_translations);
}

fn main() {
    App::new()
        .insert_resource(Generator(StdRng::seed_from_u64(55)))
        .add_plugins(DefaultPlugins)
        .add_plugin(Boids)
        .run();
}
