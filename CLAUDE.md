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

- **`crates/worldgen`** — No Bevy dependency. Procedural generation pipeline: tectonics → climate → hydrology → biomes → resources → starting positions. Seeded deterministic RNG (`rand_chacha::ChaCha8Rng`); each pipeline stage derives a sub-RNG from a master with a fixed stage-identifier salt so tuning one stage doesn't shift others. Output is `WorldData`, consumed by the `game` crate as a Bevy resource.

- **`crates/game`** — Bevy application. Feature-per-plugin structure. Receives `WorldData` from `worldgen` before Bevy initialises rendering. All game-state mutations flow through the command layer (`Input → Command → CommandProcessor`) — systems never mutate state directly (multiplayer determinism requirement).

- **`tools/mapdump`** — CLI binary. Takes a seed + parameter overrides, calls `worldgen`, writes per-stage PNG debug images. Primary tuning tool for generation work; must run without Bevy.

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

The project is in **Stage 0** (foundations). `coords` is substantially implemented with full tests. `worldgen` and `game` are stubs. `mapdump` is a stub. See `documents/planning/implementation-plan.md` for the full staged plan (Stages 0–3+ covering worldgen infrastructure, generation pipeline, and game scaffolding).

Design decisions are in `documents/design/design-decisions.md`. Architecture decisions are in `documents/adr/`.
