# Contributing

This document describes the development workflow for the Strategy Game project.
It is written for a sole-developer project but structured to scale if contributors
join later.

---

## Prerequisites

- [rustup](https://rustup.rs)
- Git
- A Unix-like shell (Linux, macOS, or WSL on Windows)
- **Linux only:** Bevy system libraries — installed automatically by the
  setup script on apt-based distros. On other distros, install manually:
  ALSA (`libasound2-dev`), udev (`libudev-dev`), Wayland (`libwayland-dev`),
  xkbcommon (`libxkbcommon-dev`), and `pkg-config`.

## Initial setup

Clone the repository and run the setup script once:

```bash
git clone <repo-url>
cd strategy-game
./scripts/setup-dev-env.sh
```

The script installs the pinned Rust toolchain, configures git hooks, and
verifies the workspace builds. It is safe to re-run.

**What the script sets up:**
- Rust 1.95.0 (pinned in `rust-toolchain.toml` — all `cargo` commands use
  this automatically)
- `clippy` and `rustfmt` components
- `cargo-audit` for dependency vulnerability scanning
- Git hooks via `.githooks/` (see below)

---

## Daily workflow

### Building

```bash
cargo build --workspace          # debug build
cargo build --workspace --release  # release build
```

### Testing

```bash
cargo test --workspace           # all tests, all crates
cargo test -p coords             # one crate only
cargo test -- --nocapture        # show println! output during tests
```

### Linting and formatting

```bash
cargo fmt                        # format all code
cargo fmt --check                # check without modifying (what CI runs)
cargo clippy --workspace --all-targets -- -D warnings  # what CI runs
```

Run both before opening a PR. The git hooks catch most issues before
commit, but running them manually gives faster feedback during development.

### Documentation

```bash
cargo doc --workspace --no-deps --open   # build and open in browser
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps  # what CI runs
```

---

## Git workflow

### Branch strategy

Trunk-based development. `main` is always buildable.

- **Small changes** (single PR-sized): work directly on `main` locally,
  push to a feature branch, open a PR.
- **Larger work**: use a short-lived feature branch named after the change:
  `feat/tectonic-generation`, `fix/wrap-boundary-edge-case`.

Never push broken code to `main`. If work needs to pause mid-feature,
keep it on a branch.

### Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
The `commit-msg` git hook validates the format on every commit.

Format: `<type>(<scope>): <subject>`

```
feat(coords): add wrap-aware hex distance
fix(worldgen): sort boundaries before classification
docs(adr): record hybrid error handling decision
refactor(game): extract turn phase into plugin
test(coords): add property tests for wrap idempotency
build(deps): bump bevy to 0.18.1
```

Full convention in `documents/guides/documentation-guide.md §6`.

**On squash-merge:** PRs are merged to `main` via squash. The squash commit
message is what goes into `main`'s history — make it a clean Conventional
Commits subject line, not "squashed commits of branch feat/..."

### Opening a PR

1. Push your branch to GitHub.
2. Open a PR against `main`.
3. The PR template (`.github/pull_request_template.md`) auto-populates —
   fill in the summary and work through the self-review checklist.
4. CI runs automatically. Fix any failures before merging.
5. Self-approve and squash-merge.

The full self-review checklist is in `documents/guides/pr-checklist.md`. CI covers
formatting, linting, tests, and rustdoc — the checklist covers what
automation cannot.

### Git hooks

Installed by the setup script. Live in `.githooks/`:

| Hook | When it runs | What it checks |
|------|-------------|----------------|
| `pre-commit` | Before every commit | `cargo fmt --check` on staged `.rs` files; TODO comment format |
| `commit-msg` | After writing commit message | Conventional Commits format |

If a hook rejects a commit, the error message tells you exactly what to fix.

To temporarily bypass hooks (use sparingly, never on `main`):
```bash
git commit --no-verify
```

---

## Code style

See `documents/guides/style-guide.md` for the full coding conventions. Key points:

- `rustfmt` enforces formatting — run `cargo fmt` before committing.
- `clippy` at pedantic level — warnings are errors in CI.
- `pub(crate)` is the default visibility; `pub` requires justification.
- No `unsafe` — `unsafe_code = "forbid"` is set workspace-wide.
- Error handling: `thiserror` in library crates, `anyhow` at application
  boundaries. See `documents/guides/style-guide.md §8`.

---

## Documentation

See `documents/guides/documentation-guide.md` for the full documentation conventions. Key points:

- Every `pub` and non-trivial `pub(crate)` item needs rustdoc.
- Every `.rs` file needs a `//!` module-level doc block.
- Design decisions go in `documents/design/`. Cross-cutting decisions get an
  ADR in `documents/adr/`.
- Commit messages follow Conventional Commits (enforced by git hook).

---

## Repository structure

```
strategy-game/
├── Cargo.toml              ← workspace manifest
├── Cargo.lock              ← committed; ensures reproducible builds
├── rust-toolchain.toml     ← pins Rust 1.95.0
├── rustfmt.toml            ← formatter config
├── .editorconfig
├── .githooks/              ← local git hooks (installed by setup script)
│   ├── commit-msg
│   └── pre-commit
├── .github/
│   ├── workflows/
│   │   └── ci.yml          ← CI: fmt, clippy, test, doc
│   └── pull_request_template.md
├── scripts/
│   └── setup-dev-env.sh    ← one-time setup
├── crates/
│   ├── coords/             ← wrap-aware hex coordinate math (no Bevy)
│   ├── worldgen/           ← procedural generation pipeline (no Bevy)
│   └── game/               ← Bevy application
├── tools/
│   └── mapdump/            ← CLI for generation debugging
└── documents/
    ├── adr/                ← architecture decision records
    ├── architecture/       ← ARCHITECTURE.md
    ├── design/             ← design documents
    ├── guides/             ← STYLE_GUIDE, DOCUMENTATION_GUIDE, PR_CHECKLIST
    ├── planning/           ← IMPLEMENTATION_PLAN, STATUS
    └── worldgen/           ← worldgen parameter references
```

Full architecture description in `documents/architecture/architecture.md`.

---

## CI

GitHub Actions runs on every PR and every push to `main`. Four parallel jobs:

| Job | Command |
|-----|---------|
| Format | `cargo fmt --check --all` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace` |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` |

All four must pass before a PR can merge. Branch protection on `main`
enforces this.

Dependencies are cached between runs (`Swatinem/rust-cache`). First run
after a `Cargo.lock` change is slow (recompiles dependencies); subsequent
runs are fast.
