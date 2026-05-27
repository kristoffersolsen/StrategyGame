# ADR-0017: Resource Placement Model

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

Stage 3.5 places resources onto the world map using the plate mineral
character data already produced by Stage 3.1. The design doc (§1.13)
describes the high-level intent — geological placement, biome filtering,
plate-weighted luxuries, clustered versus scattered placement — but
leaves several concrete decisions deferred to implementation:

- Which resources to include in the first pass.
- How strategic resources should relate to mineral character, given that
  a player locked out of iron in the early game is not playing a
  different strategy — they are simply losing.
- Whether the RON config is one file or per-resource files.
- Whether a tile holds one resource or many.
- How ocean tiles (e.g. fish) interact with the plate mineral character
  system designed for land.

These decisions are cross-cutting: they affect `world_data.rs`,
`params/resources.rs`, `assets/worldgen/resources.ron`, and the
placement algorithm in `stages/resources/`. Recording them here avoids
scattering the rationale across code comments.

## Decisions

### 1. One resource per tile

`ResourceData` stores a `Field<Option<ResourceId>>`. Each tile holds at
most one resource. This matches Civilization-style conventions, keeps
rendering and game logic simple, and is consistent with the rest of the
pipeline's one-value-per-tile field design. Resources are not stacked.

### 2. Three placement behaviours by category

Resources fall into three categories with distinct placement behaviours.
The category determines which algorithm is used; the per-resource RON
config supplies the parameters.

**Luxury:** Almost entirely plate-mineral-character driven. A small
baseline fraction (≈5%) is placed globally across eligible tiles to
keep the code path consistent and guard against degenerate maps with
very few plates of the right character. The concentration fraction
(≈95%) uses mineral character weighting. Luxuries cluster into 2–4
tile groups via Poisson-disk seed placement followed by neighbour
expansion. The intended gameplay result is genuine geographic
monopolies: this continent has gold, that one has gems.

**Strategic:** Mixed placement. A baseline fraction is placed globally
across eligible tiles without any plate weighting, guaranteeing that
no region is completely locked out. An additional concentration
fraction uses plate mineral character weighting to produce regions
of greater abundance. The baseline/concentration split is tuned per
resource: early-game resources critical to progression (iron) sit
closer to 60/40, giving a higher floor; later-game resources (coal)
sit closer to 40/60, allowing more geographic concentration since
trade networks exist by the time they matter. The distinction between
luxury and strategic is functional, not aesthetic: all luxuries
provide equivalent happiness bonuses and are interchangeable; strategic
resources unlock specific eras and unit types and are not.

**Bonus:** Biome and terrain filtered only. Plate mineral character
plays no role. Uniform scatter via Poisson-disk with minimum spacing.
Bonus resources are farmland, coastal food, and raw materials — their
scarcity comes from terrain eligibility alone.

### 3. First-pass resource list

Ten resources are implemented in Stage 3.5. This set exercises every
code path (clustered luxury, mixed strategic, ocean-tile bonus, purely
biome-filtered bonus) while remaining small enough to tune across a
range of seeds.

**Strategic (2):** iron, horses.  
**Luxury (4):** gold, gems, spices, wine.  
**Bonus (3):** wheat, fish, stone.

Later-tech strategics (coal, oil, uranium) are deferred. Their
placement rules are identical in structure; adding them is a content
addition, not an algorithm change.

Silk is deferred from the luxury set. Its biome eligibility is a
narrow edge case requiring a biome variant not yet present, and the
four included luxuries already exercise the full range of placement
behaviours.

### 4. Baseline/concentration split per strategic resource

| Resource | Baseline | Concentration | Primary mineral affinity |
|---|---|---|---|
| Iron | 60% | 40% | Ferrous |
| Horses | 70% | 30% | Industrial (marginal) |

Horses receive a marginal Industrial affinity (open plains on stable
continental plates) but the 70% baseline makes placement effectively
uniform in practice. The affinity adds slight regional flavour without
creating meaningful scarcity.

### 5. Mineral character to resource affinity mapping

Each luxury's concentration pass is weighted by how well the tile's
plate mineral character matches the resource. The mapping is content
data in the RON config (a per-resource affinity weight per
`MineralCharacter` variant), not hardcoded in generation logic.
Initial defaults:

| Resource | Ferrous | Precious | Gems | Hydrocarbons | Industrial |
|---|---|---|---|---|---|
| Gold | 0.6 | 1.0 | 0.2 | 0.0 | 0.1 |
| Gems | 0.1 | 0.3 | 1.0 | 0.0 | 0.1 |
| Spices | 0.0 | 0.1 | 0.0 | 0.0 | 0.2 |
| Wine | 0.0 | 0.1 | 0.0 | 0.0 | 0.3 |

Spices and wine are primarily biome-restricted (tropical-only and
temperate-only, respectively); the affinities are low and non-zero
only to preserve code path consistency. Their scarcity comes from
biome eligibility, not mineral character.

### 6. Ocean tiles use plate mineral character normally

Fish placement filters to water tiles (ocean, coast) rather than land
tiles. Ocean tiles already carry a `PlateId` from Stage 3.1 plate
growth, so the same mineral character lookup works without special
casing. Fish is a bonus resource and uses uniform scatter, so the
affinity weight is irrelevant for the initial implementation; the
infrastructure is in place for future ocean luxury or strategic
resources that may use plate weighting.

### 7. Single RON config file, structured by category

All resources are defined in `assets/worldgen/resources.ron`. The file
is divided into three named sections: `strategic`, `luxury`, and
`bonus`. Within each section, resources are named map entries. This
keeps all resource content data in one place (easy to compare and
cross-reference) while remaining readable as the list grows. A
per-resource file layout would be cleaner for large lists but adds
loader complexity and filesystem noise for ten resources.

Per-resource fields in the config:

- `category`: `Strategic | Luxury | Bonus`
- `eligible_biomes`: list of `Biome` variants (empty = all land, or
  specify `ocean`/`coast` for water tiles)
- `eligible_terrain`: optional terrain constraints (e.g. horses require
  non-hilly, non-mountain tiles)
- `latitude_preference`: optional `(min, max)` normalised latitude band
  (spices in tropics, tundra furs in cold — future use)
- `rarity`: target fraction of eligible tiles to receive this resource
- `cluster`: `true` for luxuries (Poisson-disk seed + neighbour
  expansion); `false` for scattered placement
- `cluster_radius`: tile radius for neighbour expansion (luxuries only)
- `min_spacing`: minimum tile distance between separate deposits
- `baseline_fraction`: fraction of target count placed without plate
  weighting (strategics only; ignored for luxury and bonus)
- `mineral_affinities`: map of `MineralCharacter → f32` weight
  (luxuries only; ignored for bonus)

## Consequences

**Positive:**

- Adding a new resource requires only a new entry in `resources.ron`
  and a new `ResourceId` variant. No algorithm changes.
- The baseline/concentration split eliminates the strategic scarcity
  failure mode (player locked out of a necessary resource) without
  flattening geographic character on the body of the map.
- Luxury monopolies are preserved at full strength — the plate
  clustering behaviour the design doc calls for is uncompromised.
- Ocean tiles work without special-casing the mineral character lookup.
- One RON file is easy to audit, diff, and compare across resources.

**Negative:**

- `baseline_fraction` is a strategic-only field that is present but
  ignored on luxury and bonus entries. The config is slightly noisier
  than a fully discriminated union would be. Acceptable at this scale.
- The 5% luxury baseline means the monopoly mechanic is not absolute.
  In practice the biome filter already provides non-zero scarcity
  independent of plate character; the 5% baseline adds at most a few
  tiles globally and does not undermine the monopoly feel.

## Alternatives considered

**Strategic resources fully plate-dependent (no baseline):** Rejected.
A player starting on a map with no iron-adjacent plates in their region
has no early-game path to iron units. This is a degenerate start, not
interesting geographic constraint. The starting-position adequacy floor
(Stage 3.6) requires at least one strategic resource near every start,
but it cannot guarantee iron specifically — only the placement stage can
do that by ensuring global coverage.

**Per-resource RON files:** Rejected. Loader complexity and filesystem
noise are not worth the cleanliness benefit at ten resources. Revisit
if the list exceeds thirty or forty entries.

**Multiple resources per tile:** Rejected. Adds rendering and game
logic complexity. No gameplay need identified: a tile with both iron
and horses is not meaningfully different from two adjacent tiles with
one resource each.

**Separate ocean mineral character:** Rejected. Ocean tiles already
have a plate assignment from Stage 3.1 that carries mineral character.
Using it directly avoids a new data field and is geologically plausible
(ocean floor mineral character reflects the same geological processes
as the plate assignment).

## References

- Design doc §1.13 — resource placement design
- Design doc §1.14 — starting position adequacy floor (context for why
  strategic scarcity is a placement-stage problem, not a
  starting-position-stage problem)
- `crates/worldgen/src/world_data.rs` — `MineralCharacter` enum,
  `PlateId` field
- `assets/worldgen/resources.ron` — resource config (created in
  Stage 3.5)
- ADR-0016 — biome table as data asset (precedent for RON data assets)
