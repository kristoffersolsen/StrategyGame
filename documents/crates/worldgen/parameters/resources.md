# Resource Stage - Parameter Reference

Parameters for Stage 3.5. Consumed by `ResourceParams` in
`crates/worldgen/src/params/resources.rs`.

The resource stage places one optional resource per tile from the
designer-authored config at `assets/worldgen/resources.ron`. The two Rust tuning
parameters control shared scatter behavior; per-resource rarity, category,
eligibility, mineral affinities, and clustering live in the RON asset documented in
[../data-assets.md](../data-assets.md).

Two sets of defaults exist:

| Parameter | Struct default | Preset default |
|-----------|----------------|----------------|
| `cluster_expansion_attempts` | `8` | `8` |
| `poisson_max_attempts` | `30` | `30` |

## Shared Placement Model

For each resource, the stage first builds an eligible tile list from the RON config.
The target count is:

```text
round(eligible_tile_count * rarity * abundance_multiplier)
```

The abundance multiplier comes from `AdvancedParams::resource_abundance`:

| `ResourceAbundance` | Multiplier |
|---------------------|------------|
| `Scarce` | `0.6` |
| `Standard` | `1.0` |
| `Abundant` | `1.5` |

Actual placement can be lower than the target if the stage exhausts placement
attempts, if spacing rules reject candidates, or if earlier resources already occupy
candidate tiles. Resource categories are processed in this order:

1. Bonus
2. Strategic
3. Luxury

## Step 7 - Clustered Luxury Expansion

### `cluster_expansion_attempts`

| | |
|---|---|
| Type | `u8` |
| Struct default | `8` |
| Preset default | `8` |
| Range | `1..=32` |

Maximum neighbor candidates tried per clustered luxury seed when expanding the seed
into a small deposit cluster.

Luxury placement first places baseline deposits, then chooses mineral-weighted seed
tiles. For each successful seed, the stage attempts to grow a cluster of size 2-4
within the resource's configured `cluster_radius`. Each expansion attempt picks a
random candidate within that radius and accepts it if the tile is eligible and empty.

**Tuning:** Higher values make configured luxury clusters more likely to reach their
desired size. Lower values produce smaller or more fragmented clusters.

## Steps 4-7 - Scatter Attempt Limit

### `poisson_max_attempts`

| | |
|---|---|
| Type | `u8` |
| Struct default | `30` |
| Preset default | `30` |
| Range | `5..=100` |

Rejection-sampling attempt multiplier used by uniform and mineral-weighted scatter.

For a target count `N`, the stage tries up to:

```text
max(N * poisson_max_attempts, poisson_max_attempts)
```

candidates. Each candidate must be eligible, currently empty, and at least
`min_spacing` away from already placed deposits of the same resource.

**Tuning:** Higher values help dense or tightly constrained resources reach their
target counts, at the cost of more placement work. Lower values make placement faster
but increase the chance that resources underfill when eligibility is sparse or
spacing is strict.

