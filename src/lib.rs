pub mod actors;
pub mod ai;
pub mod map;
pub mod rendering;
pub mod visibility;

use bevy::prelude::*;

pub const APP_TITLE: &str = "Roguelike Practice";

pub struct RoguelikePracticePlugin;

impl Plugin for RoguelikePracticePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.035)))
            .insert_resource(map::WorldSeed::default())
            .add_plugins((
                map::MapPlugin,
                rendering::RenderingPlugin,
                actors::ActorsPlugin,
                visibility::VisibilityPlugin,
                ai::AiPlugin,
            ));
    }
}

pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: APP_TITLE.to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }),
        RoguelikePracticePlugin,
    ));
    app
}
