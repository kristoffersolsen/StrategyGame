# ADR-0027: Landform Geometry — Structured Relief over Noise Displacement

**Status:** Accepted
**Date:** 2026-06-05
**Deciders:** Kristoffer

## Context

ADR-0026 decided that hills and mountains would be rendered as noise
displacement regimes on subdivided hex cores — low amplitude for hills,
high for mountains — and bet that contiguous mountain tiles sharing a
noise field would read as connected ranges. It explicitly left the door
open: "revisit only if the procedural result reads poorly."

The result reads poorly. After implementation (R1–R4) and extensive
parameter tuning, the displaced surfaces read as random noise rather than
landforms. The cause is inherent, not a tuning failure:

- fBm and related noise are structureless by design. Adding octaves adds
  finer crinkle, not large-scale organization. Landforms are the
  opposite — large-scale structure (summits, ridgelines, flanks) with
  detail subordinate to it.
- A *shared* noise field gives neighboring tiles correlated crinkle, not
  connected high ground. Correlation is not connectivity. Mountain tiles
  rendered as isolated bumps; the connectivity that makes a range read as
  a range never appeared.

A first attempt at a fix considered a single "structured relief"
mechanism (raised shared corners/edges) for both hills and mountains.
That fails for hills: raising shared height turns adjacent hills into a
rising mean, which reads as a tier change / plateau rather than rolling
hills. This surfaced the principle this ADR records.

## Decision

Replace noise displacement with structured relief, using **different
mechanisms for mountains and hills** because they have different
connectivity semantics.

**Principle.** A mountain range's connectivity *is* continuity of height
(clustered mountains share high ground; mean elevation rises where they
cluster). A hill field's connectivity is continuity of a *pattern at
constant mean height* (ridge-and-swale lines flowing across boundaries
while ground level stays put). One amplitude-driven mechanism cannot
serve both.

**Mountains — corner-and-edge crest skeleton.** Extend the corner-height
function (the R1/R2 lattice) so corners and shared edges of clustered
mountain tiles are raised toward peak height, forming a connected graph
of crests (massifs with ridgelines), watertight by the shared-corner
construction. Mountain-to-lower edges stay unraised and fall away as
flanks. 3–4 authored interior variants modulate the fill *under* the
crest graph as secondary character (applied with hex rotation/reflection);
the skeleton owns the dominant relief.

**Hills — directional ridge field.** A low-amplitude ridged profile
advances across a hybrid direction field (elevation aspect where regional
slope is defined; low-frequency, domain-warped perturbation on flats and
as variation), at constant tier height. Amplitude persists across
hill→hill edges and fades at hill→non-hill edges. Continuity comes from
the shared field (a pure function of world position), not from shared
height. No corner raising; no authored variants (a directional dome
re-introduces the singular-mound/plateau problem).

**Noise demoted.** Noise becomes a low-amplitude top detail layer on both
landform types — to break up interpolated smoothness — never the
structural mechanism.

**Transition detail (refines R2 geometry).** Slope transitions are
rounded with a smoothstep/Hermite height profile plus subdivision and
recomputed averaged normals (subdivision alone keeps the hard crease).
Cliff edges keep near-vertical faces with a hard top lip and face
normals. The slope/cliff choice is read from `edge_class` (core-owned, ≥2
delta = cliff per ADR-0012); the renderer owns no threshold, so visual
and passability cannot diverge. Subdivision is adaptive (slopes/hills
dense, cliffs sparse) and must match along every shared edge to avoid
T-junction cracks.

**Legibility split (narrows ADR-0026's corner decision).** ADR-0026
resolved to preserve tier-boundary legibility at corners. That is now
narrowed: tile *identity* moves to a future grid overlay, so slope
corners are rounded (identity is no longer carried by geometry), while
*passability* stays in geometry — cliff corners stay sharp. Sharp at
cliffs, rounded at slopes.

All landform and transition classification is presentation, derived in
`rendering` from the discrete mountain/hill masks. No new dependency into
`sim`; `edge_class` remains the only tendril.

This supersedes the displacement-regime portion of ADR-0026 and narrows
its corner-legibility decision. The rest of ADR-0026 (the corner-primary
lattice, tier height from `mountain_level`, per-vertex biome blending,
slope-gated triplanar, the ocean shader, cadence-separated render layers,
the frame-time gate) stands.

## Consequences

**Positive:**

- Mountains read as connected ranges with ridgelines; hills read as
  flowing rolling terrain at constant mean height. Both fix connectivity,
  the dominant failure of the noise approach.
- Tunable parameters map to visible features (ridge spacing, curve rate,
  meander, crest height) instead of abstract octaves, making the systems
  tractable to tune.
- Rounded slopes remove the blocky look; sharp cliffs with a lip make
  passable-vs-impassable legible through geometry and lighting.
- Connectivity remains watertight by the existing shared-corner
  construction; re-bakes stay byte-identical (field and skeleton are pure
  functions of position/seed).
- No new `sim` dependency; passability stays the single tendril.

**Negative:**

- The hill direction field is a new tuning risk (corduroy if too slow,
  chaos if too fast); mitigated by a gated spike (R3.0) before commit.
- Hill corrugation is higher spatial frequency than a massif and may set
  the vertex budget; measured at R6, helped by adaptive subdivision.
- Mountain interior variants are an authoring cost, and repetition must
  be watched across large ranges.
- The displacement decision has now reversed twice; this ADR records the
  principle (different connectivity semantics) that explains both
  failures, to stop a third iteration from repeating them.

## Alternatives considered

- **Keep noise, tune harder.** Rejected — structurelessness is inherent
  to noise; tuning cannot add large-scale organization.
- **One structured-relief mechanism for both** (raised corners/edges for
  hills and mountains alike). Rejected — raising shared height for hills
  reads as a tier change, not rolling terrain.
- **Authored variants for hill edges** (Wang/corner-tile edge matching).
  Rejected — combinatorial authoring (3⁶ neighbor configurations) and
  edge-match bookkeeping for what is fundamentally a texture; the
  directional field gets cross-tile continuity for free.
- **Authored center variants as the primary mechanism** (peaks at tile
  centers). Rejected for mountains — center peaks are not shared, so
  ranges fragment into one-bump-per-tile; the dominant relief must live
  on the shared corner/edge skeleton. Variants retained only as secondary
  interior character.
- **A passable "steep slope" class between slope and cliff** (cliffs only
  at delta ≥ 3). Rejected — splits every passability consumer
  (pathfinding, movement cost, AI, UI) into three cases for a visual
  nicety, and weakens the passable/impassable signal. Two classes, one
  threshold in `edge_class`.
- **Explicit mountain ridge-spine extraction** (skeletonize connected
  mountain components into meshed polylines). Still rejected as in
  ADR-0026 — the crest emerges from raising shared corners/edges of the
  existing lattice, without a separate extraction subsystem.
- **Subdivision alone to round corners.** Rejected — the blockiness is
  the hard normal crease at the ramp/flat join, not the triangle count;
  subdivision without a smooth height profile costs more and looks the
  same.

## References

- ADR-0026 (terrain renderer rework — displacement-regime portion
  superseded; corner-legibility decision narrowed; remainder stands)
- ADR-0012 (cliff impassability; design doc §1.4 delta ≥ 2 rule)
- ADR-0025 (`StaticTerrain`)
- `documents/planning/rendering-rework-plan.md` and its R3 revision
- `documents/planning/elevation-transition-methodology.md`
