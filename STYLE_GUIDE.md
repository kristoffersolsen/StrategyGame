# Project Style Guide

This guide defines the coding conventions for the Strategy Game project. It
exists so that any contributor — including future-us — can read code in this
repo and find a consistent voice, layout, and set of expectations.

The guide is **mandatory**. Every rule here either supplements or refines
Rust's standard tooling, which we use as our base.

---

## 1. Base style

We follow Rust community conventions, enforced by three layers of automated
tooling. If any of these fail, the build fails. Don't disable rules locally;
raise them for discussion instead.

### 1.1 Formatting — `rustfmt`

`cargo fmt` is the formatter. Configured by `rustfmt.toml` at the workspace
root. Use defaults except where the file documents a reason otherwise.
4-space indentation, 100-character line limit, standard import sorting.

CI runs `cargo fmt --check`. A diff means the build fails.

### 1.2 Linting — `clippy`

`cargo clippy` is the linter. We run **pedantic** level: every lint in
`clippy::all` and `clippy::pedantic` is enabled. Configured in
`Cargo.toml` under `[workspace.lints]`.

CI runs `cargo clippy -- -D warnings`. Warnings are errors.

A small allow-list of pedantic lints is permitted at the workspace level —
documented in `Cargo.toml` with a comment explaining each. Adding to that
list requires justification. Suppressing a lint at a use site requires a
comment explaining why.

### 1.3 Compiler lints

The following rustc lints are promoted beyond their defaults:

- `unsafe_code = "forbid"` — no unsafe blocks anywhere in the codebase
- `missing_docs = "warn"` — public items must be documented
- `unreachable_pub = "warn"` — flags `pub` items that aren't reachable from
  the crate root, supporting visibility discipline (§4)
- `unused_must_use = "deny"` — discarding `Result` or `#[must_use]` values
  is an error
- `dead_code = "deny"` — unused code is an error, not a warning

### 1.4 Toolchain

Rust toolchain pinned in `rust-toolchain.toml` to a specific stable version
(currently 1.95.0). Updates are deliberate, not automatic.

`Cargo.lock` is committed. This is the Rust convention for binaries and
ensures reproducible builds.

---

## 2. Project structure

The project is a Cargo **workspace** with multiple crates:

```
strategygame/
├── Cargo.toml              ← workspace manifest, shared dependency versions
├── Cargo.lock              ← committed
├── rust-toolchain.toml
├── rustfmt.toml
├── .editorconfig
├── crates/
│   ├── coords/             ← wrap-aware hex coordinate math, no Bevy
│   ├── worldgen/           ← procedural generation pipeline, no Bevy
│   └── game/               ← Bevy application code
└── tools/
    └── mapdump/            ← CLI binary for generation debugging
```

**Rationale:** The `worldgen` crate must run without Bevy (per design doc
§1.8), and the `coords` crate is shared between generation and runtime.
A workspace makes these boundaries physical and keeps CLI iteration fast.

Dependency versions live in `[workspace.dependencies]` in the root
`Cargo.toml`. Member crates reference them with `bevy = { workspace = true }`.
No inline versions in member manifests.

---

## 3. Module organization

Modern Rust style — `foo.rs` + `foo/` directory, **not** `foo/mod.rs`.

```
src/
├── lib.rs
├── coords.rs              ← declares the coords module, may contain code
└── coords/
    ├── axial.rs
    └── wrap.rs
```

**Rationale:** With `mod.rs` everywhere, multiple open files all named
`mod.rs` are indistinguishable. The modern style avoids that and matches
what every recent Rust codebase (including Bevy itself) uses.

---

## 4. Visibility

Four visibility levels, in order of restrictiveness:

| Modifier | Visible where | When to use |
|----------|---------------|-------------|
| (none) | this module only | Default. Most items. |
| `pub(super)` | parent module | When a sibling needs it. |
| `pub(crate)` | this crate | Cross-module within the crate. **The workhorse.** |
| `pub` | everywhere | Only for the crate's external API. |

**Discipline rule:** never write `pub` without asking "does this need to be
visible outside this crate?" For most items in a game binary, the answer is
no, and `pub(crate)` is correct.

The `unreachable_pub` lint catches `pub` items that should have been
`pub(crate)`. Don't suppress it; fix the visibility.

---

## 5. Language

- **American English** in identifiers, comments, and rustdoc
  (`color` not `colour`, `realized` not `realised`).
- Misspellings have no purpose and must be fixed immediately when found.
  When renaming public API, check every call site to make sure the rename
  is safe.

---

## 6. Naming

Standard Rust naming applies — enforced by the compiler's built-in
`non_camel_case_types`, `non_snake_case`, and `non_upper_case_globals` lints:

- `PascalCase` for types, traits, enum variants
- `snake_case` for functions, methods, variables, modules, fields
- `SCREAMING_SNAKE_CASE` for constants and statics

Beyond the compiler's enforcement:

- Avoid abbreviations in identifiers. `transaction` not `tx`, `quantity` not
  `qty`, except where the abbreviation is already a domain term (e.g.,
  `rng`, `aoe`, `npc`).
- **Bevy systems** are verbs describing what they do: `move_units`,
  `update_supply_network`, `handle_player_input`. No `_system` suffix.
- **Bevy components** are nouns or adjectives describing what the entity
  *is* or *has*: `Position`, `Movable`, `HasSupplyLine`. No `_component`
  suffix.
- **Bevy resources** are nouns: `WorldData`, `TurnCounter`, `GameConfig`.
- **Bevy events** are past-tense events: `CityFounded`, `UnitMoved`,
  `TurnEnded`.

---

## 7. Constants and magic numbers

Numeric literals other than `0`, `1`, and `-1` must be named constants in
the type or module that owns them. The same applies to repeated string
literals.

```rust
// Bad
if week >= 20 && net_worth >= starting * 2.0 { ... }

// Good
const SPECULATOR_WEEKS: u32 = 20;
const SPECULATOR_GROWTH: f64 = 2.0;

if week >= SPECULATOR_WEEKS && net_worth >= starting * SPECULATOR_GROWTH { ... }
```

Where a set of related constants describes mutually exclusive states (turn
phases, terrain types, panel identifiers), prefer an `enum` over loose
constants.

For configuration-style numbers (tuning parameters, balance values), use
`const` at module scope or associated constants on a relevant type, not
literals at the use site.

---

## 8. Error handling

We use a **hybrid** approach: typed errors via `thiserror` for library-style
modules where callers might react to specific failures, and `anyhow` at the
application boundary (Bevy systems, CLI tools) for ergonomic propagation.

### 8.1 When to use `thiserror`

Library-style modules with errors that callers may handle differently per
variant. Examples in this project: `worldgen` errors (UI surfaces "starting
position adequacy failed" specifically), save/load errors, network
protocol errors, combat resolution errors.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("invalid seed: {0}")]
    InvalidSeed(u64),

    #[error("map dimensions {width}x{height} exceed maximum")]
    DimensionsExceedMaximum { width: u32, height: u32 },

    #[error("tectonic stage failed")]
    Tectonics(#[from] TectonicError),
}
```

### 8.2 When to use `anyhow`

Application-boundary code: `main.rs`, top-level Bevy systems, CLI tools,
top-level glue. Anywhere errors will be logged or displayed, not branched on.

```rust
use anyhow::{Context, Result};

fn start_generation(seed: u64, params: WorldParams) -> Result<WorldData> {
    let plates = generate_tectonics(seed, &params)
        .context("tectonic stage failed during world generation")?;
    // ...
    Ok(world_data)
}
```

### 8.3 Panics

`Result` is for expected failures. **Panics are for bugs.** Use them only
when a code path is genuinely unreachable given the program's invariants.

- `.expect("reason")` over `.unwrap()` — always include a reason that
  explains why this can't fail
- `unreachable!()` for impossible match arms
- `unimplemented!()` is allowed during scaffolding but must not ship; the
  `dead_code` lint and code review catch these

Common panic-vs-Result confusions:

| Situation | Use |
|-----------|-----|
| File the user selected doesn't exist | `Result` |
| Network connection drops | `Result` |
| Function precondition we just verified | `.expect("just checked above")` |
| Math overflow in a checked-domain calculation | `Result` |
| Math overflow that can't actually happen given domain bounds | `.expect("max grid is 1000x1000, can't overflow u32")` |

---

## 9. The `coords` module convention

The `coords` crate provides wrap-aware coordinate math used by both
generation and runtime. Its API conventions inform the rest of the project:

- Operations that can fall off the map in the **non-wrap (north-south)**
  direction return `Option<Coord>`. Never panic.
- The **wrap (east-west)** direction never produces `None`; coordinates
  wrap automatically.
- Distance, range, and neighbor queries are wrap-aware by default.
- Hot-loop operations may have `_unchecked` variants that take known-valid
  coordinates, but the safe form is the default.

This sets the pattern: throughout the project, "can't happen at runtime"
becomes a typed `Option` or `Result`, not a panic.

---

## 10. Logging

Use the **`tracing`** crate. Each module that logs imports the macros it
needs:

```rust
use tracing::{debug, error, info, warn};
```

Log levels:

- `error!` — unexpected failures the user or operator needs to know about
- `warn!` — recoverable problems
- `info!` — high-level lifecycle events (world generated, game saved)
- `debug!` — diagnostic detail, off by default in release
- `trace!` — fine-grained tracing, almost never committed enabled

### 10.1 Forbidden APIs

The following are not allowed in committed code:

- `println!`, `eprintln!` — use `info!` / `error!`
- `dbg!` — debugging-only macro, never committed
- `panic!("...")` for diagnostics — use `error!` and return a `Result`
- Logging without including the error — always pass the error value so the
  cause chain is preserved (`error!(err = ?e, "operation failed")`)

### 10.2 User-facing errors

Validation errors that the player can act on go through the UI's inline
error surface. Unexpected errors go through a shared error dialog. The
Bevy application has a default panic handler so nothing fails silently.

Boundary discipline: deep modules (`worldgen`, `coords`) throw typed
errors; the Bevy plugin layer decides whether to show an inline error,
pop a dialog, log it, or all three.

---

## 11. Comments

### 11.1 Allowed forms

Three forms, no others:

**Section separator** — groups related items inside a larger file:

```rust
// ------------- Map generation phases -------------
fn generate_tectonics(...) { ... }
fn generate_climate(...) { ... }
```

**Above-line explanation** — a single line above the code it describes:

```rust
// Week 1 is the first price entry; offset = current_week - history_size + 1
let start_week = (current_week - history_size + 1).max(1);
```

**End-of-line note** — short clarification on the same line:

```rust
self.money = starting_money; // tracked separately so net worth includes cash
```

### 11.2 SAFETY comments

`unsafe` is forbidden by the lint configuration, so this rule should never
apply. If the forbid is ever lifted for a specific module, every `unsafe`
block in that module requires a `// SAFETY: ...` comment explaining why
the operation is sound.

### 11.3 Forbidden forms

The boxed banner is not allowed:

```rust
// -----------------------------------
// ----------- text ------------------
// -----------------------------------
```

If a section header needs more visual weight than the single-line form,
the file has too much in it — split the module.

### 11.4 TODO comments

`// TODO(name): description` where `name` identifies who can answer
questions about the TODO. Tell the human reviewer when you add a TODO so
it doesn't get lost. Don't ship TODOs without a description.

### 11.5 Dead code

Don't comment out code "in case we need it later." Delete it. Git
remembers, and the `dead_code` lint catches unused items.

---

## 12. Rustdoc

### 12.1 When required

Rustdoc is required on:

- Every `pub` item (enforced by `missing_docs` lint)
- Every `pub(crate)` item that isn't trivially self-explanatory from its
  name and signature

Trivial accessors — single-line getters that return a field, methods like
`Display::fmt` that follow a known trait contract — don't need rustdoc.
Anything that does work (computes, validates, mutates state, has
non-obvious behavior) does.

### 12.2 Format

- The first sentence is a summary ending in a period. It stands alone —
  IDEs show it in tooltips and docs overview tables.
- A blank line separates the summary from the longer explanation.
- Standard section headers: `# Arguments`, `# Returns`, `# Errors`,
  `# Panics`, `# Examples`.
- Use `[OtherType]` or `[`OtherType::method`]` for cross-references in
  prose. These survive renames and are clickable in IDEs.
- Use `` `code` `` inline. No special escaping.
- Examples in rustdoc are compiled and tested by default. This is a
  feature; broken examples fail CI.

```rust
/// Generates the world's tectonic plates for the given seed.
///
/// Plates are generated via wrap-aware Poisson disk sampling, then
/// assigned types and motion vectors. Output is deterministic for any
/// given seed and parameter set.
///
/// # Arguments
///
/// * `seed` - the master RNG seed; this stage consumes a derived sub-seed.
/// * `params` - generation parameters; uses `plate_count` and `land_ratio`.
///
/// # Errors
///
/// Returns [`GenerationError::InvalidParams`] if `plate_count < 4` or
/// `plate_count > 32`.
///
/// # Examples
///
/// ```
/// # use worldgen::*;
/// let plates = generate_tectonics(42, &WorldParams::default())?;
/// assert_eq!(plates.count(), 10);
/// # Ok::<(), GenerationError>(())
/// ```
pub fn generate_tectonics(
    seed: u64,
    params: &WorldParams,
) -> Result<Plates, GenerationError> { ... }
```

The `# Errors` and `# Panics` sections are required when the function
returns `Result` or can panic. The `missing_errors_doc` and
`missing_panics_doc` clippy lints enforce this.

---

## 13. Bevy conventions

### 13.1 Plugin structure

Each major feature is a Bevy plugin in its own module:

```rust
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TileSelected>()
            .insert_resource(MapState::default())
            .add_systems(Startup, generate_map)
            .add_systems(Update, (
                update_tile_highlights,
                handle_tile_clicks,
            ));
    }
}
```

The plugin's `build` method is the manifest of what that feature
contributes: resources, components, systems, events, run conditions. New
contributors should be able to read it and know everything the plugin does.

### 13.2 Resource vs Component vs Event

Decision tree, applied at design time:

- **Global game state shared across systems** → `Resource`. Examples: the
  current turn number, the active player, world generation parameters.
- **Per-entity state** → `Component`. Examples: a unit's HP, a tile's
  terrain, a city's population.
- **Something that happens at a point in time and gets consumed** →
  `Event`. Examples: "a city was founded," "a unit moved," "turn ended."

When in doubt: if multiple systems need to react to a change, prefer
`Event` over polling component changes.

### 13.3 System signature ordering

Within a system function signature, parameters in this order:

1. `Commands`
2. `Res<T>` (reads first), then `ResMut<T>` (writes after)
3. `Query<T>` (reads first), then `Query` with `&mut` after
4. `EventReader<T>` (reads first), then `EventWriter<T>` (writes after)
5. `Local<T>` last

```rust
fn handle_city_founding(
    mut commands: Commands,
    config: Res<GameConfig>,
    mut player_data: ResMut<PlayerData>,
    tiles: Query<&Tile>,
    mut city_events: EventWriter<CityFounded>,
) { ... }
```

### 13.4 System sets and ordering

Use `SystemSet` to group systems that must run together as a logical phase.
Use `.before()` / `.after()` for explicit ordering **only when necessary**.
Over-ordering creates a brittle schedule that's painful to refactor.

If two systems can't run in either order, they probably belong in the same
set with internal sequential ordering or — better — one of them should be
split.

### 13.5 The command layer (multiplayer safety)

Per design pillar 0.4, all game-state mutations flow through the command
layer: `Input → Command → CommandProcessor`. Bevy systems do not mutate
game state directly. They either issue commands (input handling) or react
to processed command events (everything else).

This is foundational, not optional. New systems must be designed within
this pattern.

---

## 14. Testing

### 14.1 Test location

- **Unit tests** in `#[cfg(test)] mod tests { ... }` blocks within the
  same file as the code under test. These can access private items.
- **Integration tests** in `tests/` at each crate's root. These test the
  crate from outside as a consumer would. Determinism tests — same seed
  produces byte-identical output across runs and machines — live here.

### 14.2 Test naming

Test function names are full descriptive sentences with `_` separating
words. The function name *is* the human-readable output.

```rust
#[test]
fn wraps_east_to_west_on_horizontal_edge() { ... }

#[test]
fn returns_none_when_moving_off_north_pole() { ... }

#[test]
fn same_seed_produces_identical_heightmaps() { ... }
```

### 14.3 Test structure

- Group related tests in nested modules named after the feature:
  `mod wrap_behavior { ... }`, `mod determinism { ... }`.
- One behavior per test. If the name needs "and also," split the test.
- Multiple assertions per test are allowed when they verify a single
  behavior — for example, asserting a collection is empty, adding an
  element, and asserting the new size are all part of one logical check.
- Prefer `assert_eq!` for equality; `assert!` for booleans. Custom
  messages (`assert!(x, "context")`) are encouraged when failure isn't
  self-explanatory.

### 14.4 Property-based testing

For systems with combinatorial input spaces (coordinate wrap, generation
determinism), use `proptest` to generate inputs. Example properties worth
testing:

- Wrap idempotency: wrapping a coordinate twice equals wrapping once.
- Distance symmetry: `distance(a, b) == distance(b, a)` for all coords.
- Determinism: same seed produces identical output across N runs.

Property tests live alongside unit tests but use `proptest!` macro.

---

## 15. Dependency management

- All dependency versions live in `[workspace.dependencies]` in the root
  `Cargo.toml`, not inline in member crates.
- Use version ranges (`bevy = "0.18"`), not exact pins (`bevy = "=0.18.0"`).
  `Cargo.lock` locks exact versions for reproducible builds.
- Adding a dependency requires justification — what does it provide that
  the standard library doesn't, and what's the maintenance cost.
- Prefer well-established crates over niche ones for the same job.
- Audit dependencies periodically with `cargo audit`.

Current core dependencies (subject to change):

- `bevy` — game engine
- `rand` + `rand_chacha` — deterministic seeded RNG (per design pillar 0.4)
- `thiserror` — typed errors in library crates
- `anyhow` — application-boundary error propagation
- `tracing` + `tracing-subscriber` — logging
- `serde` + `ron` — config files (RON format per design doc)
- `proptest` — property-based testing (dev-dependency)

---

## 16. When this guide is silent

Defer to:

1. The Rust API Guidelines (https://rust-lang.github.io/api-guidelines/)
2. Existing patterns in well-documented modules (these are the reference
   implementations of the style)
3. Bevy's own source for Bevy-specific patterns
4. Ask, then update this guide

Style decisions don't get made twice. If a question comes up and the
answer isn't here, decide it once and add it.
