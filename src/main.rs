use bevy::input::mouse::MouseWheel;
use bevy::prelude::shape::{Circle, Quad};
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::view::Visibility;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rand::distributions::Standard;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// const GRAY1: Color = Color::rgb(153. / 255., 153. / 255., 153. / 255.);
// const GRAY2: Color = Color::rgb(119. / 255., 119. / 255., 119. / 255.);
const GRAY3: Color = Color::rgb(85.0 / 255., 85. / 255., 85. / 255.);
const GRAY4: Color = Color::rgb(51. / 255., 51. / 255., 51. / 255.);
const GRAY5: Color = Color::rgb(17. / 255., 17. / 255., 17. / 255.);

const BOID_TIMESTEP: f32 = 1.0 / 60.0;
const NUM_BOIDS: usize = 200;

struct Boids;

impl Plugin for Boids {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.insert_resource(FixedTime::new_from_secs(BOID_TIMESTEP));
        app.insert_resource(Generator(StdRng::seed_from_u64(55)));
        app.insert_resource(ClearColor(GRAY5));
        app.insert_resource(BoxSize(250.0));
        app.insert_resource(MaxBoidSpeed(60.0));
        app.insert_resource(MinBoidSpeed(15.0));
        app.insert_resource(MouseFollowRadius(60.0));
        app.insert_resource(SeparationRadius(3.0));
        app.insert_resource(SeparationCoefficient(3.0));
        app.insert_resource(SeparationCoefficient(0.1));
        app.insert_resource(VisibleRadius(6.0));
        app.insert_resource(AlignmentCoefficient(0.005));
        app.insert_resource(CohesionCoefficient(0.0005));
        app.insert_resource(BoxBoundCoefficient(1.0));
        app.insert_resource(MouseFollowCoefficient(1.0));
        app.add_startup_system(setup);
        app.add_startup_system(spawn_boids);
        app.add_system(ui);
        app.add_system(mouse_outline);
        app.add_system(box_outline);
        app.add_system(camera_scale);
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 0.5,
            ..default()
        },
        ..default()
    });
    commands.spawn((
        MaterialMesh2dBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.1),
                ..default()
            },
            mesh: meshes.add(Circle::new(1.0).into()).into(),
            material: materials.add(ColorMaterial::from(GRAY3)),
            visibility: Visibility::Hidden,
            ..default()
        },
        MouseOutline,
    ));
    commands.spawn((
        MaterialMesh2dBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                ..default()
            },
            mesh: meshes.add(Quad::new(Vec2::new(1.0, 1.0)).into()).into(),
            material: materials.add(ColorMaterial::from(GRAY4)),
            ..default()
        },
        BoxOutline,
    ));
}

#[derive(Resource)]
struct BoxSize(f32);

#[derive(Resource)]
struct MaxBoidSpeed(f32);

#[derive(Resource)]
struct MinBoidSpeed(f32);

#[derive(Resource)]
struct MouseFollowRadius(f32);

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

#[derive(Resource)]
struct MouseFollowCoefficient(f32);

#[derive(Component)]
struct MouseOutline;

#[derive(Component)]
struct BoxOutline;

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

#[allow(clippy::too_many_arguments)]
fn spawn_boids(
    mut generator: ResMut<Generator>,
    box_size: Res<BoxSize>,
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
                        translation: Vec3::new((x - 0.5) * box_size.0, (y - 0.5) * box_size.0, 1.0),
                        rotation: Quat::from_rotation_z(
                            Vec2::new(0.0, 1.0).angle_between(velocity),
                        ),
                        ..default()
                    },
                    mesh: meshes.add(create_boid_mesh()).into(),
                    material: materials.add(ColorMaterial::from(Color::WHITE)),
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

#[allow(clippy::too_many_arguments)]
fn update_boid_velocity(
    box_size: Res<BoxSize>,
    min_speed: Res<MinBoidSpeed>,
    max_speed: Res<MaxBoidSpeed>,
    mouse_follow_radius: Res<MouseFollowRadius>,
    separation_coefficient: Res<SeparationCoefficient>,
    alignment_coefficient: Res<AlignmentCoefficient>,
    cohesion_coefficient: Res<CohesionCoefficient>,
    box_bound_coefficient: Res<BoxBoundCoefficient>,
    mouse_follow_coefficient: Res<MouseFollowCoefficient>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut boids: Query<(
        &Transform,
        &Separation,
        &Alignment,
        &Cohesion,
        &mut Velocity,
    )>,
) {
    let left_margin = 0. - box_size.0 * 0.5;
    let right_margin = box_size.0 * 0.5;
    let top_margin = right_margin;
    let bottom_margin = left_margin;
    let (camera, camera_transform) = camera_query.single();
    let cursor = window
        .single()
        .cursor_position()
        .and_then(|position| camera.viewport_to_world_2d(camera_transform, position));
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
        if let Some(cursor_position) = cursor {
            let translation = Vec2::new(transform.translation.x, transform.translation.y);
            if translation.distance(cursor_position) < mouse_follow_radius.0 {
                velocity.0 +=
                    (cursor_position - translation).normalize() * mouse_follow_coefficient.0
            }
        }
        if transform.translation.x < left_margin {
            velocity.0.x += box_bound_coefficient.0;
        }
        if transform.translation.x > right_margin {
            velocity.0.x -= box_bound_coefficient.0;
        }
        if transform.translation.y < bottom_margin {
            velocity.0.y += box_bound_coefficient.0;
        }
        if transform.translation.y > top_margin {
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

fn camera_scale(
    mut scroll_events: EventReader<MouseWheel>,
    mut camera_projection: Query<&mut OrthographicProjection>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    for event in scroll_events.iter() {
        match event.unit {
            MouseScrollUnit::Line => {
                let scale = camera_projection.single_mut().scale;
                camera_projection.single_mut().scale = (scale + event.y * 0.25).clamp(0.1, 1.0);
            }
            MouseScrollUnit::Pixel => {
                let scale = camera_projection.single_mut().scale;
                camera_projection.single_mut().scale = (scale + event.y * 0.005).clamp(0.1, 1.0);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn ui(
    mut contexts: EguiContexts,
    mut generator: ResMut<Generator>,
    mut box_size: ResMut<BoxSize>,
    mut max_speed: ResMut<MaxBoidSpeed>,
    mut min_speed: ResMut<MinBoidSpeed>,
    mut separation_radius: ResMut<SeparationRadius>,
    mut visible_radius: ResMut<VisibleRadius>,
    mut mouse_follow_radius: ResMut<MouseFollowRadius>,
    mut separation_coefficient: ResMut<SeparationCoefficient>,
    mut alignment_coefficient: ResMut<AlignmentCoefficient>,
    mut cohesion_coefficient: ResMut<CohesionCoefficient>,
    mut box_bound_coefficient: ResMut<BoxBoundCoefficient>,
    mut mouse_follow_coefficient: ResMut<MouseFollowCoefficient>,
    mut boids: Query<(&mut Transform, &mut Velocity)>,
) {
    egui::Window::new("Parameters").show(contexts.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut min_speed.0, 0.0..=100.0).text("Min Boid Speed"));
        ui.add(egui::Slider::new(&mut max_speed.0, 0.0..=100.0).text("Max Boid Speed"));
        ui.add(egui::Slider::new(&mut separation_radius.0, 0.0..=100.0).text("Separation Radius"));
        ui.add(egui::Slider::new(&mut visible_radius.0, 0.0..=100.0).text("Visible Radius"));
        ui.add(
            egui::Slider::new(&mut separation_coefficient.0, 0.0..=1.0)
                .text("Separation Coefficient"),
        );
        ui.add(
            egui::Slider::new(&mut alignment_coefficient.0, 0.0..=1.0)
                .text("Alignment Coefficient"),
        );
        ui.add(
            egui::Slider::new(&mut cohesion_coefficient.0, 0.0..=1.0).text("Cohesion Coefficient"),
        );
        ui.add(egui::Slider::new(&mut box_size.0, 0.0..=600.0).text("Box Size"));
        ui.add(
            egui::Slider::new(&mut box_bound_coefficient.0, 0.0..=1.0)
                .text("Box Bound Coefficient"),
        );
        ui.add(
            egui::Slider::new(&mut mouse_follow_radius.0, 0.0..=100.0).text("Mouse Follow Radius"),
        );
        ui.add(
            egui::Slider::new(&mut mouse_follow_coefficient.0, -1.0..=1.0)
                .text("Mouse Follow Coefficient"),
        );
        if ui.button("Restart Simulation").clicked() {
            for (mut transform, mut velocity) in boids.iter_mut() {
                let [x, y, vx, vy, vmag]: [f32; 5] = generator.0.sample(Standard);
                let velocity_direction = (Vec2::new(vx, vy) - 0.5).normalize();
                velocity.0 = velocity_direction * min_speed.0
                    + velocity_direction * vmag * (max_speed.0 - min_speed.0);

                transform.translation =
                    Vec3::new((x - 0.5) * box_size.0, (y - 0.5) * box_size.0, 1.0);
                transform.rotation =
                    Quat::from_rotation_z(Vec2::new(0.0, 1.0).angle_between(velocity.0));
            }
        }
    });
}

fn mouse_outline(
    mut mouse_outline_query: Query<(&mut Transform, &mut Visibility), With<MouseOutline>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    outline_size: Res<MouseFollowRadius>,
) {
    let (mut mouse_outline_transform, mut mouse_outline_visiblity) =
        mouse_outline_query.single_mut();

    let (camera, camera_transform) = camera_query.single();
    let cursor = window
        .single()
        .cursor_position()
        .and_then(|position| camera.viewport_to_world_2d(camera_transform, position));
    if let Some(cursor_position) = cursor {
        mouse_outline_transform.translation.x = cursor_position.x;
        mouse_outline_transform.translation.y = cursor_position.y;
        *mouse_outline_visiblity = Visibility::Visible;
        mouse_outline_transform.scale = Vec3::new(outline_size.0, outline_size.0, outline_size.0);
    } else {
        *mouse_outline_visiblity = Visibility::Hidden;
    }
}

fn box_outline(box_size: Res<BoxSize>, mut r#box: Query<&mut Transform, With<BoxOutline>>) {
    let mut transform = r#box.single_mut();
    transform.scale = Vec3::new(box_size.0, box_size.0, box_size.0);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some("#bevy-canvas".to_string()),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(Boids)
        .run();
}
