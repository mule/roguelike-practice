use bevy::prelude::*;

use crate::{
    actors::{FacingDirection, GridPosition, Npc, Player},
    ai::NpcTurnPending,
    map::{Map, WorldSeed},
    visibility::{VisibilityDirty, VisibilityState},
};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugSettings {
    pub overlay_visible: bool,
    pub reveal_all: bool,
    pub npc_turns_paused: bool,
    pub allow_paused_npc_turn: bool,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            overlay_visible: true,
            reveal_all: false,
            npc_turns_paused: false,
            allow_paused_npc_turn: false,
        }
    }
}

#[derive(Component)]
struct DebugOverlay;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSettings>()
            .add_systems(Startup, spawn_debug_overlay)
            .add_systems(Update, (handle_debug_input, update_debug_overlay));
    }
}

fn spawn_debug_overlay(mut commands: Commands) {
    commands.spawn((
        DebugOverlay,
        Text::new("debug"),
        TextFont::from_font_size(15.0),
        TextColor(Color::srgb(0.80, 0.92, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
        GlobalZIndex(100),
    ));
}

fn handle_debug_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugSettings>,
    mut visibility_dirty: ResMut<VisibilityDirty>,
    mut npc_turns: ResMut<NpcTurnPending>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        settings.overlay_visible = !settings.overlay_visible;
    }

    if keyboard.just_pressed(KeyCode::F2) {
        settings.reveal_all = !settings.reveal_all;
        visibility_dirty.mark();
    }

    if keyboard.just_pressed(KeyCode::F3) {
        settings.npc_turns_paused = !settings.npc_turns_paused;
    }

    if keyboard.just_pressed(KeyCode::Space) && settings.npc_turns_paused {
        settings.allow_paused_npc_turn = true;
        if !npc_turns.has_pending() {
            npc_turns.request();
        }
    }
}

fn update_debug_overlay(
    settings: Res<DebugSettings>,
    seed: Res<WorldSeed>,
    map: Res<Map>,
    visibility_state: Res<VisibilityState>,
    player: Query<(&GridPosition, &FacingDirection), With<Player>>,
    npcs: Query<&GridPosition, With<Npc>>,
    mut overlays: Query<(&mut Text, &mut Visibility), With<DebugOverlay>>,
) {
    let Some((mut text, mut visibility)) = overlays.iter_mut().next() else {
        return;
    };

    *visibility = if settings.overlay_visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    let player_status = player
        .iter()
        .next()
        .map(|(position, facing)| {
            format!(
                "player=({}, {}) facing={:?}",
                position.0.q, position.0.r, facing.0
            )
        })
        .unwrap_or_else(|| "player=not spawned".to_string());

    **text = format!(
        "seed={} map={} radius={}\n{}\nvisible={} explored={} tiles={} walkable={} npcs={}\nreveal_all={} npc_pause={}\nF1 overlay  F2 reveal  F3 pause NPCs  Space step",
        seed.0,
        map.config().preset.label(),
        map.radius(),
        player_status,
        visibility_state.visible_count(),
        visibility_state.explored_count(),
        map.tile_count(),
        map.walkable_count(),
        npcs.iter().count(),
        settings.reveal_all,
        settings.npc_turns_paused,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_settings_start_in_non_intrusive_mode() {
        let settings = DebugSettings::default();

        assert!(settings.overlay_visible);
        assert!(!settings.reveal_all);
        assert!(!settings.npc_turns_paused);
        assert!(!settings.allow_paused_npc_turn);
    }
}
