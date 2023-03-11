use std::ops::Sub;

use bevy::prelude::*;
use rand::distributions::Standard;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

struct Boids;

impl Plugin for Boids {
    fn build(&self, app: &mut App) {
        app.add_startup_system(add_boids)
            .add_system(update_velocities)
            .add_system(update_positions_with_periodic_boundaries);
    }
}

#[derive(Resource)]
struct Generator(pub StdRng);

struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Sub for Vector2 {
    type Output = Vector2;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub for &Vector2 {
    type Output = Vector2;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Component)]
struct Position(pub Vector2);

#[derive(Component)]
struct Velocity(pub Vector2);

#[derive(Resource)]
struct WorldBounds {
    pub max_x_position: f32,
    pub max_y_position: f32,
}

#[derive(Resource)]
struct MaxBoidVelocity(pub f32);

impl WorldBounds {
    fn apply_periodic_boundary_condition(&self, position: &mut Position) {
        if position.0.x > self.max_x_position {
            position.0.x = position.0.x % self.max_x_position
        }
        if position.0.y > self.max_y_position {
            position.0.y = position.0.y % self.max_y_position
        }
    }
}

fn boid(
    (px, py, vx, vy): (f32, f32, f32, f32),
    max_velocity: &MaxBoidVelocity,
    bounds: &WorldBounds,
) -> (Position, Velocity) {
    (
        Position(Vector2 {
            x: px * bounds.max_x_position,
            y: py * bounds.max_y_position,
        }),
        Velocity(Vector2 {
            x: vx * max_velocity.0,
            y: vy * max_velocity.0,
        }),
    )
}

fn update_velocities(mut _query: Query<(Entity, &Position, &mut Velocity)>) {}

fn update_positions_with_periodic_boundaries(
    bounds: Res<WorldBounds>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    query.iter_mut().for_each(|(mut position, velocity)| {
        position.0.x += velocity.0.x;
        position.0.y += velocity.0.y;
        bounds.apply_periodic_boundary_condition(&mut position);
    });
}

fn add_boids(
    bounds: Res<WorldBounds>,
    max_velocity: Res<MaxBoidVelocity>,
    mut generator: ResMut<Generator>,
    mut commands: Commands,
) {
    let num_boids = 200_usize;
    let boids: Vec<(Position, Velocity)> = (&mut generator.0)
        .sample_iter(Standard)
        .take(num_boids)
        .map(|data| boid(data, &max_velocity, &bounds))
        .collect();
    commands.spawn_batch(boids);
}

fn main() {
    App::new()
        .insert_resource(WorldBounds {
            max_x_position: 100.,
            max_y_position: 100.,
        })
        .insert_resource(MaxBoidVelocity(1.))
        .insert_resource(Generator(StdRng::seed_from_u64(55)))
        .add_plugins(DefaultPlugins)
        .add_plugin(Boids)
        .run();
}
