# Adding Biomes to the Pipeline

This document describes how to extend the biome set after Stage 3.4
has landed. It is a checklist-style procedure, not a design document —
the design decisions that shaped the biome system are in
`documents/design/design-decisions.md` §1.12 and ADR-0008.

---

## When to do this

Adding a biome is a **generation-output change**. It shifts the biome
map for all seeds. This matters because:

- Resources (Stage 3.5) are placed using biome filtering. A new biome
  with no resource rules will be treated as having no valid resource
  placements until those rules are added.
- Starting positions (Stage 3.6) use viability scores that depend on
  biome. A new biome with no viability weight defaults to zero, which
  can starve starts near large areas of that biome.
- Determinism hashes in the test suite reference stable generation
  output. Adding a biome updates those hashes.

The right time to add biomes is **before Stage 3.5 and 3.6 are tuned**,
or as a deliberate design iteration after both stages are complete and
you are prepared to re-tune downstream parameters from scratch.

Do not add biomes mid-tuning of resources or starting positions.

---

## Checklist

Work through these steps in order. Each step is independently
committable; prefer one commit per step.

### 1. Update the `Biome` enum

File: `crates/worldgen/src/world_data.rs`

Add the new variant to the `Biome` enum. Follow the naming conventions
in `documents/guides/style-guide.md` §6 (`PascalCase`, no abbreviations).

```rust
pub enum Biome {
    TropicalRainforest,
    TemperateForest,
    BorealForest,
    Grassland,
    Desert,
    Tundra,
    Alpine,          // ← example new variant
}
```

`Biome` derives `Ord` (required for `BTreeMap` keying in
`hydrology_overrides`). If you add a variant, the derived `Ord` order
changes based on declaration order — this is intentional and stable as
long as you do not reorder existing variants. **Do not reorder existing
variants.**

The compiler will now produce exhaustive-match errors everywhere `Biome`
is matched without a wildcard. Use those errors as your checklist for
the remaining steps.

### 2. Add a colour to the categorical colormap

File: `crates/worldgen/src/debug_viz.rs` (or wherever `BiomeCmap` is
defined after Stage 3.4 lands)

Every `Biome` variant must map to a distinct, visually separable RGB
colour in the categorical colormap used by `mapdump` and `maptest`.
No two biomes that commonly appear adjacent should share similar hues.

Add a match arm. Keep the colour palette consistent with the existing
entries — cooler colours for cold/wet biomes, warmer for hot/dry.

### 3. Update the biome table RON file

File: `assets/worldgen/biome_table.ron`

Decide which `(temperature_bin, moisture_bin)` cells should classify
as the new biome. Edit the relevant entries. If the new biome requires
a finer grid (more bins), update `temperature_bin_count` or
`moisture_bin_count` in the file header **and** update `BiomeParams`
defaults to match — the two must agree or `validate()` rejects the
table at load time.

Run `validate()` mentally or via a quick `mapdump` run before
committing. Every cell in the configured grid must have exactly one
entry.

If you are expanding the bin grid, the existing entries will no longer
cover the full new grid. Fill all new cells before committing.

### 4. Update hydrology overrides (if applicable)

File: `assets/worldgen/biome_table.ron` (`hydrology_overrides` map)  
File: `crates/worldgen/src/stages/biomes/hydrology_overrides.rs`

Decide whether the new biome should participate in hydrology overrides:

- Should rivers/floodplains/lake-adjacent tiles ever reclassify *to*
  the new biome? Add it as a value in `hydrology_overrides`.
- Should tiles *of* the new biome be reclassified by water proximity?
  Add it as a key in `hydrology_overrides` with its target biome.

If the new biome does not interact with hydrology at all, no change is
needed here.

### 5. Add resource placement rules (Stage 3.5 only)

File: wherever Stage 3.5 defines per-biome resource filters

If Stage 3.5 is already implemented, the new biome needs explicit
resource rules. A biome with no rules is treated as having no valid
resource placements — this silently produces resource-free dead zones
across any large area of the new biome.

Add a match arm to every per-biome filtering function in the resource
stage. If the new biome should behave like an existing one for
resources, copy its rules; do not use a wildcard default that silently
forwards to an arbitrary biome.

### 6. Add starting position viability weight (Stage 3.6 only)

File: wherever Stage 3.6 defines per-biome viability scores

If Stage 3.6 is already implemented, the new biome needs a viability
weight. A missing weight defaults to zero, which means the adequacy
floor can fail for seeds where a new biome covers candidate start areas.

Add a match arm. If the new biome is hospitable (e.g. a fertile
wetland), give it a weight similar to `Grassland`. If it is hostile
(e.g. a salt flat), weight it similarly to `Desert`.

### 7. Update the determinism test hashes

File: wherever the determinism scaffold stores reference hashes
(see `crates/worldgen/src/lib.rs` and the test scaffold from Stage 2)

Run the pipeline with a fixed seed and update the reference hash for
the `biome` field. The hash will have changed because tile
classifications changed. This is expected; commit the new hash with a
message that explains which biome was added.

If resources and starting positions also have reference hashes, update
those too.

### 8. Run visual validation

```bash
cargo run --release --bin maptest -- run --seeds 1,2,3,4,5,6,7,8
```

Open the biome map PNGs. Verify:

- The new biome appears in at least one seed across the test range.
  If it never appears, your bin assignments or grid coverage may be
  wrong.
- The new biome's colour is visually separable from its neighbours.
- No unexpected holes or artifacts at the edges of the new biome's
  climate range.

If the new biome requires re-tuning `hydrology_override_strength` or
`smoothing_passes` because the new zone interacts poorly with the
existing overrides, do that tuning now before moving on.

### 9. Update documentation

- `documents/design/design-decisions.md` §1.12 — add the new biome to
  the biome set list and note any design rationale (why this biome, what
  it represents, what gameplay role it fills).
- `assets/worldgen/biome_table.ron` — the file is self-documenting via
  comments; ensure the bin-range legend at the top is still accurate
  after any grid changes.
- This document — if the procedure itself needed deviation, record what
  changed and why so the next addition is easier.

---

## Shrinking the biome set

Removing a biome follows the same checklist in reverse. The compiler's
exhaustive-match errors again serve as the checklist. The main risk is
that tiles previously classified as the removed biome will fall back to
a neighbour biome in the lookup table — review the table entries for
the removed variant's cells and decide their new classifications
explicitly rather than relying on whatever the surrounding cells happen
to be.

---

## Reference

- `documents/design/design-decisions.md` §1.12 — biome system design
- `documents/adr/ADR-0008` — biome table as a data asset
- `assets/worldgen/biome_table.ron` — the table itself