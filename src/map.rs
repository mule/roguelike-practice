use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSeed(pub u64);

impl Default for WorldSeed {
    fn default() -> Self {
        Self(0x5EED_0002)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
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

#[derive(Resource, Debug, Clone)]
pub struct Map {
    pub seed: WorldSeed,
    pub origin: HexCoord,
}

impl Map {
    pub const fn starter(seed: WorldSeed) -> Self {
        Self {
            seed,
            origin: HexCoord::new(0, 0),
        }
    }
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_starter_map);
    }
}

fn setup_starter_map(mut commands: Commands, seed: Res<WorldSeed>) {
    commands.insert_resource(Map::starter(*seed));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seed_is_stable() {
        assert_eq!(WorldSeed::default(), WorldSeed(0x5EED_0002));
    }

    #[test]
    fn starter_map_keeps_seed_and_origin() {
        let map = Map::starter(WorldSeed(42));

        assert_eq!(map.seed, WorldSeed(42));
        assert_eq!(map.origin, HexCoord::new(0, 0));
    }

    #[test]
    fn tile_passability_and_opacity_are_derived_from_kind() {
        assert!(Tile::floor().is_walkable());
        assert!(!Tile::floor().blocks_sight());

        assert!(!Tile::wall().is_walkable());
        assert!(Tile::wall().blocks_sight());
    }
}
