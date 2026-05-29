# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Initial setup (installs toolchain, hooks, system libs)
./scripts/setup-dev-env.sh

# Build
cargo build --workspace
cargo build --workspace --release

# Test
cargo test --workspace
cargo test -p coords              # single crate
cargo test -- --nocapture         # show output

# Lint and format
cargo fmt                         # format
cargo fmt --check                 # check only (what CI runs)
cargo clippy --workspace --all-targets -- -D warnings

# Docs
cargo doc --workspace --no-deps --open
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps  # CI version
```

## Architecture

A Cargo workspace with strict crate separation:

- **`crates/coords`** — Pure Rust, no dependencies. Wrap-aware hexagonal coordinate math. Axial (`q`, `r`) representation; `q` wraps east-west toroidally, `r` is bounded north-south. `Coord` carries no map context — wrap/bounds operations require a `MapBounds` passed explicitly. Everything spatial in `worldgen` and `game` is built on this.

- **`crates/worldgen`** — No Bevy dependency. Procedural generation pipeline: tectonics → climate → hydrology → biomes → resources. Starting positions are planned as Stage 3.6 but are not currently part of `generate_world`. Seeded deterministic RNG (`rand_chacha::ChaCha8Rng`); each pipeline stage derives a sub-RNG from a master with a fixed stage-identifier salt so tuning one stage doesn't shift others. Output is `WorldData`, consumed by the `game` crate as a Bevy resource.

- **`crates/game`** — Bevy application. Feature-per-plugin structure. Receives `WorldData` from `worldgen` before Bevy initialises rendering. All game-state mutations flow through the command layer (`Input → Command → CommandProcessor`) — systems never mutate state directly (multiplayer determinism requirement).

- **`tools/mapdump`** — CLI binary. Takes a seed + parameter overrides, calls `worldgen`, writes per-stage PNG debug images. Primary tuning tool for generation work; must run without Bevy.

## Session startup

Before starting work, read the context for your task type:

- **Worldgen pipeline work or a new stage**: read `documents/crates/worldgen/api.md`, then check `documents/planning/implementation-plan.md` for the current stage.
- **Parameter tuning / mapdump / maptest**: read `documents/crates/worldgen/parameters/<stage>.md` for the relevant stage.
- **Game / Bevy rendering**: read `documents/crates/game/api.md`.
- **coords or det_rng work**: read `documents/crates/coords/api.md`.
- **Any task**: check `documents/planning/implementation-plan.md` (the `## Status` footer) for the current stage.

## Lint configuration

Workspace-level lints in `Cargo.toml` are strict:
- `unsafe_code = "forbid"` — no unsafe anywhere
- `dead_code = "deny"` — unused code fails the build
- `missing_docs = "warn"` — every `pub` item needs rustdoc
- Clippy pedantic: all warnings are errors in CI

Adding a workspace-level lint allow requires a justification comment. Suppressing at a use-site requires a `#[allow(...)]` with a comment explaining why.

## Error handling

- **Library crates** (`coords`, `worldgen`): `thiserror` with typed enums. `# Errors` and `# Panics` rustdoc sections are required and enforced by clippy.
- **Application boundary** (`game`, `mapdump`): `anyhow` for propagation.
- Panics via `.expect("reason")` are for invariants that cannot fail given domain constraints, not for expected failures. `.unwrap()` is not allowed — always include a reason.

## Style rules to know

- **Module files**: use `foo.rs` + `foo/` directory, never `foo/mod.rs`.
- **Visibility**: `pub(crate)` is the default for cross-module items. `pub` only for external API. The `unreachable_pub` lint catches violations.
- **Logging**: `tracing` crate only. `println!`, `eprintln!`, `dbg!` are forbidden in committed code.
- **TODO format**: `// TODO(name): description` — always include the owner name and a description.
- **Bevy naming**: systems are verbs (`move_units`), components are nouns/adjectives (`Position`, `Movable`), events are past-tense (`UnitMoved`), resources are nouns (`WorldData`). No `_system`/`_component`/`_event` suffixes.
- **Bevy system parameter order**: `Commands`, then `Res`/`ResMut`, then `Query` reads/writes, then `EventReader`/`EventWriter`, then `Local`.

## Git workflow

Trunk-based development; `main` is always buildable. PRs are squash-merged.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`. The `commit-msg` hook validates this on every commit.

Git hooks in `.githooks/` (installed by setup script):
- `pre-commit`: `cargo fmt --check` on staged `.rs` files; TODO comment hygiene
- `commit-msg`: Conventional Commits format validation

## Implementation status

See `documents/planning/implementation-plan.md` — the `## Status` footer tracks the current stage; inline completion notes mark each finished step.

Available tools: `mapdump` (PNG debug dumps per stage), `maptest` (multi-seed sweeps with `run`/`sweep` subcommands), `maptune` (GUI param tuning), `mapbench` (rendering benchmarks), `maprender` (render perf gates).

Design decisions are split by domain in `documents/design/`: `overview.md` (pillars + cross-cutting), `worldgen.md`, `cities.md`, `combat.md`, `known-issues.md` (open topics + risks). Architecture decisions are in `documents/adr/`.
