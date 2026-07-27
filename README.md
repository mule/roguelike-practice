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

## Test

```sh
cargo test
```

## Controls

- `W`: move forward in the current facing direction
- `S`: move backward without changing facing
- `Q`: rotate facing left
- `E`: rotate facing right

## Current Structure

- `src/main.rs` launches the Bevy app.
- `src/lib.rs` wires the game plugins together.
- `src/map.rs` owns early map data, tile concepts, and deterministic seed state.
- `src/rendering.rs` owns camera and future map rendering setup.
- `src/actors.rs` owns player/NPC marker components and grid positions.
- `src/visibility.rs` is reserved for line-of-sight and fog-of-war systems.
- `src/ai.rs` is reserved for NPC turn and movement behavior.
