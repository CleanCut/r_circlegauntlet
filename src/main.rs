use legion::prelude::*;
use rand::prelude::*;
use rusty_engine::gfx::event::{ButtonProcessor, GameEvent};
use rusty_engine::gfx::{color::Color, Sprite, Window};
use rusty_engine::glm::{cross, distance, distance2, reflect_vec, Vec2};

const GOAL_RADIUS: f32 = 1. / 8.;
const OBSTACLE_RADIUS: f32 = 1. / 12.;
const PLAYER_RADIUS: f32 = 1. / 16.;

type Position = Vec2;
struct Velocity(Vec2);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Goal;
#[derive(Clone, Copy, Debug, PartialEq)]
struct Obstacle;
#[derive(Clone, Copy, Debug, PartialEq)]
struct Player;
#[derive(Clone, Copy, Debug, PartialEq)]
struct SpriteIndex(usize);

fn main() {
    let universe = Universe::new();
    let mut world = universe.create_world();
    let mut window = Window::new(None, "Circle Gauntlet");

    // (Sprites aren't Send)
    let mut sprites = vec![
        // Goal circle (large-ish, green)
        Sprite::smooth_circle(
            &window,
            Position::new(0., 0.), // Ignored
            0.,
            1.,
            GOAL_RADIUS,
            Color::new(0., 1., 0.),
        ),
        // Player circle (small-ish, blue)
        Sprite::smooth_circle(
            &window,
            Position::new(0., 0.), // Ignored
            0.,
            1.,
            PLAYER_RADIUS,
            Color::new(0., 0., 1.),
        ),
        // Obstacle circles -- reusing the same sprite for all instances is probably a terrible idea, but I want to try it
        Sprite::smooth_circle(
            &window,
            Position::new(0., 0.), // Ignored
            0.,
            1.,
            OBSTACLE_RADIUS,
            Color::new(1., 0., 0.),
        ),
    ];

    let goal_start_pos = Position::new(0.75, -0.75);
    let player_start_pos = Position::new(-0.75, 0.75);
    world.insert((Goal,), vec![(goal_start_pos, SpriteIndex(0))]);
    world.insert(
        (Player,),
        vec![(
            player_start_pos,
            Velocity(Vec2::new(0.0, 0.0)),
            SpriteIndex(1),
        )],
    );
    let mut rng = rand::thread_rng();
    let mut prev_positions = vec![];
    let obstacle_spacing = 0.1;
    for _ in 0..16 {
        let mut pos = player_start_pos;
        while distance2(&pos, &player_start_pos) < obstacle_spacing
            || distance2(&pos, &goal_start_pos) < obstacle_spacing
            || prev_positions
                .iter()
                .map(|ref x| distance2(&pos, &x))
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(500.)
                .partial_cmp(&obstacle_spacing)
                .unwrap_or(std::cmp::Ordering::Equal)
                == std::cmp::Ordering::Less
        {
            pos = Position::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0);
        }
        prev_positions.push(pos);
        world.insert((Obstacle,), vec![(pos, SpriteIndex(2))]);
    }

    let mut button_processor = ButtonProcessor::new();
    'gameloop: loop {
        let mut dead = false;

        for event in window.poll_game_events() {
            match event {
                GameEvent::Quit => break 'gameloop,
                GameEvent::Button {
                    button_value,
                    button_state,
                } => button_processor.process(button_value, button_state),
                _ => {}
            }
        }
        window.drawstart();

        // Get the player's position
        let mut player_pos = Position::new(0., 0.);
        for pos in <Read<Position>>::query()
            .filter(tag_value(&Player))
            .iter(&mut world)
        {
            player_pos = *pos;
        }

        // Detect Collision
        let mut maybe_collision = None;
        for pos in <Read<Position>>::query()
            .filter(tag_value(&Obstacle))
            .iter(&mut world)
        {
            if distance(&player_pos, &*pos) < PLAYER_RADIUS + OBSTACLE_RADIUS {
                maybe_collision = Some(*pos);
            }
        }

        // Adjust player velocity
        for (mut pos, mut vel) in <(Write<Position>, Write<Velocity>)>::query()
            .filter(tag_value(&Player))
            .iter(&mut world)
        {
            // Player's new velocity based on previous velocity and current input
            let max_vel = 0.0008;
            let bounce_vel = 0.0015;
            let velocity_scale = 0.000005;
            let drag = 0.999;
            // Apply drag first
            (*vel).0 *= drag;
            let magnitude_before = (*vel).0.magnitude();
            (*vel).0 += button_processor.direction * velocity_scale;
            // If we're over max velocity, clamp velocity so input only affects post-drag direction
            if (*vel).0.magnitude() > max_vel {
                (*vel).0 = (*vel).0.normalize() * magnitude_before;
            }

            // Collision?
            if let Some(collision_pos) = maybe_collision {
                // Reflect velocity & boost it upon collision
                let normal_vector = (collision_pos - *pos).normalize();
                let surface_vector = Vec2::new(-normal_vector[1], normal_vector[0]);
                let new_velocity =
                    -reflect_vec(&((*vel).0), &surface_vector).normalize() * bounce_vel;
                (*vel).0 = new_velocity;
            }

            // Update position
            let new_pos = *pos + (*vel).0;
            *pos = new_pos;

            // Death by edge?
            if new_pos[0] < -1. - PLAYER_RADIUS
                || new_pos[0] > 1. + PLAYER_RADIUS
                || new_pos[1] < -1. - PLAYER_RADIUS
                || new_pos[1] > 1. + PLAYER_RADIUS
            {
                println!("DEAD");
                dead = true;
            }
        }

        // Draw the Goal
        for (pos, sprite_idx) in <(Read<Position>, Read<SpriteIndex>)>::query()
            .filter(tag_value(&Goal))
            .iter(&mut world)
        {
            let sprite = sprites.get_mut(sprite_idx.0).unwrap();
            sprite.transform.pos = *pos;
            sprite.draw(&mut window);
        }

        // Draw the Obstacles
        for (pos, sprite_idx) in <(Read<Position>, Read<SpriteIndex>)>::query()
            .filter(tag_value(&Obstacle))
            .iter(&mut world)
        {
            let sprite = sprites.get_mut(sprite_idx.0).unwrap();
            sprite.transform.pos = *pos;
            sprite.draw(&mut window);
        }

        // Draw the Player
        for (pos, sprite_idx) in <(Read<Position>, Read<SpriteIndex>)>::query()
            .filter(tag_value(&Player))
            .iter(&mut world)
        {
            let sprite = sprites.get_mut(sprite_idx.0).unwrap();
            sprite.transform.pos = *pos;
            sprite.draw(&mut window);
        }

        window.drawfinish();

        if dead {
            break 'gameloop;
        }
    }
}
