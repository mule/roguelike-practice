use bevy::prelude::*;
use std::collections::HashSet;

use crate::{
    actors::{FacingDirection, GridPosition, Player, ScreenDirection},
    map::{HexCoord, Map, TileKind},
    rendering::{RenderedTile, axial_to_world},
};

const VISION_RADIUS: i32 = 7;
const CONE_HALF_ANGLE_COS: f32 = 0.5;

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct VisibilityState {
    visible: HashSet<HexCoord>,
    explored: HashSet<HexCoord>,
}

impl VisibilityState {
    pub fn is_visible(&self, coord: HexCoord) -> bool {
        self.visible.contains(&coord)
    }

    pub fn is_explored(&self, coord: HexCoord) -> bool {
        self.explored.contains(&coord)
    }

    fn set_visible(&mut self, visible: HashSet<HexCoord>) {
        self.explored.extend(visible.iter().copied());
        self.visible = visible;
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibilityDirty(bool);

impl Default for VisibilityDirty {
    fn default() -> Self {
        Self(true)
    }
}

impl VisibilityDirty {
    pub fn mark(&mut self) {
        self.0 = true;
    }
}

#[derive(Resource)]
struct VisibilityMaterials {
    visible_floor: Handle<ColorMaterial>,
    visible_wall: Handle<ColorMaterial>,
    explored_floor: Handle<ColorMaterial>,
    explored_wall: Handle<ColorMaterial>,
    hidden: Handle<ColorMaterial>,
}

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisibilityState>()
            .init_resource::<VisibilityDirty>()
            .add_systems(Startup, setup_visibility_materials)
            .add_systems(Update, refresh_visibility);
    }
}

fn setup_visibility_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(VisibilityMaterials {
        visible_floor: materials.add(ColorMaterial::from_color(Color::srgb(0.16, 0.52, 0.48))),
        visible_wall: materials.add(ColorMaterial::from_color(Color::srgb(0.18, 0.22, 0.30))),
        explored_floor: materials.add(ColorMaterial::from_color(Color::srgb(0.05, 0.16, 0.16))),
        explored_wall: materials.add(ColorMaterial::from_color(Color::srgb(0.04, 0.05, 0.08))),
        hidden: materials.add(ColorMaterial::from_color(Color::srgb(0.005, 0.007, 0.012))),
    });
}

fn refresh_visibility(
    mut dirty: ResMut<VisibilityDirty>,
    map: Res<Map>,
    mut state: ResMut<VisibilityState>,
    player: Single<(&GridPosition, &FacingDirection), With<Player>>,
    visibility_materials: Res<VisibilityMaterials>,
    mut rendered_tiles: Query<(&RenderedTile, &mut MeshMaterial2d<ColorMaterial>)>,
) {
    if !dirty.0 {
        return;
    }

    let (grid_position, facing) = *player;
    let visible = visible_tiles_for(&map, grid_position.0, facing.0, VISION_RADIUS);
    state.set_visible(visible);

    for (rendered_tile, mut material) in &mut rendered_tiles {
        *material = MeshMaterial2d(material_for_tile(
            &map,
            &state,
            &visibility_materials,
            rendered_tile.coord,
        ));
    }

    dirty.0 = false;
}

fn material_for_tile(
    map: &Map,
    state: &VisibilityState,
    materials: &VisibilityMaterials,
    coord: HexCoord,
) -> Handle<ColorMaterial> {
    if state.is_visible(coord) {
        return match map.tile(coord).map(|tile| tile.kind) {
            Some(TileKind::Floor) => materials.visible_floor.clone(),
            Some(TileKind::Wall) => materials.visible_wall.clone(),
            None => materials.hidden.clone(),
        };
    }

    if state.is_explored(coord) {
        return match map.tile(coord).map(|tile| tile.kind) {
            Some(TileKind::Floor) => materials.explored_floor.clone(),
            Some(TileKind::Wall) => materials.explored_wall.clone(),
            None => materials.hidden.clone(),
        };
    }

    materials.hidden.clone()
}

pub fn visible_tiles_for(
    map: &Map,
    origin: HexCoord,
    facing: ScreenDirection,
    radius: i32,
) -> HashSet<HexCoord> {
    map.tiles()
        .map(|(coord, _tile)| coord)
        .filter(|coord| origin.distance(*coord) <= radius)
        .filter(|coord| is_inside_facing_cone(origin, facing, *coord))
        .filter(|coord| has_line_of_sight(map, origin, *coord))
        .collect()
}

fn is_inside_facing_cone(origin: HexCoord, facing: ScreenDirection, target: HexCoord) -> bool {
    if origin == target {
        return true;
    }

    let origin_world = axial_to_world(origin);
    let to_target = (axial_to_world(target) - origin_world).normalize_or_zero();
    let facing_vector = facing_vector(facing);

    to_target.dot(facing_vector) >= CONE_HALF_ANGLE_COS
}

fn facing_vector(facing: ScreenDirection) -> Vec2 {
    match facing {
        ScreenDirection::North => Vec2::Y,
        ScreenDirection::Northeast => hex_step_vector(5),
        ScreenDirection::Southeast => hex_step_vector(1),
        ScreenDirection::South => Vec2::NEG_Y,
        ScreenDirection::Southwest => hex_step_vector(2),
        ScreenDirection::Northwest => hex_step_vector(4),
    }
}

fn hex_step_vector(direction: usize) -> Vec2 {
    (axial_to_world(HexCoord::new(0, 0).neighbor(direction)) - axial_to_world(HexCoord::new(0, 0)))
        .normalize_or_zero()
}

fn has_line_of_sight(map: &Map, origin: HexCoord, target: HexCoord) -> bool {
    for coord in hex_line(origin, target).into_iter().skip(1) {
        if coord == target {
            return true;
        }

        if map.blocks_sight(coord) {
            return false;
        }
    }

    true
}

fn hex_line(origin: HexCoord, target: HexCoord) -> Vec<HexCoord> {
    let distance = origin.distance(target);
    if distance == 0 {
        return vec![origin];
    }

    let origin = CubeCoord::from(origin);
    let target = CubeCoord::from(target);

    (0..=distance)
        .map(|step| {
            let t = step as f32 / distance as f32;
            origin.lerp(target, t).round().into()
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CubeCoord {
    q: f32,
    r: f32,
    s: f32,
}

impl CubeCoord {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            q: self.q + (other.q - self.q) * t,
            r: self.r + (other.r - self.r) * t,
            s: self.s + (other.s - self.s) * t,
        }
    }

    fn round(self) -> Self {
        let mut q = self.q.round();
        let mut r = self.r.round();
        let mut s = self.s.round();

        let q_diff = (q - self.q).abs();
        let r_diff = (r - self.r).abs();
        let s_diff = (s - self.s).abs();

        if q_diff > r_diff && q_diff > s_diff {
            q = -r - s;
        } else if r_diff > s_diff {
            r = -q - s;
        } else {
            s = -q - r;
        }

        Self { q, r, s }
    }
}

impl From<HexCoord> for CubeCoord {
    fn from(coord: HexCoord) -> Self {
        Self {
            q: coord.q as f32,
            r: coord.r as f32,
            s: coord.s() as f32,
        }
    }
}

impl From<CubeCoord> for HexCoord {
    fn from(coord: CubeCoord) -> Self {
        Self::new(coord.q as i32, coord.r as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Tile;

    #[test]
    fn open_area_visibility_uses_a_directional_cone() {
        let map = open_test_map(3, &[]);
        let origin = HexCoord::new(0, 0);
        let visible = visible_tiles_for(&map, origin, ScreenDirection::North, 3);

        assert!(visible.contains(&origin));
        assert!(visible.contains(&HexCoord::new(0, 1)));
        assert!(visible.contains(&HexCoord::new(-1, 1)));
        assert!(!visible.contains(&HexCoord::new(0, -1)));
        assert!(!visible.contains(&HexCoord::new(1, -1)));
    }

    #[test]
    fn rotating_changes_the_visible_tiles() {
        let map = open_test_map(3, &[]);
        let origin = HexCoord::new(0, 0);

        let north = visible_tiles_for(&map, origin, ScreenDirection::North, 3);
        let south = visible_tiles_for(&map, origin, ScreenDirection::South, 3);

        assert_ne!(north, south);
        assert!(north.contains(&HexCoord::new(0, 1)));
        assert!(south.contains(&HexCoord::new(0, -1)));
    }

    #[test]
    fn walls_are_visible_but_block_tiles_behind_them() {
        let wall = HexCoord::new(0, 1);
        let behind_wall = HexCoord::new(0, 2);
        let map = open_test_map(4, &[wall]);
        let visible = visible_tiles_for(&map, HexCoord::new(0, 0), ScreenDirection::North, 4);

        assert!(visible.contains(&wall));
        assert!(!visible.contains(&behind_wall));
    }

    #[test]
    fn explored_tiles_remain_remembered_after_leaving_the_cone() {
        let map = open_test_map(3, &[]);
        let origin = HexCoord::new(0, 0);
        let mut state = VisibilityState::default();

        state.set_visible(visible_tiles_for(&map, origin, ScreenDirection::North, 3));
        assert!(state.is_visible(HexCoord::new(0, 1)));
        assert!(state.is_explored(HexCoord::new(0, 1)));

        state.set_visible(visible_tiles_for(&map, origin, ScreenDirection::South, 3));
        assert!(!state.is_visible(HexCoord::new(0, 1)));
        assert!(state.is_explored(HexCoord::new(0, 1)));
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
