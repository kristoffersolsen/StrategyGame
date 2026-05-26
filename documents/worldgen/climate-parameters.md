# Climate Stage — Parameter Reference

Parameters for Stage 3.2. Consumed by `ClimateParams` in
`crates/worldgen/src/params/climate.rs`. Set them via a RON preset
file (`--params <file.ron>`) passed to `maptest` or `mapdump`.

Two sets of defaults exist:

- **Struct defaults** — the values in `ClimateParams::defaults()`. These
  define the determinism reference output. Treat changes to them as
  generation-output changes.
- **Preset defaults** — the values in `tools/maptest/presets/default.ron`.
  These are the tuned working values used for actual map generation.

When the columns differ, prefer the preset value for real runs.

The climate stage runs a moisture advection loop: evaporation sources
inject moisture at water tiles, the wind field carries it downwind, and
a precipitation pass removes a fraction each iteration. Four supplementary
mechanisms extend the base model — see ADR-0008 for the full motivation
behind each.

---

## Step 1 — Temperature field

### `equator_temperature` / `pole_temperature`

| | Equator | Pole |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `1.0` | `−0.5` |
| Preset default | `1.0` | `−0.5` |
| Constraint | finite, equator > pole | finite, pole < equator |

Normalised temperature at the equator and poles. The temperature field
is computed as a cosine interpolation between these two values across
latitude (`r`). The scale is arbitrary — downstream stages (biomes)
compare temperature to thresholds in the same units.

**Tuning:** The gap between the two values controls the steepness of the
temperature gradient. A larger gap → sharper latitude bands, more extreme
polar/equatorial contrast. Shifting both values up or down uniformly is
equivalent to changing `climate_band_offset`. Changing only the pole
value to a less-negative number narrows the polar cold zone.

### `elevation_lapse_rate`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.25` |
| Preset default | `0.2` |
| Constraint | `≥ 0.0`, finite |

Temperature reduction per unit of elevation above sea level. Applied to
tiles with positive continuous elevation values.

**Tuning:** Higher values make mountains significantly colder than
surrounding lowlands, strengthening their effect on moisture capacity
and therefore precipitation. At the preset value `0.2`, a tile at
elevation `1.0` is `0.2` temperature units cooler than sea level. This
is the primary source of elevation-driven climate variation other than
the orographic precipitation mechanism.

### `climate_band_offset`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.0` |
| Preset default | `0.0` |
| Constraint | finite |

Global temperature shift added after all other calculations. Zero at
the tuning layer. The advanced-layer `ClimateBand` enum maps to an
offset applied here (e.g., `IceAge` maps to a negative offset). Not
typically tuned directly in presets.

---

## Step 2 — Wind field

### `wind_amplitude`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.18` |
| Preset default | `0.18` |
| Hard cap | `≤ 0.25` (CFL limit — enforced at validation) |
| Constraint | `≥ 0.0`, finite |

Amplitude `A` of the base zonal wind profile `u(φ) = −A·cos(3π·φ)`,
where `φ ∈ [−1, +1]` is signed latitude. This single-cosine formula
produces three climate bands per hemisphere (trade winds near the
equator, westerlies at mid-latitudes, polar easterlies at high
latitudes) without discontinuities at the band boundaries.

**Tuning:** The CFL constraint caps this at `0.25`. At `0.18` (the
default) wind speed is 72% of the cap, giving comfortable numerical
headroom while producing clearly visible moisture gradients. Raising
toward `0.24` strengthens moisture transport and sharpens rain shadows
but leaves little margin for the noise perturbation. Lowering below
`0.1` weakens directional moisture transport until it is comparable
to the diffusion term.

### `wind_noise_amplitude`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.04` |
| Preset default | `0.1` |
| Constraint | `≥ 0.0`, finite |

Amplitude of the low-frequency OpenSimplex noise perturbation added
independently to both wind components. Breaks up the perfectly uniform
zonal pattern.

**Tuning:** Raising this introduces more geographic variation in wind
direction, reducing stripe artefacts and making moisture distribution
less perfectly banded. Values above ~`0.1` at `wind_amplitude = 0.18`
may push individual tile wind magnitudes close to the CFL limit.
A debug assertion in the wind field constructor catches magnitudes
above `0.5`.

### `wind_noise_frequency`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.5` |
| Preset default | `4.0` |
| Constraint | `> 0.0`, finite |

Spatial frequency of the wind noise perturbation.

**Important calibration note:** The noise uses torus lifting for
east-west seamlessness. The effective spatial variation depends on
`TORUS_RADIUS × frequency` where `TORUS_RADIUS = 1/(2π) ≈ 0.159`.
At the struct default `0.5`, the effective radius is `≈ 0.08` —
adjacent tiles produce nearly identical noise values, giving negligible
perturbation despite a nonzero amplitude. A frequency of at least
`~map_width / 100` is needed for visible effect; the preset uses `4.0`
(effective radius `≈ 0.64`) which produces continent-scale wind
deflections. See ADR-0009 for the full analysis.

---

## Step 3 — Evaporation source

### `evaporation_rate`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.05` |
| Preset default | `1.0` |
| Constraint | `≥ 0.0`, finite |

Base per-iteration moisture added to each water tile, before temperature
scaling. The effective rate is `evaporation_rate × clamp((temp − floor) × scale, 0, 1)`.

**Tuning:** This is the primary control on the total moisture injected
into the system per iteration. The preset uses a high value (`1.0`)
combined with `moisture_cap = 50` to limit accumulation at ocean tiles
while maintaining a strong moisture source. At the struct default
(`0.05`) moisture levels stay low enough that no cap is needed, but
inland moisture is also very sparse.

### `evaporation_temp_floor`

| | |
|---|---|
| Type | `f32` |
| Struct default | `−0.2` |
| Preset default | `−0.2` |
| Constraint | finite |

Temperature below which no evaporation occurs. Water tiles at or below
this temperature contribute zero moisture. Prevents frozen polar oceans
from acting as moisture sources.

**Tuning:** Raising the floor (toward `0.0` or positive) reduces the
contribution of cold sub-polar seas, making polar regions drier.
Lowering it (e.g. `−1.0`) allows all ocean tiles to evaporate regardless
of temperature, useful for stress tests or unusual climate configurations.

### `evaporation_temp_scale`

| | |
|---|---|
| Type | `f32` |
| Struct default | `1.0` |
| Preset default | `0.2` |
| Constraint | finite |

Linear scaling factor for the evaporation–temperature relationship.
The effective rate fraction is `clamp((temp − floor) × scale, 0, 1)`.

**Tuning:** Higher values produce a stronger temperature dependence —
warm ocean tiles evaporate much more than cool ones. At scale `1.0` the
full evaporation rate is reached when temperature is `1/scale = 1.0`
unit above the floor. The preset uses `0.2`, which spreads the
temperature effect over a wider range and keeps most ocean tiles
contributing at a reasonable fraction of the peak rate.

### `coastal_moisture_factor` *(supplementary, step 3b)*

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.0` (disabled) |
| Preset default | `1` |
| Constraint | `≥ 0.0`, finite |

Per-iteration moisture added to each land tile per adjacent water tile.
A land tile with N water neighbours receives `coastal_moisture_factor × N`
moisture per iteration, independent of wind direction.

**Purpose:** Addresses the failure mode where wind running parallel to
a coast leaves coastal land tiles dry despite adjacent ocean. Models
sea breeze and coastal fog. Water tiles and tiles with no water
neighbours receive zero.

**Tuning:** At `0.0` (the struct default) the mechanism is disabled and
the model is purely advective. At the preset value `1`, a tile with
one water neighbour receives the same moisture per iteration as a water
tile evaporates at warm temperatures. Start with small values (`0.01`–
`0.05`) and increase until coastal land is visibly more humid than
equally-distant inland tiles.

### `moisture_cap` *(supplementary, step 3c)*

| | |
|---|---|
| Type | `f32` |
| Struct default | `f32::MAX` (disabled) |
| Preset default | `50.0` |
| Constraint | `> 0.0`, finite or `f32::MAX` |

Maximum moisture any tile can accumulate. After each advection step,
tile moisture is clamped to this value; excess moisture is removed from
the system.

**Purpose:** Prevents ocean and coastal tiles from accumulating unbounded
moisture over many iterations, which would compress interior land values
to near-zero in visualisations. `f32::MAX` disables the cap entirely.

**Tuning:** Set this to a value comparable to a few iterations' worth
of ocean evaporation. At `evaporation_rate = 1.0`, an ocean tile adds
~`1.0` moisture per iteration (temperature-scaled); a cap of `50` lets
tiles accumulate about 50 iterations' worth before clamping. Lowering
the cap below `5–10` may cause the field to saturate too aggressively,
giving inland tiles insufficient moisture signal. Note that with the
cap active, moisture + precipitation no longer equals total evaporation
(moisture is removed at the cap).

---

## Steps 4–5 — Advection and precipitation

### `base_precipitation_rate`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.02` |
| Preset default | `0.05` |
| Range | `[0.0, 1.0]` |

Fraction of tile moisture precipitated per iteration in flat, warm
terrain. The actual fraction is `min(max_fraction, base × capacity × orographic_boost)`.

**Tuning:** This is the primary control on how far inland moisture
travels. Higher rates → more precipitation near the coast but drier
interiors. Lower rates → moisture travels further inland but absolute
precipitation levels are low everywhere. A rate of `0.02` means moisture
loses ~2% per hop on flat land before temperature and orographic
factors — it survives roughly 50 hops before dropping to ~37% of its
initial value. At `0.05` that drops to ~20 hops. Combine with
`evapotranspiration_rate` to recover inland moisture transport.

### `temp_capacity_offset` / `temp_capacity_scale`

| | Offset | Scale |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `0.5` | `4.0` |
| Preset default | `2` | `1.0` |
| Constraint | finite | `> 0.0`, finite |

Parameters of the logistic sigmoid `σ((temp − offset) × scale)` that
produces the temperature-dependent moisture-holding capacity factor.
The capacity factor equals `0.5` when `temp = offset`; it approaches
`1.0` (full precipitation) for warm tiles and `0.0` (no precipitation)
for cold tiles.

**Tuning:** `temp_capacity_offset` sets the temperature at which 50%
of the base precipitation fraction is used. At the struct default
(`0.5`), tiles at moderate temperature precipitate at half the base
rate; at the preset value (`2.0`), only warm near-equatorial tiles
reach full precipitation efficiency. A higher offset suppresses
precipitation in mid-latitude and cold tiles, concentrating rainfall
in the tropics. `temp_capacity_scale` controls the sharpness of the
transition — high scale gives a sharp cold/warm boundary; low scale
spreads the transition over a wider temperature range.

### `orographic_factor`

| | |
|---|---|
| Type | `f32` |
| Struct default | `1.5` |
| Preset default | `0.05` |
| Constraint | `≥ 0.0`, finite |

Multiplier on positive elevation gain for orographic precipitation
enhancement. When a tile is higher than its upwind neighbour by `Δelev`,
the precipitation fraction is multiplied by `1 + orographic_factor × Δelev`.

**Purpose:** Produces rain shadows: windward mountain slopes force
moist air to rise and precipitate; leeward slopes receive dry air.

**Tuning:** This is the headline parameter for rain shadow intensity.
The struct default (`1.5`) is strongly orographic. The preset value
(`0.05`) is much weaker — at the preset's high `base_precipitation_rate`
and `diffusion_rate` values, the base signal is already dominant and
a large `orographic_factor` would produce unrealistically extreme
rain shadows. Tune this in combination with `base_precipitation_rate`:
at a low base rate, raise the orographic factor to compensate. At a
high base rate, lower it.

### `max_precipitation_fraction`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.6` |
| Preset default | `0.8` |
| Range | `(0.0, 1.0]` |

Hard cap on the fraction of tile moisture that can precipitate per
iteration, regardless of orographic or temperature factors. Prevents
numerical blow-up when factors combine to demand more moisture than
the tile holds.

**Tuning:** Raising toward `1.0` allows tiles to shed almost all their
moisture in a single iteration — appropriate for extreme orographic
events. Lowering to `0.3`–`0.4` softens extreme precipitation spikes,
producing a more uniform distribution. Values above `0.8` rarely change
the visual result significantly because the `capacity × orographic`
product rarely exceeds that even with `orographic_factor = 1.5`.

### `evapotranspiration_rate` *(supplementary, step 4b)*

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.0` (disabled) |
| Preset default | `0.01` |
| Constraint | `≥ 0.0`, finite |

Fraction of a land tile's moisture recycled back into the atmosphere
per iteration. Models plant transpiration and soil evaporation: wet
land tiles re-emit a fraction of their moisture, allowing it to
advect further inland rather than decaying geometrically after
each precipitation hop.

**Purpose:** Addresses the failure mode where moisture dies out too
quickly above land. Without this, moisture decays as `(1 − base_rate)^n`
per hop and is exhausted within ~10–20 tiles inland at viable
precipitation rates. Evapotranspiration breaks the geometric decay,
enabling moisture to sustain itself across a continent.

**Tuning:** At `0.01`, a tile with moisture `M` contributes
`0.01 × M` back per iteration — a small but persistent recycling.
Values above `0.2` can cause moisture to accumulate in inland basins
(wet land tiles evapotranspire more than they precipitate). Start at
`0.01`–`0.05` and raise if inland regions are too dry; lower if
inland lakes or humidity hotspots form.

### `diffusion_rate` *(supplementary, step 4c)*

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.0` (disabled) |
| Preset default | `0.5` |
| Range | `[0.0, 0.5]` |

Fraction of each tile's moisture distributed isotropically (equally in
all six hex directions) per iteration, independent of wind. Models
turbulent atmospheric mixing.

**Purpose:** Addresses the failure mode where a purely zonal wind field
cannot transport moisture north-south, preventing N-S latitude band
formation. Diffusion enables N-S moisture transport and allows wet
tropical and mid-latitude bands to form even with weak N-S wind.

**CFL interaction:** Diffusion consumes `diffusion_rate` of the total
outflow budget. The stability condition is `W + D ≤ 1.0` where `W`
is the wind outflow fraction (≤ 0.5 from the CFL cap) and `D` is
`diffusion_rate`. The validation cap of `0.5` ensures the worst case
is exactly `1.0`. See ADR-0008.

**Tuning:** The preset uses `0.5` — the maximum — which is aggressive.
At this level diffusion is as strong as the wind signal and the
moisture field is heavily smoothed. This was chosen to compensate for
the difficulty of producing N-S bands with a zonal wind field. If
distinct rain shadows are more important than latitude bands, lower
this to `0.05`–`0.15` and accept more uniform N-S distribution.

---

## Step 6 — Iteration count

### `iteration_count_factor`

| | |
|---|---|
| Type | `f32` |
| Struct default | `1.5` |
| Preset default | `1.5` |
| Constraint | `> 0.0`, finite |

Multiplier on `map_width` to derive the iteration count:
`max(iteration_count_min, round(factor × map_width))`.

At `1.5`, a 240-wide map runs 360 iterations — enough for moisture
to complete roughly 1.5 transits across the map.

**Tuning:** Sweep this from `0.8` to `2.5` in 5 steps to verify that
the moisture and precipitation fields stabilise (fields should change
little above ~`1.2` if the system reaches effective steady state).
Raising above `2.0` rarely changes the visual result and increases
generation time proportionally.

### `iteration_count_min`

| | |
|---|---|
| Type | `u32` |
| Struct default | `100` |
| Preset default | `500` |

Minimum iteration count regardless of map size. Prevents small maps
from terminating before latitude bands become visible.

**Tuning:** The preset uses `500` for small map tests. On very small
maps (width < 50), the factor-derived count may be too low for the
field to converge; raise `iteration_count_min` to compensate. For
medium and large maps the factor-derived count dominates and this
floor is not reached.

---

## Parameter interactions summary

The following pairs have the strongest interactions and should be tuned
together:

| Pair | Interaction |
|---|---|
| `evaporation_rate` + `moisture_cap` | High evaporation rate requires a finite cap to prevent coastal dominance |
| `base_precipitation_rate` + `evapotranspiration_rate` | Low precipitation decays moisture; evapotranspiration compensates. Tune together to achieve target inland moisture reach |
| `orographic_factor` + `base_precipitation_rate` | High orographic factor combined with high base rate produces extreme rain shadows; keep their product moderate |
| `diffusion_rate` + `wind_amplitude` | Strong diffusion smooths the directional signal from wind; at diffusion = 0.5, even high wind amplitudes produce soft, blended moisture gradients |
| `wind_noise_frequency` + `wind_noise_amplitude` | Frequency must be ≥ 4.0 for amplitude to have visible effect (torus-lifting geometry); see ADR-0009 |
| `temp_capacity_offset` + `elevation_lapse_rate` | Higher lapse rate makes mountains colder, pushing them below the capacity sigmoid midpoint; lowers orographic precipitation (partially counteracting `orographic_factor`) |
