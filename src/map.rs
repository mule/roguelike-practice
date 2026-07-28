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

    fn generation_spec(self) -> MapGenerationSpec {
        let room_count = (self.radius / 2).clamp(4, 14) as usize;
        let max_room_radius = if self.radius >= 18 {
            4
        } else if self.radius >= 10 {
            3
        } else {
            2
        };
        let npc_spawn_count = (room_count.saturating_sub(1)).min((self.radius / 4).max(2) as usize);
        let forest_patch_count = (self.radius / 4).max(2) as usize;
        let hill_patch_count = (self.radius / 6).max(1) as usize;

        MapGenerationSpec {
            room_count,
            min_room_radius: 2,
            max_room_radius,
            npc_spawn_count,
            min_room_spacing: max_room_radius + 2,
            terrain_patch_radius: (self.radius / 8).clamp(1, 3),
            forest_patch_count,
            hill_patch_count,
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
    Ground,
    Floor,
    Wall,
    Forest,
    Hill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub kind: TileKind,
}

impl Tile {
    pub const fn ground() -> Self {
        Self {
            kind: TileKind::Ground,
        }
    }

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

    pub const fn forest() -> Self {
        Self {
            kind: TileKind::Forest,
        }
    }

    pub const fn hill() -> Self {
        Self {
            kind: TileKind::Hill,
        }
    }

    pub const fn is_walkable(self) -> bool {
        !matches!(self.kind, TileKind::Wall)
    }

    pub const fn blocks_sight(self) -> bool {
        matches!(
            self.kind,
            TileKind::Wall | TileKind::Forest | TileKind::Hill
        )
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
    spec: MapGenerationSpec,
    radius: i32,
    tiles: HashMap<HexCoord, Tile>,
    rooms: Vec<HexCoord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MapGenerationSpec {
    room_count: usize,
    min_room_radius: i32,
    max_room_radius: i32,
    npc_spawn_count: usize,
    min_room_spacing: i32,
    terrain_patch_radius: i32,
    forest_patch_count: usize,
    hill_patch_count: usize,
}

impl StarterMapGenerator {
    fn new(seed: WorldSeed, config: MapGenerationConfig) -> Self {
        let radius = config.radius;
        let spec = config.generation_spec();
        let mut tiles = HashMap::new();

        for q in -radius..=radius {
            for r in -radius..=radius {
                let coord = HexCoord::new(q, r);
                if HexCoord::new(0, 0).distance(coord) <= radius {
                    tiles.insert(coord, Tile::ground());
                }
            }
        }

        Self {
            seed,
            config,
            spec,
            radius,
            tiles,
            rooms: Vec::new(),
        }
    }

    fn generate(mut self) -> Map {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed.0);
        let room_centers = self.generate_room_centers(&mut rng);

        self.scatter_terrain_patches(&mut rng, TileKind::Forest, self.spec.forest_patch_count);
        self.scatter_terrain_patches(&mut rng, TileKind::Hill, self.spec.hill_patch_count);

        for center in room_centers {
            let room_radius =
                rng.random_range(self.spec.min_room_radius..=self.spec.max_room_radius);

            self.carve_building(center, room_radius);
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
            .take(self.spec.npc_spawn_count)
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

    fn generate_room_centers(&self, rng: &mut ChaCha8Rng) -> Vec<HexCoord> {
        let mut centers = Vec::with_capacity(self.spec.room_count);
        let inner_radius = (self.radius - self.spec.max_room_radius - 1).max(1);
        let first_room = self.clamp_inside(HexCoord::new(
            rng.random_range(-1..=1),
            rng.random_range(-1..=1),
        ));
        centers.push(first_room);

        let mut attempts = 0;
        while centers.len() < self.spec.room_count && attempts < self.spec.room_count * 80 {
            attempts += 1;
            let candidate = random_coord_in_radius(rng, inner_radius);

            if centers
                .iter()
                .all(|center| center.distance(candidate) >= self.spec.min_room_spacing)
            {
                centers.push(candidate);
            }
        }

        let mut fallback_spacing = self.spec.min_room_spacing.saturating_sub(1);
        while centers.len() < self.spec.room_count && fallback_spacing > 0 {
            for candidate in hex_disk(inner_radius) {
                if centers.len() >= self.spec.room_count {
                    break;
                }

                if centers
                    .iter()
                    .all(|center| center.distance(candidate) >= fallback_spacing)
                {
                    centers.push(candidate);
                }
            }

            fallback_spacing -= 1;
        }

        centers
    }

    fn scatter_terrain_patches(
        &mut self,
        rng: &mut ChaCha8Rng,
        kind: TileKind,
        patch_count: usize,
    ) {
        let patch_center_radius = (self.radius - self.spec.terrain_patch_radius - 1).max(1);

        for _ in 0..patch_count {
            let center = random_coord_in_radius(rng, patch_center_radius);
            let radius = rng.random_range(1..=self.spec.terrain_patch_radius);
            self.paint_disk(center, radius, kind);
        }
    }

    fn clamp_inside(&self, coord: HexCoord) -> HexCoord {
        if HexCoord::new(0, 0).distance(coord) < self.radius {
            return coord;
        }

        self.step_toward(coord, HexCoord::new(0, 0))
    }

    fn carve_building(&mut self, center: HexCoord, radius: i32) {
        for q in center.q - radius..=center.q + radius {
            for r in center.r - radius..=center.r + radius {
                let coord = HexCoord::new(q, r);
                let distance = center.distance(coord);

                if distance < radius {
                    self.carve_floor(coord);
                } else if distance == radius {
                    self.carve_wall(coord);
                }
            }
        }
    }

    fn paint_disk(&mut self, center: HexCoord, radius: i32, kind: TileKind) {
        for q in center.q - radius..=center.q + radius {
            for r in center.r - radius..=center.r + radius {
                let coord = HexCoord::new(q, r);
                if center.distance(coord) <= radius {
                    self.paint_tile(coord, kind);
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
        self.paint_tile(coord, TileKind::Floor);
    }

    fn carve_wall(&mut self, coord: HexCoord) {
        self.paint_tile(coord, TileKind::Wall);
    }

    fn paint_tile(&mut self, coord: HexCoord, kind: TileKind) {
        if let Some(tile) = self.tiles.get_mut(&coord) {
            tile.kind = kind;
        }
    }
}

fn random_coord_in_radius(rng: &mut ChaCha8Rng, radius: i32) -> HexCoord {
    loop {
        let coord = HexCoord::new(
            rng.random_range(-radius..=radius),
            rng.random_range(-radius..=radius),
        );
        if HexCoord::new(0, 0).distance(coord) <= radius {
            return coord;
        }
    }
}

fn hex_disk(radius: i32) -> impl Iterator<Item = HexCoord> {
    (-radius..=radius).flat_map(move |q| {
        (-radius..=radius).filter_map(move |r| {
            let coord = HexCoord::new(q, r);
            (HexCoord::new(0, 0).distance(coord) <= radius).then_some(coord)
        })
    })
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
    fn starter_map_keeps_seed_config_and_walkable_player_spawn() {
        let map = Map::starter(WorldSeed(42));

        assert_eq!(map.seed, WorldSeed(42));
        assert_eq!(map.config(), MapGenerationConfig::default());
        assert!(map.is_walkable(map.player_spawn()));
    }

    #[test]
    fn tile_passability_and_opacity_are_derived_from_kind() {
        assert!(Tile::ground().is_walkable());
        assert!(!Tile::ground().blocks_sight());

        assert!(Tile::floor().is_walkable());
        assert!(!Tile::floor().blocks_sight());

        assert!(!Tile::wall().is_walkable());
        assert!(Tile::wall().blocks_sight());

        assert!(Tile::forest().is_walkable());
        assert!(Tile::forest().blocks_sight());

        assert!(Tile::hill().is_walkable());
        assert!(Tile::hill().blocks_sight());
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
        assert!(map.is_walkable(HexCoord::new(12, 0)));
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
    fn starter_map_scales_walkable_space_and_npc_spawns() {
        let small = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::small());
        let medium = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::medium());
        let large = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::large());

        assert!(small.walkable_count() < medium.walkable_count());
        assert!(medium.walkable_count() < large.walkable_count());
        assert_eq!(small.npc_spawns().len(), 2);
        assert_eq!(medium.npc_spawns().len(), 3);
        assert_eq!(large.npc_spawns().len(), 5);
    }

    #[test]
    fn starter_map_contains_outdoors_buildings_and_natural_terrain() {
        let map = Map::starter_with_config(WorldSeed(42), MapGenerationConfig::medium());

        assert!(tile_kind_count(&map, TileKind::Ground) > tile_kind_count(&map, TileKind::Floor));
        assert!(tile_kind_count(&map, TileKind::Floor) > 0);
        assert!(tile_kind_count(&map, TileKind::Wall) > 0);
        assert!(tile_kind_count(&map, TileKind::Forest) > 0);
        assert!(tile_kind_count(&map, TileKind::Hill) > 0);
    }

    #[test]
    fn starter_map_presets_guarantee_connected_walkable_space_and_npc_spawns() {
        for config in [
            MapGenerationConfig::small(),
            MapGenerationConfig::medium(),
            MapGenerationConfig::large(),
            MapGenerationConfig::custom(18).expect("valid custom radius"),
        ] {
            let map = Map::starter_with_config(WorldSeed(42), config);
            let mut unique_spawns = HashSet::new();

            assert!(map.is_walkable(map.player_spawn()));
            assert!(!map.npc_spawns().is_empty());
            assert_eq!(
                map.reachable_walkable_count(map.player_spawn()),
                map.walkable_count()
            );

            for spawn in map.npc_spawns() {
                assert!(map.is_walkable(*spawn));
                assert_ne!(*spawn, map.player_spawn());
                assert!(unique_spawns.insert(*spawn));
            }
        }
    }

    #[test]
    fn map_neighbors_are_limited_to_known_tiles() {
        let map = Map::starter(WorldSeed(42));

        assert_eq!(map.neighbors(HexCoord::new(0, 0)).count(), 6);
        assert!(map.neighbors(HexCoord::new(12, 0)).count() < 6);
    }

    fn tile_kind_count(map: &Map, kind: TileKind) -> usize {
        map.tiles()
            .filter(|(_coord, tile)| tile.kind == kind)
            .count()
    }
}
