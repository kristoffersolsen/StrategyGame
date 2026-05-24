# Architecture

This document describes the high-level structure of the codebase:
what the crates are, what they own, and how they depend on each other.
It is updated at each stage gate as new structure is added.

For *why* the architecture is this way, see `docs/design/overview.md`.
For *when* decisions were made, see `docs/adr/`.

**Current stage:** Stage 1 complete.

---

## Workspace layout
crates/ \
coords/     — wrap-aware hexagonal coordinate math (no dependencies) \
worldgen/   — procedural world generation pipeline (depends on coords) \
game/       — Bevy application and runtime (depends on coords, worldgen) \
tools/ \
mapdump/    — CLI tool for dumping generation output to PNG (standalone) 

## Crate responsibilities

### `coords`

Pure coordinate math. No external dependencies, no Bevy, no allocation
beyond `range` queries. Everything spatial in the project is built on
this crate.

Public API entry points: [`Coord`], [`MapBounds`], [`Direction`].

### `worldgen`

The generation pipeline: tectonics, climate, hydrology, biomes,
resources, starting positions. Takes a seed and [`WorldParams`],
produces [`WorldData`]. Runs standalone (used by `mapdump`) and is
loaded by `game` at world creation time.

Currently contains placeholder stubs for `WorldData` and `WorldParams`;
full implementation begins in Stage 2.

### `game`

The Bevy application. Owns the ECS world, plugin registration, systems,
and all runtime logic. Reads `WorldData` from `worldgen` to spawn
entities at game start.

Currently a stub; implementation begins in Stage 4.

### `mapdump` (tool)

A CLI binary for running the generation pipeline headlessly and dumping
intermediate and final fields to PNG. Used for parameter tuning without
launching the full game. Implementation begins in Stage 2.

---

## Dependency graph
game  ──────────────────────────────► coords  
│ &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;  ▲  
└──► worldgen ───────────────────────► coords  
mapdump ──► worldgen ──► coords

No cycles. `coords` has zero dependencies. `worldgen` does not depend
on `game`. `game` does not depend on `mapdump`.

---

## Key architectural decisions

- **Explicit `MapBounds` parameter** — coordinate operations take
  `MapBounds` as an argument rather than `Coord` carrying a reference
  to its map. Avoids lifetimes on `Coord` and keeps the type `Copy`.

- **`Coord` is `Copy`** — two `i32` fields, no heap allocation, no
  `Drop`. Passes freely by value throughout the codebase.

- **`coords` has no runtime dependencies** — keeps compile times low
  and the crate usable outside the game (tests, tools, future server).

- **`worldgen` has no Bevy dependency** — generation runs headlessly.
  `mapdump` exists specifically to exploit this.

For the full decision history, see `docs/adr/`.