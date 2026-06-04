# Resource Economic Model

This document owns the **economic role** of resources: how the deposits that
`worldgen` places on the map (`worldgen.md` §1.13, ADR-0017) become yields,
materials, amenities, and currency in the running game. It bridges three other
documents and defers to them where they are authoritative:

- **Placement** (which deposits exist, where, how rare) — `worldgen.md` §1.13,
  ADR-0017. This document does not re-decide placement.
- **Consumption by production** (the staffed-work economy, the materials pool) —
  `cities.md` §2.7.
- **Unit resource gates** (chassis/components) — `combat.md` §3.13.

The numbering here is doc-local; assign a global chapter number when slotting it
beside `worldgen.md` (§1), `cities.md` (§2), and `combat.md` (§3).

---

## 1. Economic roles over the generation categories

**Decision:** The three generation categories (Strategic, Luxury, Bonus —
ADR-0017) are placement behaviors, not economic roles. The economic model maps a
role onto each category rather than introducing a fourth taxonomy:

- **Bonus → the always-available floor.** Bonus resources split by what they
  feed, not by category. Food bonuses (wheat, fish) feed the population/growth
  loop (`cities.md` §2.6). Material bonuses (stone; plus lumber, which is a
  forest feature rather than a placed deposit) feed the base building-materials
  pool. Uniform scatter plus terrain eligibility (ADR-0017) means every player
  has enough of the floor to follow the critical path.
- **Strategic → the higher rungs of the materials ladder, which are also the
  unit gates.** A strategic resource *is* a higher material tier (Section 2) and
  the access requirement for the corresponding component tier (Section 3). There
  is no separate "unlock" mechanic distinct from "do you have the material."
- **Luxury → amenities, plus a differentiated secondary yield.** Luxuries drive
  the amenity system (Section 5) and each also carries one secondary yield —
  food, materials, or currency (Section 4). Their plate-clustered placement
  (ADR-0017) produces the geographic monopolies that motivate trade.

**Rationale:** The generation categories were designed for *how* a deposit is
distributed; the economy needs *what a deposit does*. Mapping roles onto the
existing categories avoids a parallel classification that would drift from the
generation one. The mapping is also where `cities.md` §2.7's "base building
materials + distinct higher-tier tracking" requirement is satisfied: Bonus is
the base pool, Strategic is the distinctly-tracked higher tiers.

**Risks:**
- The Bonus category mixes food and materials, so "Bonus" alone never tells you a
  resource's economic role; the role lives in this document's per-resource list
  (Section 8), not in the generation category.

**Mitigations:**
- The per-resource economic role is data, recorded in Section 8 and ultimately in
  the content registry, separate from the placement category in `resources.ron`.

---

## 2. The building-materials ladder and era alignment

**Decision:** Building materials form a tiered ladder. The base tier is always
available; higher tiers are strategic resources that gate strength and
efficiency, never progression (the `cities.md` §2.7 hybrid rule, realized here):

| Tier | Material | Source | Era |
|------|----------|--------|-----|
| Base | Lumber, Stone | Forest feature; Bonus deposit | Always |
| Higher | Copper | Strategic deposit | Ancient (Bronze) |
| Higher | Iron | Strategic deposit | Classical (Iron) |
| Higher | (Aluminium, Titanium) | deferred | Far future |

The base tier collapses into a **single building-materials quantity** (lumber and
stone are not tracked separately; terrain sets the rate). Higher tiers are
**tracked distinctly**, because they gate specific component tiers and are
consumed when building those components (Section 3).

**Rationale:** The ladder maps cleanly onto era progression — base materials
everywhere, copper in the Bronze Age, iron in the Iron Age. This is why the
Ancient era takes **copper**, not iron: iron is the Classical-era material by
the project's own era naming, and `combat.md` §3.13 commits the Ancient era to a
single bronze augment tier. Adding copper is a content addition (a `ResourceId`
variant and a `resources.ron` entry), not an algorithm change (ADR-0017), and it
moves iron into its correct era rather than leaving the Bronze Age with the wrong
defining metal.

**Open questions:**
- Whether the base pool stays a single quantity forever or splits when later
  systems (e.g. construction speed by material) need lumber and stone to differ.
  Single pool is correct for the Ancient and Classical eras.

---

## 3. Strategic resources: consumed flow with access as the gate

**Decision:** Strategic resources are a **consumed flow**, not binary access. A
worked deposit (Section 7) yields a per-turn flow of the strategic material;
building a component or building that requires that material consumes it from the
flow/stock. **Access is the gate by construction**: a player who works no copper
deposit has zero copper and therefore cannot build bronze components — without a
separate binary-access mechanism.

The hybrid gate from `cities.md` §2.7 is realized through `combat.md` §3.13: the
base chassis requires only base-tier materials and is always buildable; the
bronze component requires copper. Some buildings may likewise require a higher
tier; those on the critical path never do. ADR-0017's baseline placement (iron at
60/40, copper to be tuned similarly) guarantees the critical path is never
bricked — an unlucky start is weaker, not stuck, and trade is its path to parity.

**Rationale:** Flow falls out of the `cities.md` §2.7 pool model for free, so no
new primitive is introduced (pillar 0.5). "Zero if unworked" *is* the access
gate, so the binary mechanic is redundant. Flow also scales naturally into later
eras where strategic materials carry ongoing upkeep (the supply model, `combat.md`
§3.11), whereas binary access would have to be replaced.

**Risks:**
- If a tech or era transition is ever gated behind a building that requires a
  higher-tier strategic, the no-lockout guarantee breaks. The critical path must
  stay base-tier-buildable end to end.

**Mitigations:**
- Section 8 fixes the invariant: progression (era transitions, the tech critical
  path) never requires a strategic resource. Strategics gate strength, not
  progress. This is cross-referenced from the tech tree when it is designed.

**Rejected alternative — binary access (Civ-style):** Have iron → can build,
quantity irrelevant. Rejected because it is a second mechanic doing what "zero if
unworked" already does, and because it does not extend to the upkeep model later
eras need.

---

## 4. Luxuries: differentiated yields

**Decision:** Each luxury carries, in addition to its amenity contribution
(Section 5), exactly one **secondary yield** — food, materials, or currency.
Yields are differentiated per luxury (tin yields materials; precious-metal
deposits yield currency; etc.). Amounts are tuned later; the structural rule is
that every luxury has both an amenity role and a yield role.

**Rationale:** Differentiated yields make luxuries *not interchangeable for
yields*, only for amenities, which gives the duplicate-luxury rule (Section 5) a
reason for duplicates to exist: the first copy of a luxury grants amenity plus
yield, every duplicate grants yield only. Without yields, a duplicate luxury is
worthless. This also lets **tin** carry Bronze-Age trade flavor as a
materials-yielding luxury without making it a second bronze gate — copper remains
the sole hard gate (Section 3), so no unlucky start is bricked by lacking tin.

**Risks:**
- Severing tin from copper loses the historical reality that bronze *is* copper
  plus tin; tin becomes "a luxury that yields materials" rather than "the scarce
  half of bronze."

**Mitigations:**
- Accepted for Ancient-era simplicity. If the realism is wanted back without
  re-gating, add a small copper+tin **network synergy** (holding both grants a
  bronze-efficiency bonus) as a later refinement. Do not build it now.

**Open questions:**
- Naming collision: there is a luxury deposit named *Gold* (precious metal, from
  `Precious` mineral character) and a currency also called *gold* (Section 6).
  Resolve by renaming one (e.g. luxury "gold ore", or currency "coin"/"wealth")
  before the registry locks string keys.

---

## 5. The amenity system

**Decision:** Amenities are a **supply-versus-demand balance** resolved per city,
producing a population-growth modifier.

- **Supply.** Each *distinct* luxury available to a city provides a fixed amenity
  amount (1, tuned later). Duplicates provide no additional amenity (they still
  provide their yield, Section 4). Supply is the count of distinct luxuries the
  city has access to, plus later-era building sources.
- **Demand.** Each city's amenity demand scales with its population.
- **Balance.** Supply minus demand is a surplus or a deficit. Surplus grants a
  population-growth bonus; deficit grants a growth penalty. Both directions are
  built from the start; the deficit penalty is tuned weak initially.
- **Distribution.** A luxury's amenity reaches **all the owner's cities**
  (empire-wide) for now. The trade-network-gated version arrives with the trade
  system (Section 7, open).

**Rationale:** One-amenity-per-distinct-luxury with duplicates giving nothing is
the standard model and rewards *variety*, which is what makes trade attractive
rather than hoarding. The supply-versus-demand balance is what makes the system
*exist* in the Ancient era while only *biting* in the Medieval era: Ancient
populations are small, so demand is low and cities sit in surplus almost
automatically; as populations climb in later eras, demand outruns luxury count and
the player must scramble for amenity sources. The difficulty emerges from the
population curve, not from new rules bolted on later — the same system throughout.

Building the deficit penalty in from the start (rather than shipping an
upside-only growth bonus and adding teeth later) is what makes this "the system
exists in the Ancient era" rather than half of it. A pure bonus never creates the
pressure the design depends on.

**Risks:**
- Empire-wide distribution is more generous than the network-gated end state; cities
  far from a luxury source benefit with no logistics cost until trade exists.

**Mitigations:**
- Accepted as interim. The Ancient era has no road or trade-route system to be
  "on" (those are Classical-plus), so empire-wide is the simplest rule that works;
  it tightens to network-gated when trade is designed, which raises difficulty in
  the same direction the population curve already pushes.

**Open questions:**
- Per-luxury amenity amount, demand-per-population curve, and the growth
  bonus/penalty magnitudes. All tuning; deferred. The deficit penalty stays weak
  until later eras.

---

## 6. Gold: empire-wide currency, yield-only stub

**Decision:** Gold (currency) is an **empire-wide** accumulating resource. Some
luxuries yield gold (Section 4). In the Ancient era gold has **no sink** — it
accumulates and is not yet spent. This is a deliberate stub: the accumulator and
its yield plumbing are wired now because they ride the same yield pipe as food
and materials, but the sink design (maintenance payable in gold, unit upkeep,
purchasing) is a later system.

**Rationale:** Wiring gold alongside food and materials is nearly free — one
accumulator on an existing pipe. Empire-wide is the correct scope for currency:
it is fungible and pools at the polity level, unlike materials, which are bulky
and (provisionally) local. Deferring the sink avoids inventing a treasury balance
the Ancient era does not need.

**Scope asymmetry (recorded, not accidental).** Gold is empire-wide; building
materials and food are per-city (or per-region — still open, `cities.md` §2.7).
Two scope models coexist in one economy by design: currency pools, bulk goods do
not. This is cross-referenced in `cities.md` §2.7 so no one implementing later
infers that materials follow gold to empire scope.

**Open questions:**
- The entire sink side (recorded as a known-issues stub). An accumulator with no
  outflow looks finished and is not; the sink is a real later system.

---

## 7. The deposit-to-yield bridge

**Decision:** A placed deposit produces nothing until it is **worked**. A worked
tile — via the tile-improvement / district mechanism (`cities.md` §2.13) — turns
a deposit into a per-turn yield into the relevant pool: a mine on copper yields
copper, a farm on wheat yields food, and so on. Generation places *potential*;
the economy realizes it through deliberate development.

**Rationale:** This is the connection between the placement layer (deposits on
tiles) and the economy layer (flowing pools, `cities.md` §2.7). It keeps resource
value tied to development investment (§2.6) rather than granted by mere
ownership, consistent with the brainstorming rejection of "place a mine anywhere
and get production."

**Dependency this surfaces:** The worker / tile-improvement verb set is the
connective tissue and is currently undesigned. It moves up the dependency chain
as the next system after this document: without it, deposits cannot become yields
and none of Sections 1–6 produce anything.

**Open questions:**
- Trade-network connectivity (which determines amenity reach, Section 5, and
  future inter-city resource sharing) is undesigned and Classical-plus. Until it
  exists, amenity distribution is empire-wide and materials are local.

---

## 8. Ancient-era resource list

**Decision:** The Ancient-era economic picture, by role:

- **Strategic — Copper** *(new content; add to `resources.ron` and `ResourceId`)*:
  the Bronze-Age higher-tier material. Consumed flow; access gates bronze
  components (`combat.md` §3.13). Baseline placement tuned to guarantee no
  lockout.
- **Strategic — Iron, Horses** *(already placed)*: placed by generation but
  economically Classical. Iron is the Iron-Age material tier; horses gate a
  Classical mounted archetype (the Ancient unit set has no mounted unit — confirm
  or add a mounted scout if one is wanted). Neither has an Ancient-era economic
  role beyond being present on the map.
- **Bonus — Stone, Wheat, Fish** *(already placed)*: the floor. Stone feeds base
  materials; wheat and fish feed food. Lumber feeds base materials as a forest
  feature, not a deposit.
- **Luxury — Gold deposit, Gems, Spices, Wine, Tin** *(tin is new content)*:
  placed and economically active in the Ancient era. Each provides one amenity
  (empire-wide, Section 5) and one secondary yield (Section 4): tin → materials;
  gold deposit → currency; the others' yields tuned later (food or currency).

**Progression invariant:** No Ancient-era progression step — no era transition,
no tech on the critical path — requires a strategic resource. Strategics gate
strength and efficiency only (Section 3). Cross-referenced from the tech tree when
designed.

**Content additions required:** `Copper` (Strategic) and `Tin` (Luxury) as
`ResourceId` variants and `resources.ron` entries; placement rules follow the
existing category structure (ADR-0017), so no algorithm change.

---

## Cross-references

- `worldgen.md` §1.13, ADR-0017 — resource placement (authoritative for where
  deposits go).
- `cities.md` §2.6 — population/growth loop (consumes food, consumes amenity
  growth modifier).
- `cities.md` §2.7 — staffed-work economy, materials pool, gold scope asymmetry.
- `cities.md` §2.13 — tile improvement / district mechanism (the deposit-to-yield
  bridge).
- `combat.md` §3.13 — chassis/component model (realizes the strategic gate).
