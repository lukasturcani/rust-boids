use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::MaterialMesh2dBundle;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rand::distributions::Standard;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const BOID_TIMESTEP: f32 = 1.0 / 60.0;
const NUM_BOIDS: usize = 200;
const BOX_SIZE: f32 = 100.0;
const LEFT_MARGIN: f32 = 0. - BOX_SIZE * 0.5;
const RIGHT_MARGIN: f32 = BOX_SIZE * 0.5;
const TOP_MARGIN: f32 = RIGHT_MARGIN;
const BOTTOM_MARGIN: f32 = LEFT_MARGIN;

struct Boids;

impl Plugin for Boids {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.insert_resource(FixedTime::new_from_secs(BOID_TIMESTEP));
        app.insert_resource(Generator(StdRng::seed_from_u64(55)));
        app.insert_resource(MaxBoidSpeed(60.0));
        app.insert_resource(MinBoidSpeed(15.0));
        app.insert_resource(SeparationRadius(3.0));
        app.insert_resource(SeparationCoefficient(3.0));
        app.insert_resource(SeparationCoefficient(0.1));
        app.insert_resource(VisibleRadius(6.0));
        app.insert_resource(AlignmentCoefficient(0.005));
        app.insert_resource(CohesionCoefficient(0.0005));
        app.insert_resource(BoxBoundCoefficient(0.2));
        app.insert_resource(CameraScale(0.11));
        app.add_startup_system(setup);
        app.add_startup_system(spawn_boids);
        app.add_system(ui);
        app.add_system(clear_separation.in_schedule(CoreSchedule::FixedUpdate));
        app.add_system(clear_alignment.in_schedule(CoreSchedule::FixedUpdate));
        app.add_system(clear_cohesion.in_schedule(CoreSchedule::FixedUpdate));
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
            calculate_cohesion
                .after(clear_cohesion)
                .in_schedule(CoreSchedule::FixedUpdate),
        );
        app.add_system(
            update_boid_velocity
                .after(calculate_separation)
                .after(calculate_alignment)
                .after(calculate_cohesion)
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

fn setup(mut commands: Commands, scale: Res<CameraScale>) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: scale.0,
            ..default()
        },
        ..default()
    });
}

#[derive(Resource)]
struct CameraScale(f32);

#[derive(Resource)]
struct MaxBoidSpeed(f32);

#[derive(Resource)]
struct MinBoidSpeed(f32);

#[derive(Resource)]
struct SeparationRadius(f32);

#[derive(Resource)]
struct SeparationCoefficient(f32);

#[derive(Resource)]
struct VisibleRadius(f32);

#[derive(Resource)]
struct AlignmentCoefficient(f32);

#[derive(Resource)]
struct CohesionCoefficient(f32);

#[derive(Resource)]
struct BoxBoundCoefficient(f32);

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

#[derive(Component)]
struct Cohesion {
    translation_sum: Vec2,
    num_neighbors: u16,
}

impl Cohesion {
    fn new() -> Self {
        Self {
            translation_sum: Vec2::new(0., 0.),
            num_neighbors: 0,
        }
    }
}

fn spawn_boids(
    mut generator: ResMut<Generator>,
    max_speed: Res<MaxBoidSpeed>,
    min_speed: Res<MinBoidSpeed>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let boids: Vec<_> = (&mut generator.0)
        .sample_iter(Standard)
        .map(|[x, y, vx, vy, vmag]: [f32; 5]| {
            let velocity_direction = (Vec2::new(vx, vy) - 0.5).normalize();
            let velocity = velocity_direction * min_speed.0
                + velocity_direction * vmag * (max_speed.0 - min_speed.0);
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
                Cohesion::new(),
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

fn clear_cohesion(mut boids: Query<&mut Cohesion>) {
    for mut alignment in boids.iter_mut() {
        alignment.translation_sum.x = 0.;
        alignment.translation_sum.y = 0.;
        alignment.num_neighbors = 0;
    }
}

fn calculate_separation(
    separation_radius: Res<SeparationRadius>,
    mut boids: Query<(&Transform, &mut Separation)>,
) {
    let mut combinations = boids.iter_combinations_mut();
    while let Some([(transform1, mut separation1), (transform2, mut separation2)]) =
        combinations.fetch_next()
    {
        let displacement = transform1.translation - transform2.translation;
        if displacement.length() < separation_radius.0 {
            separation1.displacment_sum.x += displacement.x;
            separation1.displacment_sum.y += displacement.y;
            separation2.displacment_sum.x -= displacement.x;
            separation2.displacment_sum.y -= displacement.y;
        }
    }
}

fn calculate_alignment(
    separation_radius: Res<SeparationRadius>,
    visible_radius: Res<VisibleRadius>,
    mut boids: Query<(&Transform, &Velocity, &mut Alignment)>,
) {
    let mut combinations = boids.iter_combinations_mut();
    while let Some(
        [(transform1, velocity1, mut alignment1), (transform2, velocity2, mut alignment2)],
    ) = combinations.fetch_next()
    {
        let displacement = transform1.translation - transform2.translation;
        if displacement.length() > separation_radius.0 && displacement.length() < visible_radius.0 {
            alignment1.velocity_sum += velocity2.0;
            alignment2.velocity_sum += velocity1.0;
            alignment1.num_neighbors += 1;
            alignment2.num_neighbors += 1;
        }
    }
}

fn calculate_cohesion(
    separation_radius: Res<SeparationRadius>,
    visible_radius: Res<VisibleRadius>,
    mut boids: Query<(&Transform, &mut Cohesion)>,
) {
    let mut combinations = boids.iter_combinations_mut();
    while let Some([(transform1, mut cohesion1), (transform2, mut cohesion2)]) =
        combinations.fetch_next()
    {
        let displacement = transform1.translation - transform2.translation;
        if displacement.length() > separation_radius.0 && displacement.length() < visible_radius.0 {
            cohesion1.translation_sum.x += transform2.translation.x;
            cohesion1.translation_sum.y += transform2.translation.y;
            cohesion2.translation_sum.x += transform1.translation.x;
            cohesion2.translation_sum.y += transform1.translation.y;
            cohesion1.num_neighbors += 1;
            cohesion2.num_neighbors += 1;
        }
    }
}

fn update_boid_velocity(
    min_speed: Res<MinBoidSpeed>,
    max_speed: Res<MaxBoidSpeed>,
    separation_coefficient: Res<SeparationCoefficient>,
    alignment_coefficient: Res<AlignmentCoefficient>,
    cohesion_coefficient: Res<CohesionCoefficient>,
    box_bound_coefficient: Res<BoxBoundCoefficient>,
    mut boids: Query<(
        &Transform,
        &Separation,
        &Alignment,
        &Cohesion,
        &mut Velocity,
    )>,
) {
    for (transform, separation, alignment, cohesion, mut velocity) in boids.iter_mut() {
        if alignment.num_neighbors > 0 {
            let velocity_update =
                (alignment.velocity_sum / (alignment.num_neighbors as f32)) - velocity.0;
            velocity.0 += velocity_update * alignment_coefficient.0;
        }
        if cohesion.num_neighbors > 0 {
            let mut velocity_update = cohesion.translation_sum / (cohesion.num_neighbors as f32);
            velocity_update.x -= transform.translation.x;
            velocity_update.y -= transform.translation.y;
            velocity.0 += velocity_update * cohesion_coefficient.0;
        }
        if transform.translation.x < LEFT_MARGIN {
            velocity.0.x += box_bound_coefficient.0;
        }
        if transform.translation.x > RIGHT_MARGIN {
            velocity.0.x -= box_bound_coefficient.0;
        }
        if transform.translation.y < BOTTOM_MARGIN {
            velocity.0.y += box_bound_coefficient.0;
        }
        if transform.translation.y > TOP_MARGIN {
            velocity.0.y -= box_bound_coefficient.0;
        }
        velocity.0 += separation.displacment_sum * separation_coefficient.0;
        velocity.0 = velocity.0.clamp_length(min_speed.0, max_speed.0);
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

fn ui(mut contexts: EguiContexts) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");
    });
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(Boids)
        .run();
}
