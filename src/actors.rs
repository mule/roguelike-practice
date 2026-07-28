use bevy::prelude::*;

use crate::{
    ai::NpcTurnPending,
    map::{HexCoord, Map},
    rendering::axial_to_world,
    visibility::VisibilityDirty,
};

const PLAYER_RADIUS: f32 = 13.0;
const PLAYER_NOSE_RADIUS: f32 = 3.5;
const PLAYER_NOSE_OFFSET: f32 = 8.0;
const PLAYER_Z: f32 = 2.0;
const PLAYER_NOSE_Z: f32 = 0.1;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition(pub HexCoord);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FacingDirection(pub ScreenDirection);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerticalStepPhase {
    use_right_diagonal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenDirection {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
}

impl ScreenDirection {
    pub const ALL: [Self; 8] = [
        Self::North,
        Self::Northeast,
        Self::East,
        Self::Southeast,
        Self::South,
        Self::Southwest,
        Self::West,
        Self::Northwest,
    ];

    pub const fn rotate_left(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub const fn rotate_right(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub const fn opposite(self) -> Self {
        Self::ALL[(self.index() + 4) % Self::ALL.len()]
    }

    const fn index(self) -> usize {
        match self {
            Self::North => 0,
            Self::Northeast => 1,
            Self::East => 2,
            Self::Southeast => 3,
            Self::South => 4,
            Self::Southwest => 5,
            Self::West => 6,
            Self::Northwest => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideStepDirection {
    Left,
    Right,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Npc;

pub struct ActorsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorSystems {
    PlayerInput,
}

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_player)
            .add_systems(Update, move_player.in_set(ActorSystems::PlayerInput));
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

    commands
        .spawn((
            Player,
            GridPosition(coord),
            FacingDirection(ScreenDirection::North),
            VerticalStepPhase::default(),
            Mesh2d(meshes.add(RegularPolygon::new(PLAYER_RADIUS, 3))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.88, 0.92, 0.42)))),
            Transform::from_xyz(world.x, world.y, PLAYER_Z)
                .with_rotation(facing_rotation(ScreenDirection::North)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh2d(meshes.add(Circle::new(PLAYER_NOSE_RADIUS))),
                MeshMaterial2d(
                    materials.add(ColorMaterial::from_color(Color::srgb(0.08, 0.12, 0.16))),
                ),
                Transform::from_xyz(0.0, PLAYER_NOSE_OFFSET, PLAYER_NOSE_Z),
            ));
        });
}

fn move_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    map: Res<Map>,
    mut visibility_dirty: ResMut<VisibilityDirty>,
    mut npc_turns: ResMut<NpcTurnPending>,
    mut player: Single<
        (
            &mut GridPosition,
            &mut FacingDirection,
            &mut VerticalStepPhase,
            &mut Transform,
        ),
        With<Player>,
    >,
) {
    let Some(action) = pressed_player_action(&keyboard) else {
        return;
    };

    let (grid_position, facing, phase, transform) = &mut *player;
    match action {
        PlayerAction::RotateLeft => {
            facing.0 = facing.0.rotate_left();
            transform.rotation = facing_rotation(facing.0);
            visibility_dirty.mark();
        }
        PlayerAction::RotateRight => {
            facing.0 = facing.0.rotate_right();
            transform.rotation = facing_rotation(facing.0);
            visibility_dirty.mark();
        }
        PlayerAction::MoveForward => {
            if try_move(grid_position, facing.0, phase, transform, &map) {
                visibility_dirty.mark();
                npc_turns.request();
            }
        }
        PlayerAction::MoveBackward => {
            if try_move(grid_position, facing.0.opposite(), phase, transform, &map) {
                visibility_dirty.mark();
                npc_turns.request();
            }
        }
        PlayerAction::SideStepLeft => {
            if try_side_step(
                grid_position,
                facing.0,
                SideStepDirection::Left,
                transform,
                &map,
            ) {
                visibility_dirty.mark();
                npc_turns.request();
            }
        }
        PlayerAction::SideStepRight => {
            if try_side_step(
                grid_position,
                facing.0,
                SideStepDirection::Right,
                transform,
                &map,
            ) {
                visibility_dirty.mark();
                npc_turns.request();
            }
        }
    }
}

fn try_move(
    grid_position: &mut GridPosition,
    movement_direction: ScreenDirection,
    phase: &mut VerticalStepPhase,
    transform: &mut Transform,
    map: &Map,
) -> bool {
    if let Some(destination) = walk_destination(
        grid_position.0,
        movement_direction,
        phase.use_right_diagonal,
        map,
    ) {
        if matches!(
            movement_direction,
            ScreenDirection::North | ScreenDirection::South
        ) {
            phase.use_right_diagonal = !phase.use_right_diagonal;
        }

        move_to_grid_position(grid_position, destination, transform);

        return true;
    }

    false
}

fn try_side_step(
    grid_position: &mut GridPosition,
    facing: ScreenDirection,
    side: SideStepDirection,
    transform: &mut Transform,
    map: &Map,
) -> bool {
    if let Some(destination) = side_step_destination(grid_position.0, facing, side, map) {
        move_to_grid_position(grid_position, destination, transform);
        return true;
    }

    false
}

fn move_to_grid_position(
    grid_position: &mut GridPosition,
    destination: HexCoord,
    transform: &mut Transform,
) {
    grid_position.0 = destination;
    let world = axial_to_world(destination);
    transform.translation.x = world.x;
    transform.translation.y = world.y;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerAction {
    MoveForward,
    MoveBackward,
    SideStepLeft,
    SideStepRight,
    RotateLeft,
    RotateRight,
}

fn pressed_player_action(keyboard: &ButtonInput<KeyCode>) -> Option<PlayerAction> {
    [
        (KeyCode::KeyW, PlayerAction::MoveForward),
        (KeyCode::KeyS, PlayerAction::MoveBackward),
        (KeyCode::KeyA, PlayerAction::SideStepLeft),
        (KeyCode::KeyD, PlayerAction::SideStepRight),
        (KeyCode::KeyQ, PlayerAction::RotateLeft),
        (KeyCode::KeyE, PlayerAction::RotateRight),
    ]
    .into_iter()
    .find_map(|(key, action)| keyboard.just_pressed(key).then_some(action))
}

pub fn walk_destination(
    current: HexCoord,
    direction: ScreenDirection,
    use_right_diagonal: bool,
    map: &Map,
) -> Option<HexCoord> {
    let destination = current.neighbor(hex_direction_for_screen_direction(
        direction,
        use_right_diagonal,
    ));
    map.is_walkable(destination).then_some(destination)
}

pub fn side_step_destination(
    current: HexCoord,
    facing: ScreenDirection,
    side: SideStepDirection,
    map: &Map,
) -> Option<HexCoord> {
    let destination = current.neighbor(side_step_hex_direction(facing, side));
    map.is_walkable(destination).then_some(destination)
}

pub const fn hex_direction_for_screen_direction(
    direction: ScreenDirection,
    use_right_diagonal: bool,
) -> usize {
    match (direction, use_right_diagonal) {
        (ScreenDirection::East, _) => 0,
        (ScreenDirection::Northeast, _) => 5,
        (ScreenDirection::Southeast, _) => 1,
        (ScreenDirection::Southwest, _) => 2,
        (ScreenDirection::West, _) => 3,
        (ScreenDirection::Northwest, _) => 4,
        (ScreenDirection::South, true) => 1,
        (ScreenDirection::South, false) => 2,
        (ScreenDirection::North, false) => 4,
        (ScreenDirection::North, true) => 5,
    }
}

pub fn side_step_hex_direction(facing: ScreenDirection, side: SideStepDirection) -> usize {
    let facing_vector = facing_vector(facing);
    let lateral_vector = match side {
        SideStepDirection::Left => Vec2::new(-facing_vector.y, facing_vector.x),
        SideStepDirection::Right => Vec2::new(facing_vector.y, -facing_vector.x),
    };

    let mut best_direction = 0;
    let mut best_score = f32::NEG_INFINITY;
    for hex_direction in 0..HexCoord::DIRECTIONS.len() {
        let score = hex_step_vector(hex_direction).dot(lateral_vector);
        if score > best_score {
            best_direction = hex_direction;
            best_score = score;
        }
    }

    best_direction
}

pub fn facing_rotation(direction: ScreenDirection) -> Quat {
    let radians = match direction {
        ScreenDirection::North => 0.0,
        ScreenDirection::Northeast => -std::f32::consts::FRAC_PI_4,
        ScreenDirection::East => -std::f32::consts::FRAC_PI_2,
        ScreenDirection::Southeast => -3.0 * std::f32::consts::FRAC_PI_4,
        ScreenDirection::South => std::f32::consts::PI,
        ScreenDirection::Southwest => 3.0 * std::f32::consts::FRAC_PI_4,
        ScreenDirection::West => std::f32::consts::FRAC_PI_2,
        ScreenDirection::Northwest => std::f32::consts::FRAC_PI_4,
    };

    Quat::from_rotation_z(radians)
}

pub fn facing_vector(direction: ScreenDirection) -> Vec2 {
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    match direction {
        ScreenDirection::North => Vec2::Y,
        ScreenDirection::Northeast => Vec2::new(diagonal, diagonal),
        ScreenDirection::East => Vec2::X,
        ScreenDirection::Southeast => Vec2::new(diagonal, -diagonal),
        ScreenDirection::South => Vec2::NEG_Y,
        ScreenDirection::Southwest => Vec2::new(-diagonal, -diagonal),
        ScreenDirection::West => Vec2::NEG_X,
        ScreenDirection::Northwest => Vec2::new(-diagonal, diagonal),
    }
}

fn hex_step_vector(direction: usize) -> Vec2 {
    let origin = HexCoord::new(0, 0);
    (axial_to_world(origin.neighbor(direction)) - axial_to_world(origin)).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Tile, WorldSeed};

    #[test]
    fn walk_destination_allows_adjacent_walkable_tiles() {
        let map = Map::starter(WorldSeed(42));
        let start = map.player_spawn();
        let direction = ScreenDirection::ALL
            .into_iter()
            .find(|direction| walk_destination(start, *direction, false, &map).is_some())
            .expect("starter player spawn has at least one walkable neighbor");

        assert!(walk_destination(start, direction, false, &map).is_some());
    }

    #[test]
    fn walk_destination_rejects_blocked_tiles() {
        let map = Map::starter(WorldSeed(42));
        let (_wall, floor_neighbor, hex_direction) = map
            .tiles()
            .map(|(coord, _tile)| coord)
            .filter(|coord| !map.is_walkable(*coord))
            .find_map(|wall| {
                wall.neighbors()
                    .into_iter()
                    .find(|coord| map.is_walkable(*coord))
                    .and_then(|floor| {
                        HexCoord::DIRECTIONS
                            .iter()
                            .position(|offset| {
                                HexCoord::new(floor.q + offset.q, floor.r + offset.r) == wall
                            })
                            .map(|direction| (wall, floor, direction))
                    })
            })
            .expect("starter map has a diagonal wall adjacent to a floor");
        let (direction, phase) = screen_direction_for_hex_direction(hex_direction);

        assert_eq!(
            walk_destination(floor_neighbor, direction, phase, &map),
            None
        );
    }

    #[test]
    fn vertical_movement_alternates_diagonal_steps() {
        let start = HexCoord::new(0, 0);

        let north_left = start.neighbor(hex_direction_for_screen_direction(
            ScreenDirection::North,
            false,
        ));
        let north_right = north_left.neighbor(hex_direction_for_screen_direction(
            ScreenDirection::North,
            true,
        ));
        assert_eq!(north_left, HexCoord::new(-1, 1));
        assert_eq!(north_right, HexCoord::new(-1, 2));

        let south_left = start.neighbor(hex_direction_for_screen_direction(
            ScreenDirection::South,
            false,
        ));
        let south_right = south_left.neighbor(hex_direction_for_screen_direction(
            ScreenDirection::South,
            true,
        ));
        assert_eq!(south_left, HexCoord::new(0, -1));
        assert_eq!(south_right, HexCoord::new(1, -2));
    }

    #[test]
    fn diagonal_forward_and_backward_are_opposites() {
        assert_eq!(
            ScreenDirection::Northeast.opposite(),
            ScreenDirection::Southwest
        );
        assert_eq!(
            hex_direction_for_screen_direction(ScreenDirection::Northeast, false),
            5
        );
        assert_eq!(
            hex_direction_for_screen_direction(ScreenDirection::Southwest, false),
            2
        );
    }

    #[test]
    fn east_and_west_forward_and_backward_are_opposites() {
        assert_eq!(ScreenDirection::East.opposite(), ScreenDirection::West);
        assert_eq!(
            hex_direction_for_screen_direction(ScreenDirection::East, false),
            0
        );
        assert_eq!(
            hex_direction_for_screen_direction(ScreenDirection::West, false),
            3
        );
    }

    #[test]
    fn side_step_uses_actor_relative_lateral_directions() {
        assert_eq!(
            side_step_hex_direction(ScreenDirection::North, SideStepDirection::Left),
            3
        );
        assert_eq!(
            side_step_hex_direction(ScreenDirection::North, SideStepDirection::Right),
            0
        );
        assert_eq!(
            side_step_hex_direction(ScreenDirection::East, SideStepDirection::Left),
            4
        );
        assert_eq!(
            side_step_hex_direction(ScreenDirection::East, SideStepDirection::Right),
            1
        );
        assert_eq!(
            side_step_hex_direction(ScreenDirection::Southwest, SideStepDirection::Left),
            1
        );
        assert_eq!(
            side_step_hex_direction(ScreenDirection::Southwest, SideStepDirection::Right),
            4
        );
    }

    #[test]
    fn side_step_destination_does_not_depend_on_global_east_and_west() {
        let map = open_test_map(2);
        let start = HexCoord::new(0, 0);

        assert_eq!(
            side_step_destination(start, ScreenDirection::North, SideStepDirection::Left, &map),
            Some(HexCoord::new(-1, 0))
        );
        assert_eq!(
            side_step_destination(start, ScreenDirection::East, SideStepDirection::Left, &map),
            Some(HexCoord::new(-1, 1))
        );
    }

    #[test]
    fn facing_direction_rotates_through_eight_facings() {
        assert_eq!(
            ScreenDirection::North.rotate_left(),
            ScreenDirection::Northwest
        );
        assert_eq!(
            ScreenDirection::North.rotate_right(),
            ScreenDirection::Northeast
        );
        assert_eq!(
            ScreenDirection::Northeast.rotate_right(),
            ScreenDirection::East
        );
        assert_eq!(
            ScreenDirection::Southwest.rotate_right(),
            ScreenDirection::West
        );
        assert_eq!(
            ScreenDirection::South.rotate_left(),
            ScreenDirection::Southeast
        );
        assert_eq!(
            ScreenDirection::South.rotate_right(),
            ScreenDirection::Southwest
        );
    }

    fn screen_direction_for_hex_direction(hex_direction: usize) -> (ScreenDirection, bool) {
        match hex_direction {
            0 => (ScreenDirection::East, false),
            1 => (ScreenDirection::Southeast, false),
            2 => (ScreenDirection::Southwest, false),
            3 => (ScreenDirection::West, false),
            4 => (ScreenDirection::Northwest, false),
            5 => (ScreenDirection::Northeast, false),
            _ => unreachable!("hex direction is wrapped before this helper"),
        }
    }

    fn open_test_map(radius: i32) -> Map {
        let mut tiles = Vec::new();

        for q in -radius..=radius {
            for r in -radius..=radius {
                let coord = HexCoord::new(q, r);
                if HexCoord::new(0, 0).distance(coord) <= radius {
                    tiles.push((coord, Tile::floor()));
                }
            }
        }

        Map::from_tiles_for_test(radius, tiles, HexCoord::new(0, 0))
    }
}
