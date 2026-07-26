use bevy::prelude::*;

use crate::map::HexCoord;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition(pub HexCoord);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Npc;

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, _app: &mut App) {}
}
