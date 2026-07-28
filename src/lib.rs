pub mod actors;
pub mod ai;
pub mod debug;
pub mod map;
pub mod rendering;
pub mod visibility;

use bevy::prelude::*;
use std::env;

pub const APP_TITLE: &str = "Roguelike Practice";
const SEED_ENV_VAR: &str = "ROGUELIKE_SEED";

pub struct RoguelikePracticePlugin;

impl Plugin for RoguelikePracticePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.035)))
            .insert_resource(configured_world_seed())
            .add_plugins((
                map::MapPlugin,
                rendering::RenderingPlugin,
                actors::ActorsPlugin,
                visibility::VisibilityPlugin,
                ai::AiPlugin,
                debug::DebugPlugin,
            ));
    }
}

fn configured_world_seed() -> map::WorldSeed {
    world_seed_from_sources(env::args(), env::var(SEED_ENV_VAR).ok().as_deref())
}

fn world_seed_from_sources(
    args: impl IntoIterator<Item = String>,
    env_seed: Option<&str>,
) -> map::WorldSeed {
    cli_seed(args)
        .or_else(|| env_seed.and_then(parse_seed))
        .unwrap_or_default()
}

fn cli_seed(args: impl IntoIterator<Item = String>) -> Option<map::WorldSeed> {
    let mut args = args.into_iter().skip(1);

    while let Some(arg) = args.next() {
        if let Some(seed) = arg.strip_prefix("--seed=") {
            return parse_seed(seed);
        }

        if arg == "--seed" {
            return args.next().as_deref().and_then(parse_seed);
        }
    }

    None
}

fn parse_seed(value: &str) -> Option<map::WorldSeed> {
    value.parse().ok().map(map::WorldSeed)
}

pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: APP_TITLE.to_string(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            })
            .disable::<bevy::audio::AudioPlugin>(),
        RoguelikePracticePlugin,
    ));
    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_seed_defaults_when_no_override_exists() {
        assert_eq!(
            world_seed_from_sources(["app".to_string()], None),
            map::WorldSeed::default()
        );
    }

    #[test]
    fn world_seed_can_come_from_environment() {
        assert_eq!(
            world_seed_from_sources(["app".to_string()], Some("123")),
            map::WorldSeed(123)
        );
    }

    #[test]
    fn command_line_seed_overrides_environment_seed() {
        assert_eq!(
            world_seed_from_sources(
                ["app".to_string(), "--seed".to_string(), "456".to_string()],
                Some("123"),
            ),
            map::WorldSeed(456)
        );
        assert_eq!(
            world_seed_from_sources(["app".to_string(), "--seed=789".to_string()], Some("123"),),
            map::WorldSeed(789)
        );
    }

    #[test]
    fn invalid_seed_falls_back_to_default() {
        assert_eq!(
            world_seed_from_sources(
                ["app".to_string(), "--seed=nope".to_string()],
                Some("also-nope")
            ),
            map::WorldSeed::default()
        );
    }
}
