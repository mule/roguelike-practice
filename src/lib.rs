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
const MAP_SIZE_ENV_VAR: &str = "ROGUELIKE_MAP_SIZE";
const MAP_RADIUS_ENV_VAR: &str = "ROGUELIKE_MAP_RADIUS";

pub struct RoguelikePracticePlugin;

impl Plugin for RoguelikePracticePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.035)))
            .insert_resource(configured_world_seed())
            .insert_resource(configured_map_generation_config())
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

fn configured_map_generation_config() -> map::MapGenerationConfig {
    map_generation_config_from_sources(
        env::args(),
        env::var(MAP_SIZE_ENV_VAR).ok().as_deref(),
        env::var(MAP_RADIUS_ENV_VAR).ok().as_deref(),
    )
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

fn map_generation_config_from_sources(
    args: impl IntoIterator<Item = String>,
    env_size: Option<&str>,
    env_radius: Option<&str>,
) -> map::MapGenerationConfig {
    let (cli_size, cli_radius) = cli_map_generation_config(args);

    cli_radius
        .or_else(|| {
            cli_size
                .as_deref()
                .and_then(map::MapGenerationConfig::named)
        })
        .or_else(|| env_radius.and_then(parse_map_radius))
        .or_else(|| env_size.and_then(map::MapGenerationConfig::named))
        .unwrap_or_default()
}

fn cli_map_generation_config(
    args: impl IntoIterator<Item = String>,
) -> (Option<String>, Option<map::MapGenerationConfig>) {
    let mut args = args.into_iter().skip(1);
    let mut size = None;
    let mut radius = None;

    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--map-size=") {
            size = Some(value.to_string());
            continue;
        }

        if let Some(value) = arg.strip_prefix("--map-radius=") {
            radius = parse_map_radius(value);
            continue;
        }

        if arg == "--map-size" {
            size = args.next();
            continue;
        }

        if arg == "--map-radius" {
            radius = args.next().as_deref().and_then(parse_map_radius);
        }
    }

    (size, radius)
}

fn parse_map_radius(value: &str) -> Option<map::MapGenerationConfig> {
    value
        .parse()
        .ok()
        .and_then(map::MapGenerationConfig::custom)
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

    #[test]
    fn map_generation_config_defaults_when_no_override_exists() {
        assert_eq!(
            map_generation_config_from_sources(["app".to_string()], None, None),
            map::MapGenerationConfig::default()
        );
    }

    #[test]
    fn map_generation_config_can_come_from_environment() {
        assert_eq!(
            map_generation_config_from_sources(["app".to_string()], Some("large"), None),
            map::MapGenerationConfig::large()
        );
        assert_eq!(
            map_generation_config_from_sources(["app".to_string()], Some("small"), Some("18")),
            map::MapGenerationConfig::custom(18).expect("valid custom radius")
        );
    }

    #[test]
    fn command_line_map_size_overrides_environment() {
        assert_eq!(
            map_generation_config_from_sources(
                [
                    "app".to_string(),
                    "--map-size".to_string(),
                    "small".to_string()
                ],
                Some("large"),
                None,
            ),
            map::MapGenerationConfig::small()
        );
        assert_eq!(
            map_generation_config_from_sources(
                ["app".to_string(), "--map-size=large".to_string()],
                Some("small"),
                None,
            ),
            map::MapGenerationConfig::large()
        );
    }

    #[test]
    fn command_line_map_radius_overrides_named_sizes() {
        assert_eq!(
            map_generation_config_from_sources(
                [
                    "app".to_string(),
                    "--map-size".to_string(),
                    "small".to_string(),
                    "--map-radius".to_string(),
                    "18".to_string(),
                ],
                Some("large"),
                Some("20"),
            ),
            map::MapGenerationConfig::custom(18).expect("valid custom radius")
        );
        assert_eq!(
            map_generation_config_from_sources(
                ["app".to_string(), "--map-radius=19".to_string()],
                Some("small"),
                None,
            ),
            map::MapGenerationConfig::custom(19).expect("valid custom radius")
        );
    }

    #[test]
    fn invalid_map_generation_config_falls_back_to_default() {
        assert_eq!(
            map_generation_config_from_sources(
                [
                    "app".to_string(),
                    "--map-size".to_string(),
                    "huge".to_string(),
                    "--map-radius".to_string(),
                    "999".to_string(),
                ],
                Some("enormous"),
                Some("2"),
            ),
            map::MapGenerationConfig::default()
        );
    }
}
