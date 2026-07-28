# roguelike-practice

Practice project for building a Rust + Bevy roguelike with a hex-grid map,
line-of-sight mechanics, and simple NPC movement.

The first milestone is a small technical foundation rather than a complete
game: launch a desktop app, keep game systems in focused modules, and make the
map/visibility/actor logic easy to test as it grows.

## Requirements

- Rust 1.95 or newer

## Run

```sh
cargo run
```

Run with a reproducible seed:

```sh
ROGUELIKE_SEED=123 cargo run
cargo run -- --seed 123
```

Run with a map size preset or custom radius:

```sh
ROGUELIKE_MAP_SIZE=large cargo run
cargo run -- --map-size small
cargo run -- --map-radius 18
```

## Test

```sh
cargo test
```

## Docs

- [Line of sight](docs/line-of-sight.md) explains the current facing-cone LOS
  and fog-of-war algorithm.

## Controls

- `W`: move forward in the current facing direction
- `S`: move backward without changing facing
- `A`: side-step left without changing facing
- `D`: side-step right without changing facing
- `Q`: rotate facing left
- `E`: rotate facing right

## Debug Controls

- `F1`: toggle debug overlay
- `F2`: toggle reveal-all map/NPC visibility
- `F3`: pause or resume NPC turns
- `Space`: step one NPC turn while NPC turns are paused

## Current Structure

- `src/main.rs` launches the Bevy app.
- `src/lib.rs` wires the game plugins together.
- `src/map.rs` owns early map data, tile concepts, and deterministic seed state.
- `src/rendering.rs` owns camera and future map rendering setup.
- `src/actors.rs` owns player/NPC marker components and grid positions.
- `src/visibility.rs` owns line-of-sight and fog-of-war systems.
- `src/ai.rs` owns NPC spawning, turn stepping, facing, and basic movement behavior.
