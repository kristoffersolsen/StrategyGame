# Implementation Plan — Replanned Stages (3.6, 4–7)

This file replaces the Stage 4+ placeholder section of
`implementation-plan.md`. It is the output of the post-Stage-3 replanning
described in the document's replanning protocol. Stages 3.1–3.5 and the
rendering plan's P1–P3 are complete; this plan carries the project from
"the map renders" to "the first turns are playable: a settler founds a
city."

The principles from the top of the implementation plan still hold and are
not repeated here. Two of them do most of the work in this section:
*infrastructure before features* (the runtime data model precedes the
systems that mutate it; the command and turn backbone precedes the units
that issue commands) and *defer the undecided* (city economics beyond
founding is blocked on open design and is explicitly held out of scope).

---

## What changed since the placeholders were written

The original placeholders assumed Stage 4 would be "Bevy app setup" and
Stage 6 would spawn a tile entity per tile with click-to-select. Both
assumptions are now wrong and the replan corrects them.

The Bevy app, plugin architecture, chunked rendering, 3D scene, and
camera already exist — that was the rendering plan's P1–P3 work, which
ran ahead of the original staging. So there is no "game scaffolding"
stage left to do; the game crate is already a running Bevy app. What does
*not* yet exist is everything dynamic: a mutable runtime world model,
the command layer, the turn state machine, units, and cities. The
renderer today knows only how to draw static terrain.

The per-tile-entity assumption in the old Stage 6 is reversed (see the
runtime tile model decision below). Tiles are not entities. This is the
single most consequential decision in the replan and it shapes every
stage that follows.

---

## Cross-cutting decisions made during replanning

These four decisions are cross-cutting (they affect multiple modules and
the multiplayer-determinism pillar) and each gets an ADR before the stage
that depends on it. They are recorded here so the staging reads against a
fixed set of choices rather than re-litigating them per stage.

**Runtime tile model — dense grids, not entities (ADR-NNNN).** Dynamic
per-tile state lives in a runtime resource as a set of parallel
`Field<T>` grids (owner, occupant, improvement, infrastructure, cleared
flags), sized for the whole map at world load. Tiles are *not* ECS
entities. Only units and cities are entities. The decisive argument is
mid-to-late-game scaling: a developed map has most of its land altered,
and per-tile entities pay archetype churn and change-detection cost in
exactly that state, for no benefit — tiles are read by coordinate and by
neighbourhood (coordinate math), not iterated as a query. Dense grids
make altered and unaltered tiles cost identically: building an
improvement writes one enum value, with no allocation, no archetype move,
and no per-entity overhead. Whole-map state for the default map is on the
order of ~1 MB, contiguous and cache-friendly, flat regardless of how
developed the map becomes. This reuses the `Field` idiom the worldgen
crate already established. The constraint that makes the model simple —
**at most one player-built improvement per tile** — is locked
deliberately, not incidental; stacked improvements would reintroduce
per-tile interaction resolution and are out of scope.

**Cross-tile references are `NetworkId`, not `Entity` (ADR-NNNN).**
`Entity` indices diverge across machines, which is the reason `NetworkId`
exists in the design. All authoritative cross-tile references — a tile's
owning city, a tile's occupying unit — store the referent's `NetworkId`.
A derived `Entity` lookup cache rebuilt from `NetworkId` is permitted as a
non-authoritative convenience; it is never serialized or synced. This
requires a deterministic `NetworkId` allocator: a counter in a resource
advanced in command-processing order, never random and never derived from
an `Entity` index.

**Reaction mechanism — buffered events, observers view-only.** Game-state
changes are announced via buffered, past-tense Bevy `Events`
(`UnitMoved`, `CityFounded`, `TileImproved`) with explicit schedule
placement, not Bevy observers/`Trigger`. Buffered events with deterministic
ordering are replay-friendly and respect the determinism pillar;
observers run immediately and introduce ordering subtleties that fight it.
Observers are reserved for view-only concerns (UI, audio) if used at all.
This pairs with the tile-model decision: because per-tile change detection
is not the reaction path, giving up `Changed<T>` on tiles costs nothing.

**Yield system — additive base plus single transform (ADR-NNNN).** A
tile's yield is computed by a single shared function,
`tile_yield(coord)`, identical for owned and unowned tiles. The base is
purely additive — biome yield + feature yield + resource yield, three
enum-indexed array lookups summed. At most one player-built improvement
then transforms that base. Base values are content/balance data loaded
from a RON asset at startup (same pattern as `biome_table.ron`) into a
sorted, enum-discriminant-indexed array for O(1) lookup; the transform
resolution is a small function, not data. Cities memoize this function
over their owned tiles and recompute on change; the worker-radius view
calls it over a small neighbourhood; the full-map yield overlay calls it
on demand and caches per chunk for the overlay's lifetime, invalidated by
change events — never run per frame and never per turn. Two variables are
explicitly deferred to the (undesigned) resource and city-economy work and
do not block the architecture: whether resources yield passively or only
when improved, and whether the improvement transform is a percentage of
the base or a flat conditional addition. The model holds either way.

---

## Stage 3.7: Content registry and configuration consolidation

**Goal:** Convert the closed content enums (`ResourceId`, `Biome`, and
their kin) to a string-keyed registry so content kinds are open to data
mods; move hardcoded worldgen defaults into RON; and audit the workspace
for loose parameters that should be configuration. Done now, before any
runtime system is built on these types, because the conversion is far
cheaper against the frozen Stage-3 pipeline than after Stages 4–7 depend
on the current closed representation.

This stage is sequenced **before Stage 3.6** deliberately: 3.6 reads
resource and biome data, so converting against a pipeline that does not
yet include 3.6 keeps the conversion's moving parts to a minimum. The
determinism scaffold is the correctness check throughout — the same seed
must produce byte-identical `WorldData` before and after the conversion,
which is exactly the guarantee that proves handle assignment is
deterministic.

### Why open kinds change what a content id *is*

A closed enum requires its variants to be known at compile time; that is
what makes content kinds un-moddable without a recompile. Opening them
means kind *identity* moves out of the type system and into data: a
stable string id (`"iron"`) is the authoritative identity that crosses
saves and the network, and the runtime representation becomes a `u32`
handle that is a dense index into a registry built at load. The handle is
derived, never serialized. This mirrors the `NetworkId`-vs-`Entity`
decision one layer down: a stable authoritative identity, a derived
runtime index, and a hard rule that only the former is persisted or
synced. See ADR-NNNN.

The performance shape is unchanged from the enum: a handle-indexed array
lookup is the same machine operation as a discriminant-indexed one. Two
disciplines preserve that and are requirements of this stage, not
optimizations: string ids are resolved to handles **once at boundaries**
(content load, save load, network decode) and never carried into the
simulation; and any property read on a hot path is stored as a **dense
handle-indexed array** (e.g. an `is_strategic` bitset), not read through a
descriptor struct behind a pointer. Game logic branches on these
properties, not on kind identity — exhaustive `match` on kind is removed,
which is the open/closed principle applied as intended.

### 3.7.1 Shared registry crate

A new workspace crate (`content`, alongside `coords` and `det_rng`) owns
the registry types: the interned-handle id type, the registry container,
the string-id→handle resolution, and the deterministic index-assignment
rule (sort by string id, then assign dense indices). Placing it in its own
crate matches the existing structure — `worldgen` and `game` both
construct and query the registry, so it belongs below both rather than
inside one and re-exported. No project dependencies beyond what `coords`
already establishes.

### 3.7.2 Convert worldgen content enums

Convert `ResourceId`, `Biome`, and any other closed content enum in
`worldgen` to registry handles. The existing RON content files
(`resources.ron`, `biome_table.ron`) gain explicit stable string keys.
Worldgen logic that matches on these reads registry properties instead.
`MineralCharacter` and other purely-internal generation enums that are
**not** content kinds (no mod would add a mineral character; it is a
generation mechanism, not a content type) stay as enums — the conversion
is for open content kinds, not for every enum in the crate. Drawing that
line is part of the stage.

### 3.7.3 Worldgen defaults to RON

Move the hardcoded `Default` values for `WorldParams` and its sub-structs
(`TectonicsParams`, `ClimateParams`, etc.) into
`assets/worldgen/default_params.ron`, deserialized by the application
layer (`game` and the tools), not by the `worldgen` library — the library
continues to take a `WorldParams` struct and never touches the
filesystem, preserving its Bevy-free, standalone property. This also makes
the §1.15 three-layer parameter exposure implementable: Full Tunability is
editing these values, and the achievement-disable is "loaded values differ
from shipped defaults."

### 3.7.4 Loose-parameter audit

Sweep the workspace for parameters that have snuck in as hardcoded `const`
values or inline literals where they are really tuning knobs or content
data. A loose constant is a closed parameter that has not been noticed
yet — the same open/closed problem one level down, and cheapest to fix now
before the system grows. For each finding, decide deliberately: promote to
RON config (a tuning value), move to the content registry (content data),
or keep as a true constant (a genuine invariant — a mathematical constant,
a protocol width). Record the disposition; the point is that each is a
*decision*, not an oversight. Genuine constants stay; the audit's output
is that nothing is hardcoded *by accident*.

### Gate: Stage 3.7 complete

- `content` crate exists; registry types, deterministic index assignment,
  and string-id resolution implemented and unit-tested.
- `ResourceId`, `Biome`, and other content kinds are registry handles;
  content RON files carry stable string keys.
- Non-content enums (e.g. `MineralCharacter`) are explicitly retained as
  enums, with the content-vs-mechanism line documented.
- Hot-path properties are stored as dense handle-indexed arrays; string
  resolution occurs only at load boundaries (verified by review — no
  string lookup in any per-tile or per-turn path).
- Worldgen defaults load from `assets/worldgen/default_params.ron` via the
  application layer; `worldgen` remains filesystem-free.
- Loose-parameter audit complete; every finding promoted to RON, moved to
  the registry, or recorded as a deliberate constant.
- **Determinism: the same seed produces byte-identical `WorldData` before
  and after this stage.** This is the load-bearing gate — it proves handle
  assignment and registry construction are deterministic.
- ADR-NNNN written: content registry model (string-id authority, derived
  handles, property-based dispatch, the two hot-path disciplines).
- `documents/architecture/architecture.md` updated: the `content` crate is
  new structure, and the `assets/` config layout is recorded.
- Design doc §1.15 cross-reference noted: params-to-RON enables the
  three-layer exposure.

### Deferred from this stage (recorded, not built)

The mod-manifest and multi-bundle load/merge/order system is **not** built
here. Opening the kinds is the prerequisite; the bundle system has no
consumer until a second bundle exists (Stage 8 improvements is the first
realistic point). Building it now would be speculative structure — the
premature-abstraction trap. What this stage *does* guarantee is that the
representation it leaves behind (string-keyed, registry-resolved,
load-through-one-path) is one a manifest system can later sit on top of
without rework. The base game is structured so it can become the
lowest-priority bundle when that system arrives.

---

## Stage 3.6: Starting position selection (minimal)

**Goal:** Close the generation pipeline by selecting N start positions,
and give every later stage a real answer to "where does the first settler
spawn." This is a *minimal* real implementation, not the full design-doc
treatment — the audit/rebalancing pass (design §1.14) is deliberately
deferred.

This stage exists now, ahead of any runtime work, because a hardcoded
start tile is exactly the kind of intentional shortcut the project avoids,
and because this is the final exercise of the determinism scaffold against
the full pipeline.

### 3.6.1 Viability scoring

A single viability heuristic per land tile: a weighted score over the
local neighbourhood's food/production potential (from the yield base
table, which this stage is the first runtime-adjacent consumer of),
freshwater proximity (hydrology), and a hard adequacy floor that rejects
tiles whose neighbourhood cannot support a starting city. No audit pass,
no resource injection — a tile either clears the floor or it does not.

### 3.6.2 Selection with spacing

Select N positions greedily by descending viability subject to a minimum
spacing constraint, where N comes from `WorldParams`. Deterministic tie
handling (fixed iteration order, no `HashMap`).

### 3.6.3 Output and visualization

Add `start_positions: Vec<Coord>` to `WorldData`. Add a viability heatmap
and a selected-positions overlay to `mapdump` and `maptest` output, per
the per-stage pattern.

### Gate: Stage 3.6 complete

- `WorldData` carries `start_positions`; `generate_world` populates it.
- The adequacy floor holds across at least 20 seeds (no start below floor).
- Spacing constraint satisfied across the same seed set.
- `mapdump` produces viability and selected-position PNGs.
- Determinism test extended to cover `start_positions`; full-pipeline
  byte-identical output confirmed across runs.
- Design doc §1.14 updated to record that the audit pass is deferred and
  why; the deferral is logged, not silent.

---


## Runtime architecture: trust tiers and the headless core

This note governs how Stages 4–7 are structured internally. It is
cross-cutting rather than a stage of its own: each stage builds *within*
these boundaries, and the boundaries are the thing that keeps the
multiplayer-determinism pillar (design 0.4) enforceable as the runtime
grows.

### The organizing cut is trust, not code role

The tempting decomposition is by what code *does*: one part renders, one
simulates, one handles player input, a later one handles AI. That cut is
functional but it crosses the boundary that actually has to be
bulletproof. The load-bearing seam is **what is allowed to mutate
authoritative game state** — and by the command-layer design, the answer
is *only the simulation, and only through processed commands*. Organizing
around that gives three tiers:

- **Authoritative core (`sim`).** The deterministic heart: command
  processor, turn state machine, runtime world model, content registry,
  and the command/event type definitions. This is the only tier that
  mutates game state. It must be replayable from a command sequence and
  must run with no renderer, no window, and no input present.
- **Command sources (`input`, and later `ai`).** They observe state and
  *propose* changes by issuing commands; they never mutate. Player input
  and AI are the **same tier** — the command layer was built precisely so
  the core is indifferent to whether a `MoveUnit` came from a human click
  or an AI planner (design 3.4, "AI issues equivalent batch orders").
  This is why "a future AI segment" is not a fourth tier: it is a second
  command source, sibling to input, with the same relationship to the
  core.
- **Presentation (`view`).** Rendering, overlays, selection highlights,
  UI, audio. A pure consumer of the events the core emits. It reacts;
  it never proposes and never mutates.
### Dependency direction and data flow

The dependency graph is layered: `view`, `input`, and `ai` depend on the
core's public surface (command types, event types, read-only state
queries); **the core never names a type from any tier above it.** That
one-directional rule is the whole discipline — enforce it by review now,
and by a crate boundary once it pays off (below).

The data flow is not a single call stack, and the plan should not reason
about it as one. It is closer to CQRS: **commands flow up** into the core
(issued by sources, drained and validated by the processor), and
**events flow down** out of it (emitted after state changes, consumed by
presentation). Reads are a third, separate path — presentation and
sources query read-only views of state without mutating. Keeping these
three paths distinct is what makes the core's authority total: nothing
outside it can write, and everything outside it learns of changes the
same way (events), whether that consumer is the local renderer or, later,
a network layer.

### The boundary that becomes a crate

The core/presentation cut is the one boundary worth promoting to a crate,
and the test for when is concrete: **can the core run headless — no
renderer, no window, no input — as a determinism test harness or a
dedicated server?** A crate boundary makes the compiler guarantee the core
never depends on Bevy's render stack, lets the core build and test without
a GPU in CI (directly serving the determinism scaffold the project leans
on), and turns "dedicated server" into a build-without-the-presentation-
crate rather than a rewrite.

That promotion is deferred until the core has real mass — around Stage 5–6,
once the command layer and units exist. Promoting earlier means moving
types across a rigid boundary every time the design shifts, which is pure
churn while the shapes are still settling.

The `input`/`ai` tiers stay plugin-level boundaries (enforced by module
visibility, not crate walls) for the foreseeable plan. Their boundary with
the core is just the command type, which is already shared; a crate there
buys little and costs churn, and AI's shape is undesigned — crate-
boundarying an undesigned system is the premature-abstraction trap.

### How this lands in the stages

- **Stage 4** builds the world model and registry *inside the core tier*;
  nothing above it exists yet, so the discipline is trivially satisfied
  and the habit is established.
- **Stage 5** adds the command processor and turn machine — the core's
  defining content — and the command/event types that become the public
  surface every higher tier depends on. The headless determinism test at
  this stage's gate is the first real exercise of "core runs without
  presentation."
- **Stage 6** is the first stage with a real `view` consumer (unit
  rendering reacting to `UnitMoved`) and a real `input` source (picking,
  selection, `MoveUnit`). The one-directional rule gets its first genuine
  enforcement here: the renderer updates from the event, not by polling
  core internals.
- **Crate promotion** of the core/presentation boundary is flagged for
  Stage 5–6, taken when "does this build headless?" is a question worth
  the compiler answering rather than discipline.
  ADR-NNNN records this structure: the trust-tier decomposition, the
  one-directional dependency rule, the commands-up/events-down/reads-aside
  flow, and the headless-core crate-promotion criterion.


## Stage 4: Runtime world model

**Goal:** Establish the mutable runtime representation of the world that
every dynamic system will read and write. No visible behaviour change —
this is pure infrastructure, and it is the foundation the command layer
and turn machine have nothing to operate on without.

This is the stage that the old plan skipped entirely, and it precedes the
command layer for the same reason determinism testing preceded generation
logic: the thing being mutated has to exist and be correct before the
mutation machinery is built on top of it.

### 4.1 Static world as a resource

`WorldData` becomes a Bevy resource available at runtime, read by systems
that need static terrain (elevation, biome, resource, start positions).
It is immutable after load. The existing renderer continues to bake
terrain chunks from it unchanged.

### 4.2 Dynamic tile-state resource

A new resource holding the parallel `Field<T>` grids for mutable per-tile
state: `tile_owner: Field<Option<NetworkId>>`,
`tile_occupant: Field<Option<NetworkId>>`, and the
`improvement`/`infrastructure`/`cleared` fields (the latter defined now,
populated later). Sized for the full map at construction. This is the
ADR-NNNN tile-model decision made concrete.

### 4.3 Deterministic NetworkId allocation

A `NetworkId` allocator resource: a monotonic counter advanced only in
command-processing order. A derived, non-authoritative
`NetworkId → Entity` lookup, rebuilt from authoritative state, never
synced.

### 4.4 Yield base table and shared function

Load the RON yield base table at startup into the enum-indexed array.
Implement `tile_yield(coord)` — additive base plus the (initially
identity, since no improvements exist yet) transform. This is the shared
function cities, worker views, and the overlay will all call; building it
now, against terrain-only input, keeps it honest before improvements
complicate it.

## Stage 4.5: Anti-corruption layer — game's own world model

**Goal:** Decouple `game`'s runtime representation of the map from
`worldgen`'s data format. `game` calls `worldgen` to generate the world,
then *translates* the output into its own static-terrain model, which it
owns and which is shaped by runtime need rather than generation
internals. `WorldData` becomes a transient input consumed at world setup
and discarded, not a long-lived resource.

This corrects the Stage 4 decision that made `WorldData` itself the
long-lived runtime resource. Some Stage 4 work is redone — that is
deliberate. The boundary is far cheaper to install now, while the only
consumer of the world model is the renderer and no command layer yet
depends on it, than after Stages 5–7 accumulate dependencies on
`worldgen`'s shape.

### Why this is decoupling, not trimming

The motivating reason is **not** memory. `WorldData`'s generation-internal
fields (moisture, tectonic intermediates, climate scratch) are a few
megabytes the runtime would never notice; trimming alone would argue for
leaving `WorldData` in place and ignoring unused fields. The reason is
**coupling**: today `game` depends on `worldgen`'s internal
representation, so `worldgen` cannot evolve that representation for
generation reasons without recompiling or breaking `game` — even for
fields like moisture that `game` has no legitimate interest in. This
stage installs an anti-corruption layer (the translation step) so the two
crates' models evolve independently. Memory reduction is a side effect,
not the justification.

The distinction matters because it changes the target shape. `game`'s
model is **not** "`WorldData` with unused columns deleted" — a subset
shaped like `WorldData` would still be `worldgen`'s shape and would drift
back toward coupling. It is a model shaped by what `game` needs at
runtime, which differs from generation output in two ways a subset would
miss:

- **Some of it is derived, not raw.** Per-tile movement cost, for example,
  is a function of terrain/feature/elevation that `game` needs for
  pathfinding. Computing it once at translation and storing it flat is the
  data-oriented move the tile-grid design already favors, versus
  recomputing it from raw fields on every pathfinding step. The
  translation layer precomputes runtime-relevant derivations, not just
  copies fields.
- **It merges with mutable runtime state.** The Stage 4 dynamic grids
  (`tile_owner`, `tile_occupant`, `improvement`) are runtime concepts that
  do not and never will exist in `WorldData`. `game`'s map representation
  is therefore necessarily its own thing: the static terrain `game` cares
  about, living beside the mutable grids as one runtime world model. This
  stage is, in effect, "populate the static half of the runtime world
  model from generation output," with `WorldData` as one input.

### What `game` keeps

The player-impacting static terrain: elevation level (the discretized
tier, not raw height where raw height is generation-internal), biome,
features, resources, start positions — plus precomputed runtime
derivations such as movement cost. Generation internals
(moisture, tectonic and climate intermediates, generation scratch) stay
behind the boundary in `worldgen` and are never named by `game`.

### Why the boundary is cheap here

The legitimately-shared vocabulary already lives in shared crates from
Stage 3.7: `Coord` and the content registry handles (`ResourceId`,
`Biome`) cross the boundary *as themselves*, with no translation and no
duplication. Only truly `worldgen`-internal concepts stay behind the
wall — which are exactly the concepts `game` should never see. The
translation tax is small precisely because Stage 3.7 already carved out
the common types; without it, this boundary would force duplicating
`ResourceId` across the seam. The decisions compound.

### 4.5.1 Static-terrain model

Define `game`'s static-terrain resource: the kept fields above, as dense
`Field<T>` grids consistent with the runtime model's layout, with derived
values (movement cost) precomputed. This is the long-lived static half of
the runtime world model; it sits beside the Stage 4 mutable grids.

### 4.5.2 Translation step

A world-setup step that calls `worldgen`, consumes the returned
`WorldData`, populates the static-terrain model (copying kept fields,
resolving any generation enums to runtime form, computing derivations),
and then drops `WorldData`. After setup, no `WorldData` value and no
dependency on its shape survives in `game`'s runtime.

### 4.5.3 Repoint consumers

The renderer bakes terrain chunks from `game`'s static-terrain model
instead of from `WorldData`. Any Stage 4 code that read `WorldData` as a
resource is repointed to the new model. This is the redone work.

### Gate: Stage 4.5 complete

- `game` defines and owns its static-terrain model; its fields are the
  runtime-relevant set, with derived values precomputed at translation.
- A translation step consumes `WorldData` at setup and discards it; no
  long-lived `WorldData` resource remains, and no `game` runtime type
  names a `worldgen`-internal field.
- The renderer and all Stage 4 consumers read `game`'s model, not
  `WorldData`.
- Shared types (`Coord`, registry handles) cross the boundary without
  duplication or translation.
- Rendered output is unchanged from end-of-Stage-4 (the translation is
  representation-only; a visual or hash check confirms no terrain changed).
- ADR-NNNN written: anti-corruption layer between `worldgen` and `game`,
  recording that the boundary is for decoupling (independent evolution of
  the two models), that `game`'s model is need-shaped and includes
  derivations rather than being a `WorldData` subset, and that `WorldData`
  is a transient setup input.
- `documents/architecture/architecture.md` updated: the `worldgen`→`game`
  translation boundary and `game`'s static-terrain model are new
  structure; the Stage 4 note that `WorldData` is the long-lived resource
  is corrected.

### Gate: Stage 4 complete

- `WorldData` is a runtime resource; renderer still draws from it.
- Dynamic tile-state resource exists, full-map sized, all grids present.
- `NetworkId` allocator is deterministic; a test confirms identical
  allocation order across runs from identical command sequences.
- `tile_yield(coord)` returns correct additive yields for terrain-only
  tiles; a debug query proves it combines static `WorldData` with dynamic
  tile state at an arbitrary coordinate.
- ADR-NNNN (runtime tile model) and ADR-NNNN (`NetworkId`-authoritative
  references) written and accepted.
- `documents/architecture/architecture.md` updated: the runtime world
  model is new structure.

---

## Stage 5: Command layer and turn state machine

**Goal:** Build the multiplayer-safe mutation backbone — the
`Input → Command → CommandProcessor` path (style guide §13.5) and the
turn-phase state machine (Bevy `States`) — and prove a command flows end
to end deterministically. No input UI and no units yet; commands are
issued by a test script.

Building this before units is deliberate. Writing unit movement as direct
state mutation and retrofitting it into the command layer afterward would
violate pillar 0.4 and the §13.5 rule that systems never mutate game state
directly. The backbone comes first; units are its first real client.

### 5.1 Turn state machine

A `GameState`/`TurnPhase` enum driven by Bevy `States` (e.g. a
player-input phase and a resolution phase, with turn advance between).
Systems are gated by `run_if(in_state(...))`. This is the design's
"next recommended architectural step" finally landing.

### 5.2 Command type and queue

A `Command` enum, every variant carrying a `PlayerId`. A command queue
resource. An `issue` path (the future input layer's entry point;
script-driven for now).

### 5.3 CommandProcessor

A system, gated to the correct turn phase, that drains the queue in
deterministic order, validates each command against current game state,
applies the state change through the tile-state resource and entity
spawns/despawns, advances the `NetworkId` allocator as needed, and emits
the corresponding past-tense result event. Invalid commands are rejected
without mutating state.

### 5.4 Result events

The buffered event types (`CommandRejected` plus the per-command result
events filled in by later stages). Defined here as the reaction contract;
their consumers arrive with units and cities.

### Gate: Stage 5 complete

- Turn state machine transitions correctly; phase-gated systems run only
  in their phase.
- A scripted command flows `issue → queue → process → event` and mutates
  the tile-state resource.
- Command processing order is deterministic; a determinism test runs an
  identical command sequence twice and confirms byte-identical resulting
  game state.
- An invalid command is rejected and emits `CommandRejected` without
  state change.
- ADR-NNNN (buffered-events reaction mechanism) written and accepted, if
  not folded into the Stage 4 batch.
- Style guide §13.5 cross-reference holds: no system outside the
  processor mutates game state.

---

## Stage 6: Units

**Goal:** Units exist, render, are selectable, and move — through the
command layer, under 1UPT occupancy, with movement points that reset on
turn advance. The starting settler spawns at a Stage-3.6 start position.

This is the first stage with visible dynamic behaviour and the first real
client of both the command layer and the turn machine.

### 6.1 Unit components and spawn

Unit components (`NetworkId`, `PlayerId`, position as `Coord`,
movement points, a unit-kind enum). Spawn the starting settler for each
player at its start position; write its `NetworkId` into the tile-state
occupant grid.

### 6.2 Placeholder unit rendering

Units render as instanced placeholder models on the existing rendering
Layer 3, positioned from their `Coord`. Instanced, per the rendering
plan, so this scales to many units later without rework.

### 6.3 Picking and selection

Screen-ray → terrain → hex `Coord` picking against the 3D scene (the 3D
pivot makes this a ray test, not the old 2D hit test), and a selection
resource with a selection-highlight overlay on Layer 2. This is the
input capability the whole game needs and is currently absent.

### 6.4 MoveUnit command

A `MoveUnit` command (carrying `PlayerId`) issued from a selected unit and
a clicked destination. The processor validates ownership, reachability
within movement points, and destination occupancy (1UPT — ending on an
occupied tile is rejected; the design's move-through-friendly rule can
follow later), updates the occupant grid, spends movement points, and
emits `UnitMoved`. Movement points reset on turn advance.

### Gate: Stage 6 complete

- The starting settler spawns at a real start position and renders.
- A unit can be picked and selected via the 3D scene.
- `MoveUnit` moves a unit through the command layer; 1UPT is enforced
  (destination-occupied is rejected); movement points spend and reset.
- `UnitMoved` is emitted and the renderer updates the unit's position
  from the event, not by polling.
- Determinism test covers a scripted multi-unit move sequence.
- `documents/architecture/architecture.md` updated: units are new
  structure.

---

## Stage 7: Found a city

**Goal:** A settler founds a city. The `FoundCity` command consumes the
settler, spawns a `City` entity, claims its centre tile, and the city
renders. This is the milestone — the first turns are playable.

The v1 city is deliberately **inert**. Founding is cheap (design §2.4);
an unsupported city holds territory and does almost nothing. Population,
the four growth dimensions (§2.6), production (§2.7), and the
ownership-tier system (§2.2) are all blocked on open design (§5.3 lists
these as foundational-but-undecided) and are explicitly out of scope.
This stage builds the city's *existence and identity*, not its economy.

### 7.1 City entity and founding command

A `City` entity (`NetworkId`, `PlayerId`, centre `Coord`, owned-tile set).
A `FoundCity` command (carrying `PlayerId`) issued from a selected settler.
The processor validates that the actor is a settler on a foundable tile
and satisfies minimum spacing from existing cities, then: spawns the
`City`, writes its `NetworkId` into the centre tile's `tile_owner`,
despawns the settler, clears its occupant-grid entry, and emits
`CityFounded`. Invalid foundings are rejected.

### 7.2 City rendering and back-reference

A minimal city marker on rendering Layer 3. The centre tile's
`tile_owner` is the authoritative city↔tile link (via `NetworkId`); the
city's owned-tile set is the city-side view. The yield function already
works on the claimed tile with no city-specific code path — the city
simply memoizes `tile_yield` over the tiles it owns.

### Gate: Stage 7 complete

- A selected settler founds a city through the command layer; the settler
  is consumed and the city renders.
- Founding is rejected on an invalid tile or inside the spacing radius,
  emitting `CommandRejected`.
- The centre tile's ownership is set via `NetworkId`; a query resolves
  tile → owning city and city → owned tiles consistently.
- `CityFounded` is emitted; the renderer reacts to the event.
- Determinism test covers a found-city command sequence.
- The full first-turns loop runs: spawn settler → select → move → found
  city, across a turn advance, deterministically.
- Design doc updated to record that v1 cities are inert and which §2
  systems remain deferred.

---

## Beyond Stage 7

City economics (the §2.6 four-dimension interactions, the §2.7 production
model, §2.9 emergent-vs-declared regions) are the next design frontier and
remain undesigned — §5.3 flags them as foundational, meaning they must be
resolved before, not during, their implementation stage. The
substantially-designed combat system (§3.1–3.11) is a separate large body
of work whose sequencing depends on what city infrastructure looks like.
Neither is planned in detail here, by the *defer the undecided* principle.

The improvement/development system — the first real exercise of the
`improvement` grid and the yield transform — is the most likely Stage 8
candidate, because it is comparatively well-bounded by the yield-system
decision above and because worker-type units are a natural follow-on to
Stage 6. It is gated on the resource-yield and transform-shape decisions
noted in the yield-system ADR.

---

## Status

**Current stage:** Stage 6 complete; Stages 5, 4.5, 4, 3.6, and rendering plan P1–P3 also complete.

**Next action:** Begin Stage 7 — Found a city.