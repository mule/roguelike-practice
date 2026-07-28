use bevy::prelude::*;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSeed(pub u64);

impl Default for WorldSeed {
    fn default() -> Self {
        Self(0x5EED_0002)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSizePreset {
    Small,
    Medium,
    Large,
    Custom,
}

impl MapSizePreset {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Custom => "custom",
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapGenerationConfig {
    pub preset: MapSizePreset,
    pub radius: i32,
}

impl MapGenerationConfig {
    pub const MIN_RADIUS: i32 = 6;
    pub const MAX_RADIUS: i32 = 48;

    pub const fn small() -> Self {
        Self {
            preset: MapSizePreset::Small,
            radius: 8,
        }
    }

    pub const fn medium() -> Self {
        Self {
            preset: MapSizePreset::Medium,
            radius: 12,
        }
    }

    pub const fn large() -> Self {
        Self {
            preset: MapSizePreset::Large,
            radius: 20,
        }
    }

    pub fn custom(radius: i32) -> Option<Self> {
        if !(Self::MIN_RADIUS..=Self::MAX_RADIUS).contains(&radius) {
            return None;
        }

        Some(Self {
            preset: MapSizePreset::Custom,
            radius,
        })
    }

    pub fn named(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "small" => Some(Self::small()),
            "medium" => Some(Self::medium()),
            "large" => Some(Self::large()),
            _ => None,
        }
    }
}

impl Default for MapGenerationConfig {
    fn default() -> Self {
        Self::medium()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub const DIRECTIONS: [Self; 6] = [
        Self::new(1, 0),
        Self::new(1, -1),
        Self::new(0, -1),
        Self::new(-1, 0),
        Self::new(-1, 1),
        Self::new(0, 1),
    ];

    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    pub const fn neighbor(self, direction: usize) -> Self {
        let offset = Self::DIRECTIONS[direction % Self::DIRECTIONS.len()];
        Self::new(self.q + offset.q, self.r + offset.r)
    }

    pub fn neighbors(self) -> [Self; 6] {
        Self::DIRECTIONS.map(|offset| Self::new(self.q + offset.q, self.r + offset.r))
    }

    pub const fn s(self) -> i32 {
        -self.q - self.r
    }

    pub fn distance(self, other: Self) -> i32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = (self.s() - other.s()).abs();

        dq.max(dr).max(ds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Floor,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub kind: TileKind,
}

impl Tile {
    pub const fn floor() -> Self {
        Self {
            kind: TileKind::Floor,
        }
    }

    pub const fn wall() -> Self {
        Self {
            kind: TileKind::Wall,
        }
    }

    pub const fn is_walkable(self) -> bool {
        matches!(self.kind, TileKind::Floor)
    }

    pub const fn blocks_sight(self) -> bool {
        matches!(self.kind, TileKind::Wall)
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub seed: WorldSeed,
    config: MapGenerationConfig,
    radius: i32,
    tiles: HashMap<HexCoord, Tile>,
    player_spawn: HexCoord,
    npc_spawns: Vec<HexCoord>,
}

impl Map {
    pub fn starter(seed: WorldSeed) -> Self {
        Self::starter_with_config(seed, MapGenerationConfig::default())
    }

    pub fn starter_with_config(seed: WorldSeed, config: MapGenerationConfig) -> Self {
        StarterMapGenerator::new(seed, config).generate()
    }

    #[cfg(test)]
    pub(crate) fn from_tiles_for_test(
        radius: i32,
        tiles: impl IntoIterator<Item = (HexCoord, Tile)>,
        player_spawn: HexCoord,
    ) -> Self {
        Self {
            seed: WorldSeed(0),
            config: MapGenerationConfig::default(),
            radius,
            tiles: tiles.into_iter().collect(),
            player_spawn,
            npc_spawns: Vec::new(),
        }
    }

    pub const fn radius(&self) -> i32 {
        self.radius
    }

    pub const fn config(&self) -> MapGenerationConfig {
        self.config
    }

    pub const fn player_spawn(&self) -> HexCoord {
        self.player_spawn
    }

    pub fn npc_spawns(&self) -> &[HexCoord] {
        &self.npc_spawns
    }

    pub fn contains(&self, coord: HexCoord) -> bool {
        self.tiles.contains_key(&coord)
    }

    pub fn tile(&self, coord: HexCoord) -> Option<Tile> {
        self.tiles.get(&coord).copied()
    }

    pub fn tiles(&self) -> impl Iterator<Item = (HexCoord, Tile)> + '_ {
        self.tiles.iter().map(|(coord, tile)| (*coord, *tile))
    }

    pub fn is_walkable(&self, coord: HexCoord) -> bool {
        self.tile(coord).is_some_and(Tile::is_walkable)
    }

    pub fn blocks_sight(&self, coord: HexCoord) -> bool {
        self.tile(coord).is_none_or(Tile::blocks_sight)
    }

    pub fn neighbors(&self, coord: HexCoord) -> impl Iterator<Item = HexCoord> + '_ {
        coord
            .neighbors()
            .into_iter()
            .filter(|neighbor| self.contains(*neighbor))
    }

    pub fn walkable_neighbors(&self, coord: HexCoord) -> impl Iterator<Item = HexCoord> + '_ {
        self.neighbors(coord)
            .filter(|neighbor| self.is_walkable(*neighbor))
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn walkable_count(&self) -> usize {
        self.tiles
            .values()
            .filter(|tile| tile.is_walkable())
            .count()
    }

    pub fn reachable_walkable_count(&self, start: HexCoord) -> usize {
        if !self.is_walkable(start) {
            return 0;
        }

        let mut visited = HashSet::from([start]);
        let mut frontier = VecDeque::from([start]);

        while let Some(coord) = frontier.pop_front() {
            for neighbor in self.walkable_neighbors(coord) {
                if visited.insert(neighbor) {
                    frontier.push_back(neighbor);
                }
            }
        }

        visited.len()
    }
}

struct StarterMapGenerator {
    seed: WorldSeed,
    config: MapGenerationConfig,
    radius: i32,
    tiles: HashMap<HexCoord, Tile>,
    rooms: Vec<HexCoord>,
}

impl StarterMapGenerator {
    fn new(seed: WorldSeed, config: MapGenerationConfig) -> Self {
        let radius = config.radius;
        let mut tiles = HashMap::new();

        for q in -radius..=radius {
            for r in -radius..=radius {
                let coord = HexCoord::new(q, r);
                if HexCoord::new(0, 0).distance(coord) <= radius {
                    tiles.insert(coord, Tile::wall());
                }
            }
        }

        Self {
            seed,
            config,
            radius,
            tiles,
            rooms: Vec::new(),
        }
    }

    fn generate(mut self) -> Map {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed.0);
        let base_rooms = [
            HexCoord::new(0, 0),
            HexCoord::new(5, -4),
            HexCoord::new(5, 1),
            HexCoord::new(0, 5),
            HexCoord::new(-5, 4),
            HexCoord::new(-5, -1),
        ];

        for base in base_rooms {
            let jitter = HexCoord::new(rng.random_range(-1..=1), rng.random_range(-1..=1));
            let center = self.clamp_inside(HexCoord::new(base.q + jitter.q, base.r + jitter.r));
            let room_radius = rng.random_range(2..=3);

            self.carve_disk(center, room_radius);
            self.rooms.push(center);
        }

        for index in 1..self.rooms.len() {
            self.carve_corridor(self.rooms[index - 1], self.rooms[index]);
        }
        self.carve_corridor(*self.rooms.last().expect("starter rooms"), self.rooms[0]);

        let player_spawn = self.rooms[0];
        let npc_spawns = self
            .rooms
            .iter()
            .copied()
            .skip(1)
            .filter(|coord| self.tiles.get(coord).is_some_and(|tile| tile.is_walkable()))
            .take(3)
            .collect();

        Map {
            seed: self.seed,
            config: self.config,
            radius: self.radius,
            tiles: self.tiles,
            player_spawn,
            npc_spawns,
        }
    }

    fn clamp_inside(&self, coord: HexCoord) -> HexCoord {
        if HexCoord::new(0, 0).distance(coord) < self.radius {
            return coord;
        }

        self.step_toward(coord, HexCoord::new(0, 0))
    }

    fn carve_disk(&mut self, center: HexCoord, radius: i32) {
        for q in center.q - radius..=center.q + radius {
            for r in center.r - radius..=center.r + radius {
                let coord = HexCoord::new(q, r);
                if center.distance(coord) <= radius {
                    self.carve_floor(coord);
                }
            }
        }
    }

    fn carve_corridor(&mut self, from: HexCoord, to: HexCoord) {
        let mut current = from;
        self.carve_floor(current);

        while current != to {
            current = self.step_toward(current, to);
            self.carve_floor(current);
        }
    }

    fn step_toward(&self, from: HexCoord, to: HexCoord) -> HexCoord {
        from.neighbors()
            .into_iter()
            .filter(|coord| self.tiles.contains_key(coord))
            .min_by_key(|coord| coord.distance(to))
            .unwrap_or(from)
    }

    fn carve_floor(&mut self, coord: HexCoord) {
        if let Some(tile) = self.tiles.get_mut(&coord) {
            *tile = Tile::floor();
        }
    }
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_starter_map);
    }
}

fn setup_starter_map(
    mut commands: Commands,
    seed: Res<WorldSeed>,
    config: Res<MapGenerationConfig>,
) {
    commands.insert_resource(Map::starter_with_config(*seed, *config));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seed_is_stable() {
        assert_eq!(WorldSeed::default(), WorldSeed(0x5EED_0002));
    }

    #[test]
    fn map_generation_config_has_named_presets() {
        assert_eq!(
            MapGenerationConfig::default(),
            MapGenerationConfig::medium()
        );
        assert_eq!(
            MapGenerationConfig::named("small"),
            Some(MapGenerationConfig::small())
        );
        assert_eq!(
            MapGenerationConfig::named("MEDIUM"),
            Some(MapGenerationConfig::medium())
        );
        assert_eq!(
            MapGenerationConfig::named("large"),
            Some(MapGenerationConfig::large())
        );
        assert_eq!(MapGenerationConfig::named("huge"), None);
    }

    #[test]
    fn custom_map_generation_config_validates_radius_bounds() {
        assert_eq!(
            MapGenerationConfig::custom(18),
            Some(MapGenerationConfig {
                preset: MapSizePreset::Custom,
                radius: 18,
            })
        );
        assert_eq!(
            MapGenerationConfig::custom(MapGenerationConfig::MIN_RADIUS - 1),
            None
        );
        assert_eq!(
            MapGenerationConfig::custom(MapGenerationConfig::MAX_RADIUS + 1),
            None
        );
    }

    #[test]
    fn starter_map_keeps_seed_and_player_spawn_snapshot() {
        let map = Map::starter(WorldSeed(42));

        assert_eq!(map.seed, WorldSeed(42));
        assert_eq!(map.config(), MapGenerationConfig::default());
        assert_eq!(map.player_spawn(), HexCoord::new(-1, 1));
    }

    #[test]
    fn tile_passability_and_opacity_are_derived_from_kind() {
        assert!(Tile::floor().is_walkable());
        assert!(!Tile::floor().blocks_sight());

        assert!(!Tile::wall().is_walkable());
        assert!(Tile::wall().blocks_sight());
    }

    #[test]
    fn hex_neighbors_and_distance_follow_axial_topology() {
        let origin = HexCoord::new(0, 0);

        assert_eq!(
            origin.neighbors(),
            [
                HexCoord::new(1, 0),
                HexCoord::new(1, -1),
                HexCoord::new(0, -1),
                HexCoord::new(-1, 0),
                HexCoord::new(-1, 1),
                HexCoord::new(0, 1),
            ]
        );
        assert_eq!(origin.distance(HexCoord::new(3, -2)), 3);
        assert_eq!(HexCoord::new(-2, 5).distance(HexCoord::new(2, 1)), 4);
    }

    #[test]
    fn starter_map_is_deterministic_for_a_seed() {
        assert_eq!(Map::starter(WorldSeed(7)), Map::starter(WorldSeed(7)));
        assert_ne!(Map::starter(WorldSeed(7)), Map::starter(WorldSeed(8)));
    }

    #[test]
    fn starter_map_is_deterministic_for_seed_and_config() {
        let config = MapGenerationConfig::large();

        assert_eq!(
            Map::starter_with_config(WorldSeed(7), config),
            Map::starter_with_config(WorldSeed(7), config)
        );
        assert_ne!(
            Map::starter_with_config(WorldSeed(7), MapGenerationConfig::small()),
            Map::starter_with_config(WorldSeed(7), MapGenerationConfig::large())
        );
    }

    #[test]
    fn starter_map_has_bounded_hex_tiles_and_queries() {
        let map = Map::starter(WorldSeed(42));

        assert_eq!(map.radius(), 12);
        assert_eq!(map.tile_count(), 469);
        assert!(map.contains(HexCoord::new(0, 0)));
        assert!(!map.contains(HexCoord::new(13, 0)));
        assert!(map.is_walkable(map.player_spawn()));
        assert!(!map.is_walkable(HexCoord::new(12, 0)));
        assert!(map.blocks_sight(HexCoord::new(12, 0)));
        assert!(map.blocks_sight(HexCoord::new(13, 0)));
    }

    #[test]
    fn starter_map_uses_configured_radius() {
        let small = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::small());
        let medium = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::medium());
        let large = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::large());

        assert_eq!(small.radius(), 8);
        assert_eq!(medium.radius(), 12);
        assert_eq!(large.radius(), 20);
        assert_eq!(small.tile_count(), 217);
        assert_eq!(medium.tile_count(), 469);
        assert_eq!(large.tile_count(), 1261);
    }

    #[test]
    fn starter_map_guarantees_connected_walkable_space_and_npc_spawns() {
        let map = Map::starter(WorldSeed(42));

        assert!(!map.npc_spawns().is_empty());
        assert_eq!(
            map.reachable_walkable_count(map.player_spawn()),
            map.walkable_count()
        );

        for spawn in map.npc_spawns() {
            assert!(map.is_walkable(*spawn));
        }
    }

    #[test]
    fn map_neighbors_are_limited_to_known_tiles() {
        let map = Map::starter(WorldSeed(42));

        assert_eq!(map.neighbors(HexCoord::new(0, 0)).count(), 6);
        assert!(map.neighbors(HexCoord::new(12, 0)).count() < 6);
    }
}
