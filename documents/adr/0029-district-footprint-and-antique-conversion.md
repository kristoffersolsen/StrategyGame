# ADR 0029: Multi-tile fixed-shape districts with era-scaled footprint and antique conversion

**Status:** Proposed
**Date:** 2026-06-20
**Deciders:** Kristoffer

## Context

`cities.md` §2.13 originally modeled districts as single-tile placements within a
gated radius. Enumerating the Ancient content forced the district spatial model to
be made concrete: how large a district is, how it is placed, how it changes across
eras, and what happens to districts the player never modernizes. Several constraints
applied: the two-axis model (districts as built infrastructure vs. claimed tiles for
working) and its load-bearing efficiency asymmetry (§2.13); pillar 0.6 (modernizing
many districts must not become per-tile tedium); pillar 0.2 (era progression should
be discovered and chosen, not auto-applied); pillar 0.4 (deterministic placement);
and the project's migration-safety discipline (a data-model change should not orphan
existing saved state).

A key tension: an auto-growing district (anchor tile plus organic expansion) is the
more flexible model, but it is also the harder one to *tighten* later without
orphaning data, whereas a fixed-shape model is the easy one to *loosen* later. The
direction of safe future change drove the choice.

## Decision

**Districts are fixed multi-tile footprints**, placed by the player as a unit:

- A district occupies a **fixed compact footprint** (rhomboid/triangular), sited
  within the center's gated radius. The radius bounds the available *tile area*, not
  a count of single-tile districts. The player sites *what and roughly where*; the
  shape is fixed (compact shapes keep orientation near-irrelevant).
- **Footprint size is an era stamp**, and bumps only in that type's **focus era**,
  so no single era boundary forces a whole-city rebuild:
  - Housing — **1 tile, every era** (in-place tier modernization; the jigsaw filler).
  - Production — **2** (Ancient) → 3 (Industrial) → 4 (Modern).
  - Military — **2** (Ancient) → 3 (Medieval) → 4 (Modern).
- **Modern buildings need modern districts:** an advanced/era building is locked to
  its era-tier district (ADR-0028), so hosting it requires rebuilding the district
  at the larger footprint. The larger footprint is worth building for exclusive
  access to the era's advanced buildings, not for a linear slot increase. Existing
  districts keep working at their tier until the player chooses to modernize;
  nothing is auto-upgraded or stranded.
- **Cost asymmetry:** clearing an obsolete district is cheap; the larger replacement
  costs full price. The player is taxed for the *upgrade*, not for *having* an old
  district.
- **Terrain eligibility:** a footprint tile may sit on any terrain **except
  mountains and open water**; **cliffs do not block** (a footprint may straddle a
  cliff edge across height tiers). Permissive eligibility is a deliberate mitigation
  of the placement jigsaw the fixed-shape choice accepts.
- **Demotion is atomic:** a district whose footprint leaves a reduced radius
  (`cities.md` §2.12) is orphaned whole; there is no partial-tile shrink.

**Antique districts:** a district left un-modernized across eras converts to an
**antique** state yielding **culture (and later tourism)** instead of its low-tier
output; longer neglect yields more. The choice is where limited modernization
throughput goes each era — a genuine fork, not a chore. **Balancing invariant:**
antique yield **compensates for, but never exceeds in expectation, forgone modern
yield**. It is cross-currency, so the magnitude cannot be tuned until culture and
tourism exist. Conversion granularity may be whole-district or per-orphaned-tile
(the dense tile-state model supports per-tile era/yield state). Modernization is
issued as an empire-level directive with per-settlement override (§0.6, `cities.md`
§2.11), not a per-district manual chore.

## Consequences

**Positive:**

- Fixed shapes are migration-safe: loosening to anchor-plus-growth later never
  orphans existing fixed-footprint data, whereas the reverse tightening could.
- Era-scaled, focus-era-staggered footprints make modernization a paced, opt-in
  decision instead of a cliff at every era boundary.
- "Antique" turns the neglected-district case from a regret into a strategic asset,
  amplifying emergent narrative texture (the grave-chronicle tone) and rewarding
  discovery (pillar 0.2).
- Permissive terrain (cliffs allowed) reduces the jigsaw friction of fixed shapes
  and yields terraced settlements straddling height tiers.

**Negative:**

- Fixed shapes are less flexible than free per-tile packing; tight or irregular
  terrain can make placement a puzzle. Mitigated by compact shapes, 1-tile housing
  filler, and permissive terrain.
- The antique balancing invariant is cross-currency and therefore untunable now; it
  is recorded as an invariant and deferred, which is a standing open tuning risk.
- Per-tile antique state adds tile-state surface area (acceptable given the dense
  field model).

## Alternatives considered

- **Auto-growing districts (anchor + organic expansion):** the more flexible model,
  rejected as the *first* model on migration-direction grounds — it is the hard one
  to tighten later without orphaning saved district state. Fixed-shape-first
  preserves the option to loosen toward auto-grow later; the reverse is not safe.
  Explicitly revisitable.
- **Free per-tile district packing (Civ-6 style):** rejected. It maximizes
  placement micro and adjacency min-maxing, against pillar 0.6, and makes era-scaled
  footprints incoherent.
- **Mandatory modernization (old districts simply become dead weight):** rejected in
  favor of antique conversion, which makes the neglected case a real choice rather
  than a punishment or a forced chore.
- **Per-district manual modernization:** rejected as tedium (pillar 0.6); replaced
  by the empire-level directive.

## References

- `cities.md` §2.13 (districts, era-size ladder, antique), §2.12 (demotion,
  construction), §2.11 (modernization directive), §2.6 (growth loop).
- ADR-0028 (advanced buildings district-locked; the reason a larger footprint is
  worth building).
- ADR-0030 (the supply network that constructs and modernizes outpost districts).
- Design pillars 0.2 (knowledge as progression), 0.4 (determinism), 0.6
  (intent-level specification).
