# Project Documentation Guide

This guide defines what to document, where it lives, and when it gets
updated. Documentation is a load-bearing part of the project. Code without
documentation is code that becomes unreadable to its own author within
months. The intent here is to keep documentation cheap to maintain and
trustworthy to read — so that six months from now the docs are still
accurate and worth consulting.

The rules are **mandatory**. Skipping documentation is the same kind of
shortcut as disabling a Checkstyle rule: it produces a debt that compounds.

---

## 1. Documentation typology

Four kinds of documentation exist in this project. Each has a distinct
purpose, audience, location, and update lifecycle. Confusing them is the
most common documentation failure mode.

| Kind | Lives in | Source of truth for | Updated when |
|------|----------|----------------------|--------------|
| **Rustdoc** | code (`///`, `//!`) | How to use a function/type/module | The code item changes |
| **Design docs** | `docs/design/*.md` | Why systems work the way they do | A design decision changes |
| **ADRs** | `docs/adr/NNNN-*.md` | When and why a decision was made | Never (immutable; superseded by new ADRs) |
| **Operational docs** | repo root + `docs/` | How to build, run, and contribute | The process changes |

The litmus test: if the information is **about a specific code item**, it
goes in rustdoc next to that item. If it's **about why an approach was
chosen**, it goes in a design doc. If it's **a decision made at a specific
point in time that should be preserved even if reversed**, it goes in an
ADR. If it's **about working with the repository**, it goes in operational
docs.

---

## 2. Rustdoc

### 2.1 What requires rustdoc

| Item | Rustdoc required |
|------|-------------------|
| `pub` items | Yes — enforced by the `missing_docs` lint |
| `pub(crate)` items, non-trivial | Yes — write the doc |
| `pub(crate)` trivial accessors | No |
| Private items, non-trivial | Yes — write at least a summary line |
| Private items, trivial | No |
| Modules (every `.rs` file with code) | Yes — module-level `//!` doc, see §2.4 |

"Trivial" means: single-line getter that returns a field; `Display::fmt`
or other obvious trait impls; a constructor with no validation. Anything
that does work — computes, validates, mutates state, has non-obvious
behavior — is non-trivial.

### 2.2 Required sections

Rustdoc sections follow the standard Rust conventions, enforced by clippy.

**Always required:**

- **Summary sentence** — single sentence ending in a period. Stands alone
  in IDE tooltips and docs overview tables. This is the most important
  line of the rustdoc; if you write nothing else, write this.

**Required when applicable:**

- **Body paragraph** — required when behavior isn't obvious from the
  signature. Explains what the function does and any caller-relevant
  details (invariants, side effects, performance characteristics).
- **`# Arguments`** — required when any parameter's purpose isn't obvious
  from its name and type. Don't document `fn distance(a: Coord, b: Coord)`;
  do document `fn generate(seed: u64, plate_count: u32, ...)`.
- **`# Returns`** — required when the return type's meaning isn't obvious.
  Almost always required for `bool` returns. Often unnecessary for
  domain-typed returns.
- **`# Errors`** — required for every function returning `Result`. Lists
  the conditions that produce each error variant. Enforced by the
  `clippy::missing_errors_doc` lint.
- **`# Panics`** — required for every function that can panic, even via
  `.expect()`. Enforced by the `clippy::missing_panics_doc` lint.
- **`# Examples`** — required on the primary entry points of each module
  (e.g., `generate_world`, `Coord::neighbors`). Optional elsewhere.
  Examples are compiled by `cargo test`, so they can't go stale silently.

### 2.3 Format rules

- First sentence is a summary ending in a period.
- Blank line separates summary from body.
- Standard section order: `# Arguments`, `# Returns`, `# Errors`,
  `# Panics`, `# Examples`. (Sometimes also `# Safety` for `unsafe`, which
  this project forbids — see code style guide §1.3.)
- Use `` `code` `` for inline code, type names, and parameter references.
- Use `[TypeName]` or `[`TypeName::method`]` for cross-references. These
  survive renames and become clickable links in IDEs and rustdoc HTML.
- No `<p>` tags — Markdown handles paragraph breaks.
- No HTML in general unless absolutely necessary.

### 2.4 Module-level docs (`//!`)

Every Rust source file containing code has a `//!` doc block at the top.
The Rust standard library does this on every module; Bevy does it on every
plugin. We do too.

```rust
//! Wrap-aware hexagonal coordinate math.
//!
//! Coordinates wrap automatically in the east-west direction (toroidal)
//! and are bounded in the north-south direction (operations falling off
//! the poles return `None`).
//!
//! The internal representation is axial; cube coordinates are derived
//! on demand for distance and line-of-sight calculations.
//!
//! See `docs/design/coordinates.md` for the design rationale and
//! `docs/adr/0003-axial-coords-with-wrap.md` for the decision history.
```

Module-level docs serve as the *first thing a reader sees* when
navigating into a module. They should answer:

1. What is this module for?
2. What are the load-bearing concepts?
3. Where do I read more if I want context?

Keep them short — 5–15 lines for most modules. If they grow longer, the
content probably belongs in a design doc with a link from the module.

### 2.5 Example rustdoc

```rust
/// Generates the world's tectonic plates for the given seed.
///
/// Plates are produced via wrap-aware Poisson disk sampling, then assigned
/// types (continental or oceanic) and motion vectors. The output is
/// deterministic for any given seed and parameter set — same inputs
/// always produce byte-identical output across runs and machines.
///
/// This is one of six stages in the world generation pipeline. See
/// `docs/design/worldgen.md` §1.9 for the full pipeline and the rationale
/// for faked tectonics over real geological simulation.
///
/// # Arguments
///
/// * `seed` — master RNG seed; this stage consumes a derived sub-seed so
///   tweaking one stage's parameters doesn't shift downstream stages.
/// * `params` — generation parameters. Uses `plate_count` (number of
///   plates), `land_ratio` (target fraction of continental area), and
///   `map_dimensions`.
///
/// # Errors
///
/// Returns [`GenerationError::InvalidParams`] if:
/// - `plate_count < 4` (too few plates produces a homogeneous map)
/// - `plate_count > 32` (too many produces fragmentation)
/// - `land_ratio` outside `0.2..=0.8`
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

Note how the doc references both a design doc (the "why" of faked
tectonics) and the ADR system isn't referenced here because no specific
decision was made about this function — only about the broader approach,
which lives in the design doc.

### 2.6 What rustdoc is *not* for

- Implementation history. Use git for that.
- "Why we chose algorithm X over Y." That's design-doc material.
- TODOs about future work. Use `// TODO(name)` comments inside the
  function, not rustdoc.
- Repetition of the type signature. If the signature says
  `fn foo(x: u32) -> Option<u32>`, don't write "Takes a u32 and returns
  an Option of u32." Say what the function *does*.

---

## 3. Design documents

Design docs explain **why** the project is built the way it is. They are
the source of truth for design intent, separated from the code which is
the source of truth for behavior.

### 3.1 Location and structure

```
docs/
└── design/
    ├── README.md              ← index of all design docs
    ├── overview.md            ← project-level design pillars (was design-decisions.md)
    ├── worldgen.md            ← map generation pipeline
    ├── combat.md              ← combat systems
    ├── economy.md             ← supply networks, cities, trade
    └── multiplayer.md         ← command layer, determinism, networking
```

The existing `design-decisions.md` is the project's design narrative as of
the implementation-start date. It will be split into the structure above
as sections grow large enough to warrant separation, but the split is
deferred — premature splitting is its own problem.

### 3.2 What goes in a design doc

A design doc captures:

- **Design pillars and principles** that drive system shape
- **Decisions made and their rationale** — the "why," with explicit
  alternatives considered and rejected
- **Risks identified and how they're mitigated**
- **Open questions and deferred decisions**
- **Cross-cutting tensions** between principles

A design doc does **not** capture:

- Current implementation details (the code documents itself)
- Specific function signatures or API choices (rustdoc covers these)
- A blow-by-blow history of revisions (git covers this)
- Aspirational features that aren't being designed (those are tickets or
  notes, not design)

### 3.3 When to update

Update a design doc when:

- A new design decision is made that affects the system
- A previous design decision is reversed (and write an ADR for the
  reversal — see §4)
- A previously-open question is resolved
- Implementation reveals a flaw in a design that requires the design to
  change

Do **not** update a design doc when:

- Code is refactored without behavior changes
- An implementation detail changes (e.g., switching from `HashMap` to
  `BTreeMap` for determinism is a code-style consequence of §0.4 in the
  design doc, not a new design decision)
- A bug is fixed

### 3.4 Style

- Markdown.
- Hierarchical numbered sections (e.g., `§1.4 Height`).
- Sections that are decisions follow a consistent structure:
  *Decision / Rationale / Risks / Mitigations / Open questions*.
- Plain prose, not bullet-point soup. Bullets for lists; prose for
  reasoning.
- Be specific. "We use ECS" is useless; "ECS replaces the OOP class
  hierarchy because per-frame iteration over components is the dominant
  workload and ECS provides O(1) bulk access" is useful.

### 3.5 Forking design docs

If a section grows large enough that it dominates its parent doc — say,
combat balance design becomes 4000 words — split it into its own file in
`docs/design/`. Add an entry to `docs/design/README.md` (the index) and
leave a short stub in the parent doc with a link.

---

## 4. Architecture Decision Records (ADRs)

ADRs capture **decisions at the moment they are made**, with the context
that motivated them. They are immutable historical records — once written,
they are never edited (except for trivial typo fixes). When a decision is
reversed, a new ADR is written that supersedes the old one. The old one
remains in the repo, marked as superseded.

### 4.1 Why ADRs

The Rust community and broader industry have converged on ADRs for
non-trivial projects. Reasons:

- **Reasoning is preserved across reversals.** Six months from now, you
  may decide to switch from `thiserror` to `anyhow` everywhere. Without
  ADRs, the original reasoning vanishes. With ADRs, you can read why the
  original choice was made and decide whether those reasons still apply.
- **Decisions are timestamped.** "We chose Bevy 0.18" is not a useful
  statement without knowing when. ADRs are dated.
- **Decisions have explicit alternatives.** Writing an ADR forces you to
  state what you didn't choose and why. This catches sloppy thinking.
- **ADRs are searchable.** Looking for "why do we forbid `unsafe`?" finds
  the relevant ADR immediately.

### 4.2 When to write an ADR

Write an ADR when **all** of the following apply:

- The decision affects multiple modules, the build, or a cross-cutting
  concern.
- The decision is not already covered by the design doc (which captures
  design-level decisions) or the code style guide (which captures
  style-level decisions).
- Future-you, or a future contributor, might reasonably ask "why did we
  do that?"

Do **not** write an ADR for:

- A single function's implementation choice (code comment, if needed)
- A design-level choice covered in the design doc (update the design doc
  instead)
- A style choice covered in the style guide (update the style guide
  instead)
- An obvious choice with no alternatives worth recording

### 4.3 Examples of ADR-worthy decisions

- "Workspace structure: three crates (coords / worldgen / game)"
- "Hybrid error handling: thiserror in library crates, anyhow at
  application boundary"
- "Bevy 0.18 selected; minimum supported version pinned to Rust 1.95.0"
- "RON chosen for config files over TOML, YAML, JSON"
- "Conventional Commits adopted for git history"
- "Forbid `unsafe_code` at the workspace level"

Examples of non-ADR decisions (these go elsewhere):

- "Tile composition: terrain + feature + resource as separate
  components" — design doc (already documented).
- "Use `BTreeMap` instead of `HashMap` for deterministic iteration" —
  style guide (or code comment in a specific spot).
- "Rename `MapGenerator` to `WorldGenerator`" — git commit message.

### 4.4 ADR format

ADRs follow a simple template. Files are named `NNNN-short-title.md`
where `NNNN` is a 4-digit sequence number starting at `0001`.

```markdown
# ADR 0007: Hybrid error handling with thiserror and anyhow

**Status:** Accepted
**Date:** 2026-05-23
**Deciders:** Kristoffer

## Context

The project needs an error-handling strategy. Rust has no exceptions;
errors are values, typically `Result<T, E>`. Two ecosystem libraries
dominate: `thiserror` (custom typed error enums) and `anyhow` (a single
opaque error type with context). Each has strengths and weaknesses.

## Decision

Use a hybrid approach:

- `thiserror` in the `coords` and `worldgen` crates and in any other
  library-style module where callers may want to react to specific
  error variants.
- `anyhow` in the `game` crate's top-level Bevy systems, the `mapdump`
  CLI, and any other application-boundary code where errors are logged
  or surfaced to UI rather than branched on.

## Consequences

**Positive:**
- Library code has typed errors that callers can match on.
- Application code has ergonomic error propagation without boilerplate.
- Boundary is clear and corresponds to the workspace structure.

**Negative:**
- Two error idioms in the codebase. Contributors must understand both.
- Conversion between layers requires explicit `.context()` calls.

## Alternatives considered

- **`thiserror` everywhere:** rejected because typed errors at the Bevy
  plugin boundary add ceremony without payoff — those errors are logged,
  not matched on.
- **`anyhow` everywhere:** rejected because some modules (worldgen
  adequacy failures, save/load) need callers to branch on error type.
  `anyhow::Error` is opaque and makes that impossible without
  downcasting.

## References

- Code style guide §8
- `thiserror` documentation
- `anyhow` documentation
```

### 4.5 ADR lifecycle

- **Drafting:** ADR is `Status: Proposed`. Used for in-progress
  decisions that aren't yet committed.
- **Acceptance:** changes to `Status: Accepted` with a date. From this
  moment the ADR is immutable.
- **Supersession:** if a later ADR reverses the decision, the new ADR
  references this one, and this one's status changes to
  `Status: Superseded by ADR NNNN` (a single-line edit, which is the
  only permitted edit after acceptance).
- **Deprecation:** if a decision becomes irrelevant (e.g., the module it
  governs is removed), status changes to `Status: Deprecated`.

### 4.6 ADR index

`docs/adr/README.md` lists all ADRs by number with one-line summaries
and current statuses. Updated whenever a new ADR is added or a status
changes.

---

## 5. Operational documentation

These files live at the repository root or under `docs/` and describe
how to work with the project.

### 5.1 README.md (required, repo root)

The README is the first thing a new reader sees on GitHub. Required
sections:

- **One-paragraph project description** — what this project is.
- **Status** — pre-alpha, in development. Set expectations.
- **Quick start** — how to clone, build, and run.
- **Documentation index** — links to architecture, design docs, style
  guides, and ADRs.
- **License** — even if not yet chosen, state that explicitly.

The README is **not** the design doc. Keep it short. Link to the design
docs for depth.

### 5.2 architecture.md (documents/architecture/architecture.md)

`documents/architecture/architecture.md` describes the **structure** of the
codebase: how crates relate, where major systems live, what the entry points
are. The intent is that a new contributor — or future-you returning after
months away — can read it in 10 minutes and know where to look for things.

Pattern adopted from matklad's
[ARCHITECTURE.md convention](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html),
widely used in the Rust ecosystem (rust-analyzer, ripgrep, others).

Required sections:

- **Workspace structure** — what each crate does and depends on.
- **Entry points** — where execution starts (e.g., `crates/game/src/main.rs`,
  `tools/mapdump/src/main.rs`).
- **Major systems and where they live** — high-level mapping from
  conceptual systems (map generation, combat, supply) to code locations.
- **Cross-cutting concerns** — multiplayer command layer, deterministic
  RNG, the `coords` module's role.
- **Where to find what** — quick lookup table for common questions
  ("where do I add a new generation stage?" → `crates/worldgen/src/stages/`).

Update when the **structure** changes — a new crate, a major refactor, a
moved subsystem. Not when individual code changes.

### 5.3 CONTRIBUTING.md (required, repo root)

Even for a sole-developer project, `CONTRIBUTING.md` is required. It
forces explicit documentation of the development workflow, which
future-you will appreciate.

Required sections:

- **Toolchain setup** — Rust version, `rustup` setup, recommended IDE.
- **Build and test commands** — what to run before committing.
- **Style and lint enforcement** — link to the code style guide;
  reproduce the CI commands (`cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo test`).
- **Commit message conventions** — link to §6 of this guide.
- **Branch strategy** — link to §7 of this guide.
- **ADR and design-doc conventions** — link to §3 and §4 of this guide.

### 5.4 docs/ structure

```
docs/
├── design/                    ← design documentation (see §3)
│   ├── README.md              ← index
│   └── *.md
├── adr/                       ← decision records (see §4)
│   ├── README.md              ← index
│   ├── 0001-*.md
│   └── ...
└── (other technical docs as needed)
```

Additional `docs/` files are created as needed for technical reference
material (e.g., `docs/determinism-testing.md` if determinism testing
grows complex enough to warrant its own document). These follow the same
"why and how, not what" principle as design docs.

### 5.5 Crate-level README

Each crate (`crates/coords/`, `crates/worldgen/`, `crates/game/`) has a
`README.md`. Keep it short:

- One-paragraph description of what the crate is.
- The crate's public API entry point (e.g., "start with `generate_world`
  in `lib.rs`").
- Link back to the top-level project docs.

The module-level `//!` doc in `lib.rs` overlaps with this; the crate-level
README is the version that appears on GitHub and crates.io if the crate
is ever published.

---

## 6. Git commit messages

We use **[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)**
v1.0.0.

### 6.1 Format

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### 6.2 Types

Standard set:

- **`feat`** — a new feature
- **`fix`** — a bug fix
- **`docs`** — documentation only changes
- **`style`** — formatting, missing semicolons, whitespace; no code
  behavior change
- **`refactor`** — code change that neither fixes a bug nor adds a
  feature
- **`perf`** — performance improvement
- **`test`** — adding or correcting tests
- **`build`** — changes to build system or dependencies
- **`ci`** — changes to CI configuration
- **`chore`** — maintenance tasks (e.g., updating `.gitignore`)
- **`revert`** — reverts a previous commit

Additional project-specific types are added by ADR if needed; don't
invent them ad-hoc.

### 6.3 Scopes

Scopes correspond to project structure. For this project:

- **`coords`** — the coords crate
- **`worldgen`** — the worldgen crate
- **`game`** — the game crate
- **`mapdump`** — the CLI tool
- **`ci`** — CI configuration
- **`build`** — Cargo workspace, rustfmt, clippy config
- **`docs`** — when the change crosses doc files (design, ADR, README)

A commit that touches multiple scopes is usually a sign the change
should be split. If it genuinely can't be split, omit the scope.

### 6.4 Subject line rules

- **Imperative mood.** "Add tectonic plate generation," not "Added" or
  "Adds." Matches how git itself describes commits ("If applied, this
  commit will...").
- **Lowercase first letter** after the colon. `feat(coords): add wrap
  helpers`, not `feat(coords): Add wrap helpers`.
- **No trailing period.**
- **50/72 rule:** subject line ≤ 50 characters where possible, hard
  limit 72. Body wrapped at 72.

### 6.5 Body

Required when the *why* isn't obvious from the diff. Optional when the
change is mechanical or self-explanatory.

The body explains *why*, not *what* (the diff shows what). Example:

```
fix(worldgen): sort tectonic boundaries before classification

HashMap iteration order is non-deterministic, which broke the
determinism test scaffold when boundary classification depended on
iteration order. Sort by (plate_a_id, plate_b_id) before processing.

Discovered via the determinism integration test added in ADR-0009.
```

### 6.6 Breaking changes

Mark with `!` after the type/scope and/or a `BREAKING CHANGE:` footer:

```
feat(worldgen)!: rename WorldParams to GenerationParams

BREAKING CHANGE: WorldParams is now GenerationParams. Update all
constructors and field accessors.
```

### 6.7 Footers

Optional. Common ones:

- **`BREAKING CHANGE: <description>`** — for major-version-bumping
  changes
- **`Refs: ADR-0007`** — links the commit to an ADR
- **`Closes: #42`** — closes an issue on merge (when issues exist)

### 6.8 Examples

Good commit subjects:

```
feat(coords): add hex distance with wrap awareness
fix(worldgen): correct CFL stability bound in advection
refactor(game): extract turn phase state machine into plugin
docs(adr): record decision to use Bevy 0.18
test(coords): add property tests for wrap idempotency
build(deps): bump bevy to 0.18.1
ci: add cargo audit to weekly schedule
```

Bad commit subjects (and why):

```
update stuff                          # no type, no specificity
fix(worldgen): fixed the bug          # past tense; vague
feat: many changes                    # no scope, no specificity
[WIP]                                 # never ship WIP commits
```

---

## 7. Git branching and tagging

### 7.1 Branch strategy

Pre-1.0 sole-developer phase: **trunk-based development**.

- `main` is always buildable.
- Work directly on `main` for small changes.
- Use short-lived feature branches (`feat/tectonic-generation`,
  `fix/wrap-boundary`) for larger work. Merge with squash or rebase to
  keep `main` history clean.
- Never push broken code to `main`. If something needs to be
  half-finished overnight, branch it.

This is simpler than GitFlow and appropriate for the current project
scale. When and if the project grows to multiple contributors or
production releases, the branch strategy is revisited via ADR.

### 7.2 Tagging

Tag any state worth returning to. Specifically:

- **Milestone tags** — short-lived, descriptive: `coords-v1`,
  `worldgen-tectonics-working`, `playable-prototype`. Use these freely.
- **Semver tags** — `v0.1.0`, `v0.2.0`, etc. Apply when releases begin
  to make sense; not yet relevant.

Tags are cheap. When something works and you want to be able to return
to it, tag it.

### 7.3 What doesn't go in git

- Build artifacts (`target/`, generated files) — `.gitignore`'d.
- Secrets, credentials, API keys — never. Use environment variables or
  a separate secret store.
- Large binary assets (textures, models) — these need a separate
  strategy when the project gets there. Likely Git LFS or an external
  asset store; decided by ADR when needed.

---

## 8. Updating documentation

### 8.1 When to update

Documentation updates are part of the change that triggers them. They
are **not** "I'll update the docs later" tasks.

| Change | What gets updated |
|--------|-------------------|
| Change a function's behavior | Its rustdoc (in the same commit) |
| Change a module's responsibility | Its `//!` doc (in the same commit) |
| Add a new feature | rustdoc on the new code, plus relevant design doc if it's a design-level addition |
| Refactor without behavior change | Usually nothing; rustdoc only if the surface changed |
| Change a design decision | Design doc updated; ADR written for the reversal |
| Change workspace structure | `documents/architecture/architecture.md` updated |
| Change build process | `CONTRIBUTING.md` updated |
| Make a new cross-cutting decision | New ADR |

### 8.2 Documentation rot

The biggest documentation failure mode is rot: docs that describe a past
state of the system. Defenses:

- **Rustdoc examples are compiled.** Broken examples fail CI. This
  catches the most common kind of rot.
- **Design docs reference code by location, not content.** "See
  `worldgen::tectonics`" survives renames better than "see the
  `generate_plates` function."
- **ADRs don't rot** because they're immutable — they describe the past
  by design.
- **Module-level docs are read by anyone navigating to the module.**
  When they go stale, someone notices quickly.

The most rot-prone documents are mid-level technical docs in `docs/`
that aren't read regularly. Avoid creating them unless they earn their
keep — link from rustdoc and design docs so they get traffic.

### 8.3 Reviewing documentation

When reviewing a change (even self-review):

- Does the rustdoc match the new code behavior?
- Are there changes that should have updated a design doc but didn't?
- Are there decisions that should have been an ADR?
- Did the module-level `//!` doc stay accurate?

A change that updates code without updating its documentation is, by
default, an incomplete change.

---

## 9. When this guide is silent

Defer to:

1. The [Rust documentation guidelines](https://rust-lang.github.io/api-guidelines/documentation.html)
   in the Rust API Guidelines
2. The [Rustdoc book](https://doc.rust-lang.org/rustdoc/) for rustdoc
   syntax and features
3. The [Conventional Commits spec](https://www.conventionalcommits.org/en/v1.0.0/)
   for commit message edge cases
4. Existing well-documented modules as reference implementations
5. Ask, then update this guide

Documentation decisions, like style decisions, don't get made twice.
