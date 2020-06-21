use legion::prelude::*;
use rand::prelude::*;
use rusty_engine::gfx::event::{ButtonProcessor, GameEvent};
use rusty_engine::gfx::{color::Color, Sprite, Window};
use rusty_engine::glm::{distance2, Vec2};

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
            1. / 8.,
            Color::new(0., 1., 0.),
        ),
        // Player circle (small-ish, blue)
        Sprite::smooth_circle(
            &window,
            Position::new(0., 0.), // Ignored
            0.,
            1.,
            1. / 16.,
            Color::new(0., 0., 1.),
        ),
        // Obstacle circles -- reusing the same sprite for all instances is probably a terrible idea, but I want to try it
        Sprite::smooth_circle(
            &window,
            Position::new(0., 0.), // Ignored
            0.,
            1.,
            1. / 12.,
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

        // Physics stuff
        for (mut pos, mut vel) in <(Write<Position>, Write<Velocity>)>::query()
            .filter(tag_value(&Player))
            .iter(&mut world)
        {
            let max_vel = 0.0005;
            let velocity_scale = 0.000005;
            let drag = 0.999;
            (*vel).0 += button_processor.direction * velocity_scale;
            (*vel).0 *= drag;
            if (*vel).0.magnitude() > max_vel {
                (*vel).0 = (*vel).0.normalize() * max_vel;
            }
            *pos = *pos + (*vel).0;
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
    }
}
