use bevy::prelude::*;

use crate::{
    map::{HexCoord, Map},
    rendering::axial_to_world,
};

const PLAYER_RADIUS: f32 = 13.0;
const PLAYER_Z: f32 = 2.0;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition(pub HexCoord);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Npc;

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_player)
            .add_systems(Update, move_player);
    }
}

fn spawn_player(
    mut commands: Commands,
    map: Res<Map>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let coord = map.player_spawn();
    let world = axial_to_world(coord);

    commands.spawn((
        Player,
        GridPosition(coord),
        Mesh2d(meshes.add(RegularPolygon::new(PLAYER_RADIUS, 6))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.88, 0.92, 0.42)))),
        Transform::from_xyz(world.x, world.y, PLAYER_Z)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_6)),
    ));
}

fn move_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    map: Res<Map>,
    mut player: Single<(&mut GridPosition, &mut Transform), With<Player>>,
) {
    let Some(direction) = pressed_hex_direction(&keyboard) else {
        return;
    };

    let (grid_position, transform) = &mut *player;
    if let Some(destination) = walk_destination(grid_position.0, direction, &map) {
        grid_position.0 = destination;
        let world = axial_to_world(destination);
        transform.translation.x = world.x;
        transform.translation.y = world.y;
    }
}

fn pressed_hex_direction(keyboard: &ButtonInput<KeyCode>) -> Option<usize> {
    [
        (KeyCode::KeyE, 0),
        (KeyCode::KeyD, 1),
        (KeyCode::KeyS, 2),
        (KeyCode::KeyA, 3),
        (KeyCode::KeyQ, 4),
        (KeyCode::KeyW, 5),
    ]
    .into_iter()
    .find_map(|(key, direction)| keyboard.just_pressed(key).then_some(direction))
}

pub fn walk_destination(current: HexCoord, direction: usize, map: &Map) -> Option<HexCoord> {
    let destination = current.neighbor(direction);
    map.is_walkable(destination).then_some(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::WorldSeed;

    #[test]
    fn walk_destination_allows_adjacent_walkable_tiles() {
        let map = Map::starter(WorldSeed(42));
        let start = map.player_spawn();
        let destination = start
            .neighbors()
            .into_iter()
            .find(|coord| map.is_walkable(*coord))
            .expect("starter player spawn has at least one walkable neighbor");
        let direction = HexCoord::DIRECTIONS
            .iter()
            .position(|offset| HexCoord::new(start.q + offset.q, start.r + offset.r) == destination)
            .expect("destination is adjacent");

        assert_eq!(walk_destination(start, direction, &map), Some(destination));
    }

    #[test]
    fn walk_destination_rejects_blocked_tiles() {
        let map = Map::starter(WorldSeed(42));
        let (wall, floor_neighbor) = map
            .tiles()
            .map(|(coord, _tile)| coord)
            .filter(|coord| !map.is_walkable(*coord))
            .find_map(|wall| {
                wall.neighbors()
                    .into_iter()
                    .find(|coord| map.is_walkable(*coord))
                    .map(|floor| (wall, floor))
            })
            .expect("starter map has a wall adjacent to a floor");
        let direction = HexCoord::DIRECTIONS
            .iter()
            .position(|offset| {
                HexCoord::new(floor_neighbor.q + offset.q, floor_neighbor.r + offset.r) == wall
            })
            .expect("wall is adjacent");

        assert_eq!(walk_destination(floor_neighbor, direction, &map), None);
    }
}
