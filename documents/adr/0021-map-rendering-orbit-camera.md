# ADR-0021: Map Rendering Orbit Camera

**Status:** Accepted  
**Date:** 2026-05-28  
**Deciders:** Kristoffer

## Context

P3 moves the map renderer to a 3D scene. The temporary P3.1 camera
proved the perspective path, but a fixed camera is not sufficient for
terrain inspection or later gameplay. The camera must support panning,
zooming, tilt, and rotation while keeping the map readable.

The plan considered the community `bevy_panorbit_camera` crate, but the
game will need custom behavior: terrain-following pivots, bounded and
eventually wrap-aware panning, tile selection integration, and strategy
camera tuning.

## Decision

Implement a small in-house orbit camera for the `Terrain3d` renderer.
The camera state is stored as a `CameraOrbit` resource:

- `pivot`: world-space target point on the map.
- `distance`: camera distance from the pivot.
- `yaw`: rotation around the Z axis.
- `pitch`: elevation angle above the ground plane.

The camera transform is derived from those values every frame. Panning
moves the pivot, scrolling changes distance, and right-mouse drag changes
yaw/pitch. Pitch and distance are clamped to prevent underground views,
flipped views, and unusable zoom levels.

P1/P2 keep the existing orthographic 2D pan/zoom controls. The orbit
system is inert unless `CameraOrbit` exists, and the old 2D pan system
returns early for perspective cameras.

## Controls

| Input | Action |
|---|---|
| Middle-mouse drag | Pan pivot in the camera ground plane |
| Right-mouse drag | Orbit yaw and pitch |
| Mouse wheel | Zoom distance |
| WASD / arrow keys | Pan pivot relative to camera yaw |

## Consequences

**Positive:**

- The 3D renderer has the camera model needed for P3 and Stage 4.
- Panning and keyboard movement are camera-relative instead of world-axis
  fixed.
- The implementation remains local to `rendering/camera.rs` and can be
  extended for terrain-following and east-west wrap behavior.

**Negative:**

- We own camera tuning and future edge cases instead of delegating them
  to a crate.
- Mouse panning sensitivity may need visual tuning after prism geometry
  and model layers are final.

## Validation

Automated validation:

| Check | Result |
|---|---|
| `cargo check --workspace` | Pass |
| `cargo check -p game` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo clippy -p game --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |

Interactive validation still needs a local run:

| Metric | Result |
|---|---|
| Right-drag yaw and pitch | Not recorded |
| Middle-drag pan | Not recorded |
| Mouse-wheel zoom | Not recorded |
| WASD/arrow pan | Not recorded |
| Pitch clamp prevents underground/flipped views | Not recorded |
| Pivot remains within map bounds | Not recorded |
