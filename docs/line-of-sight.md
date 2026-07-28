# Line of Sight

This document describes the current line-of-sight and fog-of-war implementation
for the hex-grid roguelike prototype.

The implementation lives in `src/visibility.rs`. Player movement and rotation in
`src/actors.rs` mark visibility as dirty whenever the player's view may have
changed.

## Goals

- Show only tiles inside a forward-facing vision cone.
- Let the player's facing direction change visibility even when the player does
  not move.
- Let walls be seen, but prevent seeing tiles behind them.
- Remember previously seen tiles as explored fog-of-war.
- Keep the core algorithm testable without running the Bevy app.

## Coordinate Model

The map uses axial hex coordinates:

- `q`: axial column.
- `r`: axial row.
- `s`: derived cube coordinate, computed as `-q - r`.

Axial coordinates are compact for storing map tiles, while cube coordinates are
useful for line interpolation. The LOS code converts axial coordinates to cube
coordinates only while tracing a sight line.

The current hex directions come from `HexCoord::DIRECTIONS`:

```text
0: east
1: southeast
2: southwest
3: west
4: northwest
5: northeast
```

The screen-facing directions are gameplay directions, not raw hex directions.
Because the map is pointy-top, vertical movement alternates between diagonal hex
steps. LOS uses the same visual facing model as the player triangle:

- `North`
- `Northeast`
- `Southeast`
- `South`
- `Southwest`
- `Northwest`

## Visibility State

`VisibilityState` stores two sets:

- `visible`: tiles visible right now.
- `explored`: tiles that have ever been visible.

Every recompute replaces `visible`, then merges the new visible tiles into
`explored`.

```text
new visible tiles -> explored union
new visible tiles -> current visible
```

This gives the classic fog-of-war behavior: when the player turns away, the old
tiles stop being visible but remain remembered.

## Dirty Recompute

Visibility is recomputed only when needed.

`VisibilityDirty` starts as `true`, so the first update computes initial LOS.
After that:

- rotating left or right marks visibility dirty;
- successful forward or backward movement marks visibility dirty;
- blocked movement does not mark visibility dirty.

That means a failed bump into a wall does not waste work or change the fog state.
A rotation always recomputes because the cone direction changes even if the
origin tile stays the same.

## Algorithm Overview

The LOS algorithm is:

1. Enumerate every map tile.
2. Keep only tiles within `VISION_RADIUS`.
3. Keep only tiles inside the player's facing cone.
4. Trace a hex line from the player to each candidate tile.
5. Reject candidates whose line is blocked by an opaque tile.
6. Store the final set as `visible` and merge it into `explored`.
7. Update each rendered tile's material from its visibility state.

In pseudocode:

```text
visible = {}

for tile in map.tiles:
    if distance(player, tile) > vision_radius:
        continue

    if tile is outside facing cone:
        continue

    if line from player to tile is blocked:
        continue

    visible.insert(tile)

explored.extend(visible)
```

## Facing Cone

The cone is computed in world/screen space, because the player's facing is a
visual direction.

For each candidate tile:

1. Convert the player coordinate and candidate coordinate to world positions.
2. Build a normalized vector from player to candidate.
3. Build a normalized facing vector from the player's `ScreenDirection`.
4. Use the dot product to check whether the angle is inside the cone.

The current constant is:

```rust
const CONE_HALF_ANGLE_COS: f32 = 0.5;
```

`0.5` is the cosine of 60 degrees, so this creates a 120-degree total cone.
That is intentionally broad for early gameplay: it gives a clear forward wedge
while still making facing matter.

The origin tile is always visible, regardless of cone direction.

## Hex Line Tracing

After cone filtering, the algorithm checks whether there is an unobstructed line
from the player to the candidate tile.

The implementation uses cube-coordinate interpolation:

1. Convert origin and target axial coordinates to cube coordinates.
2. Measure hex distance between origin and target.
3. Interpolate between the cube endpoints once per hex step.
4. Round each interpolated cube coordinate back to the nearest valid hex.
5. Convert the rounded cube coordinate back to axial.

This is the hex-grid version of drawing a straight line across square grid
cells.

The cube rounding step preserves the required invariant:

```text
q + r + s = 0
```

Floating-point rounding can break that invariant by a small amount. The algorithm
rounds all three components, finds the component with the largest rounding
error, and recalculates that component from the other two.

## Wall Blocking

Walls use `Map::blocks_sight`.

While tracing a line:

- the origin tile is skipped;
- intermediate opaque tiles block sight;
- the target tile is visible even if it is a wall.

This means a wall can be seen, but tiles behind that wall are hidden.

Example:

```text
P . W ? ?
```

If `P` is the player and `W` is a wall, the wall tile is visible. The tiles
behind it are not visible from that ray.

## Rendering

The visibility system does not spawn or despawn map tiles. Rendering creates
tiles once and attaches `RenderedTile { coord }` to each tile entity.

When visibility changes, the system updates each rendered tile's material:

- visible floor;
- visible wall;
- explored floor;
- explored wall;
- hidden.

This keeps rendering simple and avoids replacing tile entities every time the
player moves.

## Current Tuning Values

These values are intentionally early-prototype constants:

- `VISION_RADIUS`: `7`
- cone half angle: `60` degrees
- cone total angle: `120` degrees

Good future experiments:

- different LOS radius per actor;
- narrower or wider cones;
- peripheral dim vision;
- circular LOS for monsters;
- light sources that add extra visible tiles;
- remembering actor last-known positions separately from tile exploration.

## Test Coverage

The current tests cover:

- open-area directional cone membership;
- rotation producing a different visible set;
- walls being visible while blocking tiles behind them;
- explored tiles staying remembered after leaving the cone.

These tests focus on gameplay rules rather than specific colors, so the visual
treatment can change without weakening LOS behavior.
