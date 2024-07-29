use legion::prelude::*;
use rand::prelude::*;
use rusty_engine::audio::Audio;
use rusty_engine::gfx::event::{ButtonProcessor, GameEvent};
use rusty_engine::gfx::ShapeStyle;
use rusty_engine::gfx::{color::Color, Sprite, Window};
use rusty_engine::glm::{distance, distance2, reflect_vec, Vec2};
use std::time::Instant;

const GOAL_RADIUS: f32 = 1. / 8.;
const OBSTACLE_RADIUS: f32 = 1. / 12.;
const PLAYER_RADIUS: f32 = 1. / 16.;
const LIFE_MAX: i32 = 10;
const LIFE_CIRCLE_RADIUS: f32 = 1. / 48.;
const ENEMY_WIDTH: f32 = 1. / 8.;

type Position = Vec2;
struct Velocity(Vec2);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Goal;
#[derive(Clone, Copy, Debug, PartialEq)]
struct LifeCircle;
#[derive(Clone, Copy, Debug, PartialEq)]
struct Obstacle;
#[derive(Clone, Copy, Debug, PartialEq)]
struct Player;
#[derive(Clone, Copy, Debug, PartialEq)]
struct SpriteIndex(usize);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Enemy;

fn main() {
    let mut audio = Audio::new();
    audio.add("bounce", "sound/bounce.wav");
    audio.add("death", "sound/death.wav");
    audio.add("startup", "sound/startup.wav");
    audio.add("warning_one_life", "sound/warning_one_life.wav");
    audio.add("win", "sound/win.wav");
    audio.play("startup");

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
        // Life Circles - each circle represents a unit of life
        Sprite::smooth_circle(
            &window,
            Position::new(0., 0.), // Ignored
            0.,
            1.,
            LIFE_CIRCLE_RADIUS,
            Color::new(0., 0., 1.),
        ),
        // Enemy - Square enemy chases the player
        Sprite::new_rectangle(
            &window,
            Position::new(0., 0.),
            0.,
            1.,
            ENEMY_WIDTH,
            ENEMY_WIDTH,
            Color::new(1.0, 1.0, 0.0),
            ShapeStyle::Fill,
        ),
    ];

    let goal_start_pos = Position::new(0.75, -0.75);
    world.insert((Goal,), vec![(goal_start_pos, SpriteIndex(0))]);
    let player_start_pos = Position::new(-0.75, 0.75);
    world.insert(
        (Player,),
        vec![(
            player_start_pos,
            Velocity(Vec2::new(0.0, 0.0)),
            SpriteIndex(1),
        )],
    );

    // Obstacle starting places
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

    // Enemy starting place
    world.insert((Enemy,), vec![(Position::new(0.75, 0.75),)]);

    // GAME LOOP
    let mut life = LIFE_MAX;
    let mut button_processor = ButtonProcessor::new();
    let mut instant = Instant::now();
    'gameloop: loop {
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut dead = false;

        // Process player input
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

        // Get the player's position
        let mut player_pos = Position::new(0., 0.);
        for pos in <Read<Position>>::query()
            .filter(tag_value(&Player))
            .iter(&mut world)
        {
            player_pos = *pos;
        }

        // Detect Obstacle Collision
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
            .iter_mut(&mut world)
        {
            // Player's new velocity based on previous velocity and current input
            let max_vel = 0.5;
            let win_vel = 0.9;
            let bounce_vel = 0.75;
            let input_scale = 1.;
            let drag = 0.8;

            // Apply drag first
            (*vel).0 *= 1.0 - drag * delta.as_secs_f32();

            // Then apply accelleration in the direction of the input
            let magnitude_before = (*vel).0.magnitude();
            (*vel).0 += button_processor.direction * input_scale * delta.as_secs_f32();

            // If we're over max velocity, clamp velocity magnitude to the same as before input
            // accelleration so input only affects direction.
            if (*vel).0.magnitude() > max_vel && (*vel).0.magnitude() > magnitude_before {
                (*vel).0 = (*vel).0.normalize() * magnitude_before;
            }

            // Collision with obstacle?
            if let Some(collision_pos) = maybe_collision {
                // Colliding hurts
                life -= 1;
                if life <= 0 {
                    dead = true;
                }
                // Colliding makes a sound of some type
                if life == 1 {
                    audio.play("warning_one_life");
                } else {
                    audio.play("bounce");
                }
                // Reflect velocity & boost it upon collision
                let normal_vector = (collision_pos - *pos).normalize();
                let surface_vector = Vec2::new(-normal_vector[1], normal_vector[0]);
                let new_velocity =
                    -reflect_vec(&((*vel).0), &surface_vector).normalize() * bounce_vel;
                (*vel).0 = new_velocity;
            }

            // Almost to the goal?
            let goal_distance = distance(&*pos, &goal_start_pos);
            if goal_distance < PLAYER_RADIUS + GOAL_RADIUS {
                (*vel).0 += ((goal_start_pos - *pos).normalize() * delta.as_secs_f32()).normalize()
                    * win_vel
                    * delta.as_secs_f32();
            }

            // Reached the goal?
            if goal_distance < (PLAYER_RADIUS + GOAL_RADIUS) / 3. {
                println!("YOU WIN!");
                audio.play("win");
                break 'gameloop;
            }

            // Update position
            let new_pos = *pos + (*vel).0 * delta.as_secs_f32();
            *pos = new_pos;

            // Death by edge?
            if new_pos[0] < -1. - PLAYER_RADIUS
                || new_pos[0] > 1. + PLAYER_RADIUS
                || new_pos[1] < -1. - PLAYER_RADIUS
                || new_pos[1] > 1. + PLAYER_RADIUS
            {
                dead = true;
            }
        }

        // RENDER THE SCENE
        window.drawstart();

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

        // Draw the life circles
        for i in 0..life {
            let pos = Position::new(
                -1.0 + LIFE_CIRCLE_RADIUS + (2.0 * i as f32 * LIFE_CIRCLE_RADIUS),
                1.0 - LIFE_CIRCLE_RADIUS,
            );
            let sprite = sprites.get_mut(3).unwrap();
            sprite.transform.pos = pos;
            sprite.draw(&mut window);
        }

        // Draw the enemy
        for (pos,) in <(Read<Position>,)>::query()
            .filter(tag_value(&Enemy))
            .iter(&mut world)
        {
            let sprite = sprites.get_mut(4).unwrap();
            sprite.transform.pos = *pos;
            sprite.draw(&mut window);
        }

        window.drawfinish();

        if dead {
            println!("YOU DIED!");
            audio.play("death");
            break 'gameloop;
        }
    }
    audio.wait();
}
