# ADR-0022: Content Registry Model

**Status:** Accepted  
**Date:** 2026-05-29  
**Deciders:** Kristoffer

## Context

The worldgen pipeline used closed Rust enums (`Biome`, `ResourceId`) as
the runtime identity for content kinds. Closed enums require recompilation
to add new variants, which makes the content set un-moddable. The decision
was to open content kinds before any runtime system was built on them, when
the conversion is cheapest.

Two disciplines are required by the hot-path architecture: string IDs are
resolved to handles *once at load boundaries*, and all per-tile properties
are stored as *dense handle-indexed arrays* rather than read through
descriptor structs behind pointers.

This mirrors the `NetworkId`-vs-`Entity` split already present in the
multiplayer design: a stable authoritative identity that crosses saves and
the network, and a derived runtime index that is never persisted or synced.

## Decision

Introduce a new `content` crate below `worldgen` and `game` that owns the
registry types:

- **`Handle(u32)`** — a dense runtime index. Cheap to copy, compare,
  and use as an array index. Never serialised.
- **`Registry`** — a sorted `Vec<String>` mapping string IDs to handles.
  Built by sorting all registered IDs alphabetically and assigning
  consecutive dense indices. The same set of IDs always produces the same
  handle assignments, preserving generation determinism.

Content kinds converted to registry handles in this stage:
- **`Biome`** → `BiomeHandle = content::Handle` with `BiomeRegistry`
- **`ResourceId`** → `ResourceHandle = content::Handle` with `ResourceRegistry`

Each registry carries dense property arrays indexed by `handle.index()`:
- `BiomeRegistry::is_forest: Vec<bool>` — replaces `Biome::is_forest()`
- `ResourceRegistry::category: Vec<ResourceCategory>` — per-handle category

### Hot-path disciplines (requirements, not optimizations)

1. **Resolve once at load boundaries.** String IDs are resolved in
   `BiomeTable::load` and `ResourceConfig::load`. After that, all
   simulation and per-tile code uses handles exclusively.

2. **Dense property arrays for hot-path reads.** `is_forest_biome(handle)`
   is `self.is_forest[handle.index()]` — an array read, not a struct field
   behind a pointer.

### What stays as enums

The following types are **intentionally retained as enums** because they
are generation mechanisms, not open content kinds:

- **`MineralCharacter`** — a geological generation token. No mod would add
  a new mineral character; the mapping from character to resource is content
  data (RON config), but the token itself is closed.
- **`ResourceCategory`** (`Strategic`, `Luxury`, `Bonus`) — a placement
  algorithm selector. The set of placement algorithms is closed; only the
  resources within each category are open content.
- **`BoundaryType`**, **`PlateType`**, **`RiverSize`** — internal
  generation mechanics with no content-modding dimension.

### Stable string IDs

String IDs are the authoritative identity that crosses save files and the
network. They are snake_case ASCII:

- Biomes: `"boreal_forest"`, `"desert"`, `"grassland"`,
  `"temperate_forest"`, `"tropical_rainforest"`, `"tundra"`
- Resources: `"fish"`, `"gems"`, `"gold"`, `"horses"`, `"iron"`,
  `"spices"`, `"stone"`, `"wheat"`, `"wine"`

Handle assignment is alphabetical, so the biome handles are:
- 0: `"boreal_forest"`, 1: `"desert"`, 2: `"grassland"`,
  3: `"temperate_forest"`, 4: `"tropical_rainforest"`, 5: `"tundra"`

### Deferred: mod-manifest and multi-bundle load system

The mod-manifest and multi-bundle merge/order system is not built here.
Opening the kinds is the prerequisite; the bundle system has no consumer
until a second bundle exists (Stage 8 is the first realistic point). What
this stage guarantees is that the representation it leaves behind
(string-keyed, registry-resolved, load-through-one-path) is one a manifest
system can sit on top of without rework.

## Consequences

- Biome and resource content is now open: new kinds can be added by
  adding entries to RON files without recompiling.
- Property dispatch replaces kind identity checks: `is_forest_biome(h)`
  replaces `match biome { BorealForest | ... => true, ... => false }`.
- The determinism guarantee is preserved: the same set of string IDs
  always produces the same handle assignment (alphabetical ordering is
  stable), so seeds produce byte-identical output.
- The reference hash (`WorldData:v6`) changed from v5 because handle
  indices differ from the previous enum discriminant ordering. The hash
  was updated and committed as part of Stage 3.7.
