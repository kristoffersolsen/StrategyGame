# Hydrology Stage - Parameter Reference

Parameters for Stage 3.3. Consumed by `HydrologyParams` in
`crates/worldgen/src/params/hydrology.rs`.

The hydrology stage consumes tectonic elevation and climate precipitation, then
produces drainage, lakes, rivers, floodplains, canyons, canyon walls, and a
post-hydrology height-level copy.

Two sets of defaults exist:

| Parameter | Struct default | Preset default |
|-----------|----------------|----------------|
| `lake_basin_area_threshold` | `50` | `50` |
| `river_flow_threshold` | `0.15` | `0.04` |
| `major_river_flow_threshold` | `0.60` | `0.22` |
| `floodplain_elevation_tolerance` | `0.05` | `0.05` |
| `canyon_flow_slope_threshold` | `0.25` | `0.25` |
| `canyon_depth` | `1` | `1` |

When the columns differ, prefer the preset value for real map-generation runs.

## Steps 2-3 - Basin Detection And Sink Resolution

### `lake_basin_area_threshold`

| | |
|---|---|
| Type | `u32` |
| Struct default | `50` |
| Preset default | `50` |
| Constraint | `> 0` |

Minimum basin tile count required for a sink basin to become a lake. Basins below
this threshold are filled to their spill point instead of being marked as lakes.

Large basins are marked as lake tiles below the spill elevation. Their
`lake_outlet` points at the first outside tile where accumulated flow exits the
basin. Basins with no spill point are treated as true sinks and become lakes.

**Tuning:** Lower values create more inland lakes and interrupt more river
networks. Higher values fill more depressions, producing fewer lakes and longer
continuous drainage paths.

## Step 5 - River Identification

### `river_flow_threshold`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.15` |
| Preset default | `0.04` |
| Constraint | finite, in `[0.0, 1.0]` |

Minimum normalized accumulated flow for a tile to become a river tile. Tiles below
this threshold are not rivers.

Flow accumulation is precipitation-weighted and normalized by the maximum land flow
on the map, so this threshold is map-size independent.

**Tuning:** Lower values create more creeks and longer river networks. Higher values
restrict rivers to high-flow trunk channels.

### `major_river_flow_threshold`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.60` |
| Preset default | `0.22` |
| Constraint | finite, in `[0.0, 1.0]`, and `>= river_flow_threshold` |

Normalized accumulated flow at which a river tile becomes a major river.

The middle `River` tier is derived automatically as the midpoint between
`river_flow_threshold` and `major_river_flow_threshold`:

```text
river_threshold = (river_flow_threshold + major_river_flow_threshold) * 0.5
```

Classification:

| Flow range | Result |
|------------|--------|
| `< river_flow_threshold` | no river |
| `>= river_flow_threshold` and `< midpoint` | `Creek` |
| `>= midpoint` and `< major_river_flow_threshold` | `River` |
| `>= major_river_flow_threshold` | `MajorRiver` |

**Tuning:** Lowering this value makes major rivers common. Raising it reserves the
major tier for only the highest-flow channels.

## Step 6 - Floodplain Marking

### `floodplain_elevation_tolerance`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.05` |
| Preset default | `0.05` |
| Constraint | finite and `>= 0.0` |

Maximum absolute continuous-elevation difference between a land tile and an
adjacent river tile for the land tile to become a floodplain.

River tiles themselves are not floodplains. Water and lake tiles are not
floodplains.

**Tuning:** Higher values widen floodplain bands onto rougher terrain. Lower values
restrict floodplains to very flat land immediately beside rivers.

## Step 7 - Canyon Carving

### `canyon_flow_slope_threshold`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.25` |
| Preset default | `0.25` |
| Constraint | finite and `>= 0.0` |

Minimum stream-power product required to carve a canyon:

```text
flow_accumulation * max(0.0, elevation[current] - elevation[downstream])
```

Only river tiles with a downstream flow direction can become canyons.

**Tuning:** Lower values create more canyons, including gentler river slopes.
Higher values restrict canyon carving to steep, high-flow channels.

### `canyon_depth`

| | |
|---|---|
| Type | `u8` |
| Struct default | `1` |
| Preset default | `1` |
| Range | `1..=2` |

Number of discrete height levels removed from canyon floor tiles. The result is
saturating and clamped to at least land level `2`, so canyon carving does not turn
land into coast or ocean.

When a canyon tile is carved, the two flank directions relative to flow are marked
as canyon walls if they are land.

**Tuning:** `1` gives visual and tactical canyon features without radically
changing land classification. `2` creates stronger cuts and should be treated as
an aggressive debug or special-map value.

