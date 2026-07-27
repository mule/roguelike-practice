use bevy::prelude::*;

use crate::map::{HexCoord, Map, TileKind};

const HEX_RADIUS: f32 = 24.0;
const HEX_MESH_RADIUS: f32 = 23.5;
const MAP_Z: f32 = 0.0;
const CAMERA_Z: f32 = 1_000.0;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedTile {
    pub coord: HexCoord,
}

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(PostStartup, spawn_map_tiles);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, CAMERA_Z),
        Projection::from(OrthographicProjection {
            scale: 1.35,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn spawn_map_tiles(
    mut commands: Commands,
    map: Res<Map>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let hex_mesh = meshes.add(RegularPolygon::new(HEX_MESH_RADIUS, 6));
    let floor_material = materials.add(ColorMaterial::from_color(Color::srgb(0.12, 0.35, 0.34)));
    let wall_material = materials.add(ColorMaterial::from_color(Color::srgb(0.08, 0.10, 0.14)));

    for (coord, tile) in map.tiles() {
        let world = axial_to_world(coord);
        let material = match tile.kind {
            TileKind::Floor => floor_material.clone(),
            TileKind::Wall => wall_material.clone(),
        };

        commands.spawn((
            RenderedTile { coord },
            Mesh2d(hex_mesh.clone()),
            MeshMaterial2d(material),
            Transform::from_xyz(world.x, world.y, MAP_Z)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_6)),
        ));
    }
}

pub fn axial_to_world(coord: HexCoord) -> Vec2 {
    let q = coord.q as f32;
    let r = coord.r as f32;
    let x = HEX_RADIUS * 3.0_f32.sqrt() * (q + r / 2.0);
    let y = HEX_RADIUS * 1.5 * r;

    Vec2::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axial_to_world_places_neighbors_one_hex_step_away() {
        let origin = axial_to_world(HexCoord::new(0, 0));
        let east = axial_to_world(HexCoord::new(1, 0));
        let southeast = axial_to_world(HexCoord::new(0, 1));

        assert_eq!(origin, Vec2::ZERO);
        assert!((east.distance(origin) - HEX_RADIUS * 3.0_f32.sqrt()).abs() < f32::EPSILON);
        assert!((southeast.distance(origin) - HEX_RADIUS * 3.0_f32.sqrt()).abs() < 0.001);
    }
}
