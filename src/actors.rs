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
    Southeast,
    South,
    Southwest,
    Northwest,
}

impl ScreenDirection {
    const ALL: [Self; 6] = [
        Self::North,
        Self::Northeast,
        Self::Southeast,
        Self::South,
        Self::Southwest,
        Self::Northwest,
    ];

    pub const fn rotate_left(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub const fn rotate_right(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub const fn opposite(self) -> Self {
        Self::ALL[(self.index() + 3) % Self::ALL.len()]
    }

    const fn index(self) -> usize {
        match self {
            Self::North => 0,
            Self::Northeast => 1,
            Self::Southeast => 2,
            Self::South => 3,
            Self::Southwest => 4,
            Self::Northwest => 5,
        }
    }
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
        grid_position.0 = destination;
        if matches!(
            movement_direction,
            ScreenDirection::North | ScreenDirection::South
        ) {
            phase.use_right_diagonal = !phase.use_right_diagonal;
        }

        let world = axial_to_world(destination);
        transform.translation.x = world.x;
        transform.translation.y = world.y;

        return true;
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerAction {
    MoveForward,
    MoveBackward,
    RotateLeft,
    RotateRight,
}

fn pressed_player_action(keyboard: &ButtonInput<KeyCode>) -> Option<PlayerAction> {
    [
        (KeyCode::KeyW, PlayerAction::MoveForward),
        (KeyCode::KeyS, PlayerAction::MoveBackward),
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

pub const fn hex_direction_for_screen_direction(
    direction: ScreenDirection,
    use_right_diagonal: bool,
) -> usize {
    match (direction, use_right_diagonal) {
        (ScreenDirection::Northeast, _) => 5,
        (ScreenDirection::Southeast, _) => 1,
        (ScreenDirection::Southwest, _) => 2,
        (ScreenDirection::Northwest, _) => 4,
        (ScreenDirection::South, true) => 1,
        (ScreenDirection::South, false) => 2,
        (ScreenDirection::North, false) => 4,
        (ScreenDirection::North, true) => 5,
    }
}

pub fn facing_rotation(direction: ScreenDirection) -> Quat {
    let radians = match direction {
        ScreenDirection::North => 0.0,
        ScreenDirection::Northeast => -std::f32::consts::FRAC_PI_3,
        ScreenDirection::Southeast => -2.0 * std::f32::consts::FRAC_PI_3,
        ScreenDirection::South => std::f32::consts::PI,
        ScreenDirection::Southwest => 2.0 * std::f32::consts::FRAC_PI_3,
        ScreenDirection::Northwest => std::f32::consts::FRAC_PI_3,
    };

    Quat::from_rotation_z(radians)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::WorldSeed;

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
                            .filter(|direction| !matches!(direction, 0 | 3))
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
    fn facing_direction_rotates_through_six_facings() {
        assert_eq!(
            ScreenDirection::North.rotate_left(),
            ScreenDirection::Northwest
        );
        assert_eq!(
            ScreenDirection::North.rotate_right(),
            ScreenDirection::Northeast
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
            1 => (ScreenDirection::Southeast, false),
            2 => (ScreenDirection::Southwest, false),
            4 => (ScreenDirection::Northwest, false),
            5 => (ScreenDirection::Northeast, false),
            _ => unreachable!("hex direction is wrapped before this helper"),
        }
    }
}
