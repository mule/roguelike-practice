use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    actors::{
        ActorSystems, FacingDirection, GridPosition, Npc, Player, ScreenDirection, facing_rotation,
        facing_vector,
    },
    map::{HexCoord, Map},
    rendering::axial_to_world,
    visibility::{VisibilityState, VisibilitySystems},
};

const NPC_RADIUS: f32 = 9.0;
const NPC_NOSE_RADIUS: f32 = 2.75;
const NPC_NOSE_OFFSET: f32 = 5.75;
const NPC_Z: f32 = 1.5;
const NPC_NOSE_Z: f32 = 0.1;

type NpcMovementQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut GridPosition,
        &'static mut FacingDirection,
        &'static mut Transform,
    ),
    (With<Npc>, Without<Player>),
>;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NpcTurnPending(u32);

impl NpcTurnPending {
    pub fn request(&mut self) {
        self.0 += 1;
    }

    fn take_one(&mut self) -> bool {
        if self.0 == 0 {
            return false;
        }

        self.0 -= 1;
        true
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApproachPlayer;

pub struct AiPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiSystems {
    Turns,
    Visibility,
}

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcTurnPending>()
            .add_systems(PostStartup, spawn_npcs)
            .add_systems(
                Update,
                (
                    step_npc_turns
                        .in_set(AiSystems::Turns)
                        .after(ActorSystems::PlayerInput),
                    update_npc_visibility
                        .in_set(AiSystems::Visibility)
                        .after(AiSystems::Turns)
                        .after(VisibilitySystems::Refresh),
                ),
            );
    }
}

fn spawn_npcs(
    mut commands: Commands,
    map: Res<Map>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let npc_mesh = meshes.add(Circle::new(NPC_RADIUS));
    let npc_material = materials.add(ColorMaterial::from_color(Color::srgb(0.85, 0.20, 0.38)));
    let nose_mesh = meshes.add(Circle::new(NPC_NOSE_RADIUS));
    let nose_material = materials.add(ColorMaterial::from_color(Color::srgb(0.98, 0.86, 0.24)));

    for coord in map.npc_spawns() {
        let world = axial_to_world(*coord);
        let facing = initial_npc_facing(*coord, map.player_spawn());

        commands
            .spawn((
                Npc,
                ApproachPlayer,
                GridPosition(*coord),
                FacingDirection(facing),
                Mesh2d(npc_mesh.clone()),
                MeshMaterial2d(npc_material.clone()),
                Transform::from_xyz(world.x, world.y, NPC_Z).with_rotation(facing_rotation(facing)),
                Visibility::Hidden,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh2d(nose_mesh.clone()),
                    MeshMaterial2d(nose_material.clone()),
                    Transform::from_xyz(0.0, NPC_NOSE_OFFSET, NPC_NOSE_Z),
                ));
            });
    }
}

fn step_npc_turns(
    mut pending_turns: ResMut<NpcTurnPending>,
    map: Res<Map>,
    player: Single<&GridPosition, With<Player>>,
    mut npcs: NpcMovementQuery,
) {
    if !pending_turns.take_one() {
        return;
    }

    let player_coord = player.0;
    let mut occupied =
        occupied_actor_tiles(player_coord, npcs.iter().map(|(position, _, _)| position.0));

    for (mut position, mut facing, mut transform) in &mut npcs {
        occupied.remove(&position.0);

        if let Some(destination) =
            next_step_toward_target(position.0, player_coord, &occupied, &map)
        {
            if let Some(movement_facing) = facing_for_hex_step(position.0, destination) {
                facing.0 = movement_facing;
                transform.rotation = facing_rotation(movement_facing);
            }

            position.0 = destination;
            let world = axial_to_world(destination);
            transform.translation.x = world.x;
            transform.translation.y = world.y;
        }

        occupied.insert(position.0);
    }
}

fn update_npc_visibility(
    visibility_state: Res<VisibilityState>,
    mut npcs: Query<(&GridPosition, &mut Visibility), With<Npc>>,
) {
    for (position, mut visibility) in &mut npcs {
        *visibility = if visibility_state.is_visible(position.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn occupied_actor_tiles(
    player_coord: HexCoord,
    npc_coords: impl IntoIterator<Item = HexCoord>,
) -> HashSet<HexCoord> {
    HashSet::from([player_coord])
        .into_iter()
        .chain(npc_coords)
        .collect()
}

pub fn facing_for_hex_step(from: HexCoord, to: HexCoord) -> Option<ScreenDirection> {
    HexCoord::DIRECTIONS
        .iter()
        .position(|offset| HexCoord::new(from.q + offset.q, from.r + offset.r) == to)
        .map(|direction| match direction {
            0 => ScreenDirection::East,
            1 => ScreenDirection::Southeast,
            2 => ScreenDirection::Southwest,
            3 => ScreenDirection::West,
            4 => ScreenDirection::Northwest,
            5 => ScreenDirection::Northeast,
            _ => unreachable!("hex direction index is bounded by DIRECTIONS"),
        })
}

fn initial_npc_facing(from: HexCoord, target: HexCoord) -> ScreenDirection {
    if from == target {
        return ScreenDirection::North;
    }

    let to_target = (axial_to_world(target) - axial_to_world(from)).normalize_or_zero();
    let mut best_direction = ScreenDirection::North;
    let mut best_score = f32::NEG_INFINITY;

    for direction in ScreenDirection::ALL {
        let score = facing_vector(direction).dot(to_target);
        if score > best_score {
            best_direction = direction;
            best_score = score;
        }
    }

    best_direction
}

pub fn next_step_toward_target(
    start: HexCoord,
    target: HexCoord,
    occupied: &HashSet<HexCoord>,
    map: &Map,
) -> Option<HexCoord> {
    let target_adjacent_tiles = target_adjacent_tiles(start, target, occupied, map);
    if target_adjacent_tiles.is_empty() || target_adjacent_tiles.contains(&start) {
        return None;
    }

    let mut frontier = VecDeque::from([start]);
    let mut came_from = HashMap::from([(start, start)]);

    while let Some(current) = frontier.pop_front() {
        for neighbor in map.walkable_neighbors(current) {
            if occupied.contains(&neighbor) || came_from.contains_key(&neighbor) {
                continue;
            }

            came_from.insert(neighbor, current);

            if target_adjacent_tiles.contains(&neighbor) {
                return first_step_on_path(start, neighbor, &came_from);
            }

            frontier.push_back(neighbor);
        }
    }

    None
}

fn target_adjacent_tiles(
    start: HexCoord,
    target: HexCoord,
    occupied: &HashSet<HexCoord>,
    map: &Map,
) -> HashSet<HexCoord> {
    map.walkable_neighbors(target)
        .filter(|coord| *coord == start || !occupied.contains(coord))
        .collect()
}

fn first_step_on_path(
    start: HexCoord,
    destination: HexCoord,
    came_from: &HashMap<HexCoord, HexCoord>,
) -> Option<HexCoord> {
    let mut current = destination;

    while let Some(previous) = came_from.get(&current).copied() {
        if previous == start {
            return Some(current);
        }

        current = previous;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Tile;

    #[test]
    fn next_step_moves_toward_a_tile_adjacent_to_the_target() {
        let map = open_test_map(3, &[]);
        let start = HexCoord::new(-2, 0);
        let target = HexCoord::new(1, 0);
        let occupied = HashSet::from([target]);

        let step = next_step_toward_target(start, target, &occupied, &map)
            .expect("open map has a path toward the target");

        assert!(map.is_walkable(step));
        assert_eq!(start.distance(step), 1);
        assert!(step.distance(target) < start.distance(target));
    }

    #[test]
    fn next_step_routes_around_walls() {
        let wall = HexCoord::new(-1, 0);
        let map = open_test_map(3, &[wall]);
        let start = HexCoord::new(-2, 0);
        let target = HexCoord::new(1, 0);
        let occupied = HashSet::from([target]);

        let step = next_step_toward_target(start, target, &occupied, &map)
            .expect("open map has a route around the wall");

        assert_ne!(step, wall);
        assert!(map.is_walkable(step));
        assert_eq!(start.distance(step), 1);
    }

    #[test]
    fn next_step_avoids_occupied_tiles() {
        let map = open_test_map(3, &[]);
        let start = HexCoord::new(-2, 0);
        let target = HexCoord::new(1, 0);
        let occupied = HashSet::from([target, HexCoord::new(-1, 0)]);

        let step = next_step_toward_target(start, target, &occupied, &map)
            .expect("open map has an alternate route");

        assert!(!occupied.contains(&step));
        assert_eq!(start.distance(step), 1);
    }

    #[test]
    fn next_step_stops_when_already_adjacent_to_the_target() {
        let map = open_test_map(3, &[]);
        let start = HexCoord::new(0, 0);
        let target = HexCoord::new(1, 0);
        let occupied = HashSet::from([target]);

        assert_eq!(
            next_step_toward_target(start, target, &occupied, &map),
            None
        );
    }

    #[test]
    fn facing_for_hex_step_maps_each_hex_neighbor_to_screen_facing() {
        let origin = HexCoord::new(0, 0);

        assert_eq!(
            facing_for_hex_step(origin, origin.neighbor(0)),
            Some(ScreenDirection::East)
        );
        assert_eq!(
            facing_for_hex_step(origin, origin.neighbor(1)),
            Some(ScreenDirection::Southeast)
        );
        assert_eq!(
            facing_for_hex_step(origin, origin.neighbor(2)),
            Some(ScreenDirection::Southwest)
        );
        assert_eq!(
            facing_for_hex_step(origin, origin.neighbor(3)),
            Some(ScreenDirection::West)
        );
        assert_eq!(
            facing_for_hex_step(origin, origin.neighbor(4)),
            Some(ScreenDirection::Northwest)
        );
        assert_eq!(
            facing_for_hex_step(origin, origin.neighbor(5)),
            Some(ScreenDirection::Northeast)
        );
    }

    #[test]
    fn facing_for_hex_step_ignores_non_adjacent_movement() {
        assert_eq!(
            facing_for_hex_step(HexCoord::new(0, 0), HexCoord::new(2, 0)),
            None
        );
    }

    #[test]
    fn initial_npc_facing_points_toward_the_player() {
        assert_eq!(
            initial_npc_facing(HexCoord::new(-2, 0), HexCoord::new(0, 0)),
            ScreenDirection::East
        );
        assert_eq!(
            initial_npc_facing(HexCoord::new(2, 0), HexCoord::new(0, 0)),
            ScreenDirection::West
        );
        assert_eq!(
            initial_npc_facing(HexCoord::new(1, -2), HexCoord::new(0, 0)),
            ScreenDirection::North
        );
    }

    fn open_test_map(radius: i32, wall_coords: &[HexCoord]) -> Map {
        let walls: HashSet<_> = wall_coords.iter().copied().collect();
        let mut tiles = Vec::new();

        for q in -radius..=radius {
            for r in -radius..=radius {
                let coord = HexCoord::new(q, r);
                if HexCoord::new(0, 0).distance(coord) <= radius {
                    let tile = if walls.contains(&coord) {
                        Tile::wall()
                    } else {
                        Tile::floor()
                    };
                    tiles.push((coord, tile));
                }
            }
        }

        Map::from_tiles_for_test(radius, tiles, HexCoord::new(0, 0))
    }
}
