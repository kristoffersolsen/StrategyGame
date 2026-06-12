# Rendering Rework Plan — R3 Revision (Landforms)

**Status:** Planned
**Replaces:** the original R3 ("Displacement regimes and baked normals")
in `rendering-rework-plan.md`.
**Supersedes:** the displacement-regime decision in ADR-0026, and narrows
its corner-legibility decision (see ADR-0027).
**Standing:** R1 (corners), R2 (lattice), R4 (biome blend / triplanar)
are implemented and stand. This revision replaces only the R3
displacement step and adds a transition-detail refinement that touches
R2's bridge/corner geometry.

---

## Why R3 is being reworked

The implemented R3 displaced subdivided cores with seeded noise — low
amplitude for hills, high for mountains. It reads as random noise, not as
landforms, and no parameter tuning fixes it: fBm is structureless by
design (octaves add crinkle, not large-scale organization), and a *shared*
noise field gives neighboring tiles correlated crinkle, not connected high
ground. Correlation is not connectivity. The original ADR-0026 bet that
ranges would emerge from a shared noise field; they did not.

The replacement is built on the principle that finally explains the
failure: **mountains and hills have different connectivity semantics, so
they need different mechanisms.**

- A mountain range's connectivity *is* continuity of height — clustered
  mountains share high ground and the mean elevation rises where they
  cluster. The right mechanism raises the shared corners and edges of
  clustered mountain tiles.
- A hill field's connectivity is continuity of a *pattern at constant
  mean height* — rolling ridge-and-swale lines that flow across tile
  boundaries while the ground level stays put. Raising shared corners
  here would lift the mean and read as a tier change. The right mechanism
  is a shared directional ridge field, not shared height.

A single amplitude-driven mechanism was wrong twice (first that noise
would connect, then that one mechanism served both). The two are now
separate systems. Noise is demoted to a low-amplitude top detail layer on
both — garnish to break up interpolated smoothness, never structure.

All landform classification (crest, ridge, field direction) is
presentation, derived in `rendering` from the discrete mountain/hill
masks. No new tendril into `sim`; `edge_class` remains the only one.

---

## R3.0 — Hill direction-field spike (gate before committing)

The hill ridge field is the highest-uncertainty piece, so it is
de-risked in isolation before the full rework, the way R1 de-risked the
corner. Build the directional ridge field on a synthetic hill patch only:
no mountains, no transition rework, current cliff/slope geometry left as
is.

The field is a **hybrid direction source**: elevation aspect (the
downhill gradient of the tier field the renderer already reads) where
regional slope is defined, low-frequency perturbation on flat regions and
as variation throughout, domain-warped so ridges meander rather than
running as straight corduroy. A low-amplitude ridged profile advances
across the field to produce several flowing ridges per tile at constant
tier height. Amplitude persists across hill→hill edges and fades to zero
at hill→non-hill edges.

Tunable parameters map to visible features (this is why it is tractable
where noise was not): ridge wavelength = spacing, field rate = how fast
ridges curve, warp strength = how much they meander, amplitude = ridge
depth.

**Gate:** a field of hill tiles reads as flowing, connected rolling
terrain — multiple ridges per tile, ridge lines continuous across tile
seams, mean height visibly unchanged (no plateau/tier-change
appearance). Adjacent hill clusters do not read as elevation steps.
Failure modes checked explicitly: no artificial corduroy (field varying
too slowly), no chaos (too fast). Visual sign-off recorded. `check.sh`
passes.

If the spike fails to produce legible flowing hills, R3 stops here and
the approach is reconsidered before any further work — this gate exists
precisely to catch that cheaply.

---

## R3.1 — Hills: directional ridge field (full)

Promote the spike to the real renderer: integrate the ridge field into
the chunked core generation for hill tiles, watertight across chunk seams
(the field is a pure function of world position, so independent chunks
agree by construction). Add the low-amplitude noise detail layer on top.
Recompute per-vertex normals from the final surface.

Note for the perf gate: several fine ridges per tile is higher spatial
frequency than one massif, so hill cores may need *more* subdivision than
mountains to resolve the corrugation. Hills, not mountains, may set the
vertex budget — measure at R6.

**Gate:** hills in a full generated map read as flowing rolling terrain
with cross-seam continuity; lighting reveals the ridge form; re-bake of a
chunk is byte-identical; hill→non-hill edges fade cleanly with no wall.
`check.sh` passes.

---

## R3.2 — Mountains: corner-and-edge crest skeleton

Extend the corner-height function (R1/R2 machinery) so that a corner
where several mountain tiles meet is elevated toward peak height, a
corner where mountains meet lower land is a shoulder/saddle (partway),
and a corner among low land stays at tier height. **Raise the shared edge
between two mountain tiles** as well, turning the sagging bridge midpoint
into a crest segment connecting the two raised corners. A mountain
cluster's internal corners and edges then form a connected graph of
raised crests — a massif with ridgelines — watertight by the same
shared-corner construction as R2. Mountain-to-lower edges stay unraised,
falling away as flanks and handing the drop to the R1/R2 transition.

Author 3–4 mountain **interior** variants that modulate the fill *under*
the crest graph — asymmetry, cirque-like concavities, subsidiary bumps,
flank steepness — selected deterministically per tile and applied with
hex rotation/reflection. Variants are secondary character; the crest
skeleton owns the dominant relief. (Apparent variety comes mostly from
the edge-configuration of the crest graph plus rotation, so 3–4 base
interiors suffice.) Add the low-amplitude noise detail layer; recompute
normals (averaged on flanks, harder along crests).

Constraint: where a mountain tile borders a lower one, above-tier relief
must stay within budget so it does not visually erase the cliff/lip and
the tier legibility from R1.

Hills get no authored variants — a directional dome would re-introduce
the singular-mound / plateau problem the field was chosen to avoid.

**Gate:** contiguous mountain tiles read as connected ranges with
ridgelines, not isolated peaks; isolated single mountain tiles still read
as a peak; flanks fall to neighbors without erasing cliffs; variant
repetition is not obvious across a range; re-bake byte-identical.
`check.sh` passes.

---

## R3.3 — Transition detail: rounded slopes, vertical cliffs

This sub-stage refines the R2 bridge/corner geometry. It separates two
legibility concerns onto two mechanisms: **tile identity** moves to a
future grid overlay (out of scope here), and **passability** stays in the
terrain geometry as the rounded-vs-vertical distinction. This narrows the
R1 "preserve tier boundaries at corners" decision: sharp at cliff
corners, relaxed (rounded) at slope corners.

The slope/cliff distinction is read from `edge_class` — the renderer owns
no threshold. `Cliff` (tier delta ≥ 2, impassable per ADR-0012) and
`Slope` (passable) map directly to geometry, so the visual cannot diverge
from the mechanic.

**Rounded slopes.** Blockiness is not primarily the single large corner
triangle — it is the hard normal crease where a linear ramp meets the
flat core. Subdivision alone keeps that crease and just costs more.
Replace the linear ramp with a smoothstep / Hermite height profile: flat
near each core, steepest in the middle, eased (rounded) shoulders into
the flats top and bottom. Subdivide to resolve the curve, then recompute
averaged normals so lighting shows the roundness. The corner patch
inherits each of its three edges' treatment — round toward slope edges,
sharp toward cliff edges.

**Vertical cliffs.** A cliff edge reuses the old stepped side-face
geometry: a near-vertical face (battered a few degrees off true vertical
so it shades and avoids z-fighting) with a **hard top lip**. The lip
carries the "cliff" read more than the verticality does. Face normals on
the cliff; this reinforces passable-vs-impassable through lighting, not
just silhouette. Aligns with R4: vertical faces take the triplanar path
(no UV stretch), gentle slopes stay on the cheap planar path.

**Adaptive subdivision.** Subdivide where rounding pays (slopes, hill
cores); keep cliffs near-flat (vertical face + lip needs almost no
subdivision). Subdivision count **must match along every shared edge**,
including across chunk seams — a 4× bridge meeting a 2× corner T-junctions
and cracks. Edge subdivision density is therefore a pure function of the
shared edge, agreed by both sides the same way corner positions are.

**Gate:** slope transitions read as rounded under lighting (no hard
shoulder crease); cliff edges read as vertical with a distinct lip;
passable and impassable edges are visually distinguishable at a glance;
no cracks or T-junctions at any subdivision-density boundary or chunk
seam; cliff faces show no UV stretch (triplanar), slopes use planar.
`check.sh` passes.

---

## Risks specific to this revision

**The hill field is the new tuning risk.** Trades noise-tuning for
field-tuning, but failure modes are legible (corduroy / chaos) and
parameters are landform-meaningful, unlike fBm. R3.0 exists to surface
this before the full commit.

**Hills may set the vertex budget.** Fine corrugation is higher frequency
than a massif. Measure at R6; adaptive subdivision (R3.3) helps by not
over-subdividing cliffs.

**Mountain relief vs. R1 legibility.** Above-tier crest relief at
mountain↔lower borders must stay within budget so it does not erase the
cliff lip. Constrained in R3.2, checked at its gate.

**Variant repetition.** Mitigated by hex rotation/reflection and
edge-configuration variety; checked visually at the R3.2 gate. If 3–4
interiors still repeat visibly across large ranges, add one or two more
rather than increasing per-tile complexity.

---

## Open decisions (resolve at the stage)

- Ridge wavelength, field rate, warp strength, amplitude (R3.0/R3.1) —
  tuned visually against the legible failure modes.
- Number of mountain interior variants beyond the initial 3–4 (R3.2) —
  add only if repetition is visible.
- Smoothstep vs. Hermite (and shoulder easing strength) for slope
  profiles (R3.3) — tuned visually.
- Cliff batter angle and lip height (R3.3) — tuned visually; lip presence
  matters more than exact values.
