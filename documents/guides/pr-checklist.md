# Pull Request Checklist

Every PR to `main` is reviewed against this checklist before merge. The
items here are the things automation cannot check — formatting, linting,
and tests are enforced by CI and don't appear here.

The companion file `.github/pull_request_template.md` auto-populates on
every PR with a shortened version of this checklist. Use that as the
operational checklist; this document explains *why* each item matters.

---

## What CI already enforces

These don't appear on the checklist. If you're tempted to add them, the
fix is in CI, not here.

- `cargo fmt --check` — formatting
- `cargo clippy -- -D warnings` — linting at pedantic level
- `cargo test` — unit tests, integration tests, and rustdoc examples
- `cargo doc` — rustdoc builds without errors
- `missing_docs` lint — every `pub` item has rustdoc
- `missing_errors_doc` / `missing_panics_doc` — `# Errors` and `# Panics`
  sections present where required

---

## The checklist

### 1. All tests pass

Run and pass ./scripts/check.sh. This is a replacement for github actions.

### 2. ADRs written for cross-cutting decisions

If this PR makes a decision that meets the ADR criteria from the
documentation guide §4.2 (affects multiple modules or the build, not
already covered elsewhere, future-relevant), an ADR exists in
`docs/adr/` for that decision.

**Common cases that need an ADR:**
- Adopting a new dependency that affects how code is written elsewhere
  (a new error type, a new logging crate, a new serialization format)
- Changing a previously-recorded decision (write a new ADR superseding
  the old one)
- Establishing a new cross-cutting convention (e.g., "all background
  threads use a specific channel pattern")

**Not ADR cases:** single-function implementation choices; design
decisions covered in a design doc; style decisions covered in the style
guide.

### 2. Design doc updated if design intent changed

If this PR implements something differently than the design doc
specified — or fills in a previously-open design question — the design
doc is updated in the same PR.

**Examples:**
- Implementation revealed a flaw in the original design; design doc
  reflects the new approach.
- A previously-deferred decision was made; design doc records what was
  chosen and why.

**Not this case:** the PR implements the design doc as written. No doc
update needed.

The trap: drifting between code and design doc without updating either
intentionally. If the code now does X and the design doc says Y, one of
them is a bug.

### 3. Module-level `//!` doc updated if responsibility changed

If a module's purpose, scope, or load-bearing concepts changed, its
`//!` doc at the top of the file is updated.

**Examples:**
- A module that handled only generation now also handles serialization.
- A new submodule was added that's worth mentioning in the parent's
  overview.
- Cross-references at the bottom (`See docs/design/...`) point to a
  file that was renamed or moved.

### 4. Rustdoc on changed items is adequate, not just present

The `missing_docs` lint catches *absence* of rustdoc. It does not catch
*insufficient* rustdoc. A `pub fn` with the doc `/// Does the thing.`
passes the lint and fails this check.

**Adequate means:**
- Summary sentence states what the function does, not just that it does
  something.
- If behavior isn't obvious from the signature, a body paragraph
  explains it.
- If parameters' purposes aren't obvious from name + type, `# Arguments`
  documents them.

If you've changed a function's behavior, the rustdoc reflects the new
behavior — not the old one.

### 5. `# Errors` and `# Panics` sections still accurate

When a function's error or panic behavior changes:
- New error variants added → `# Errors` updated
- New panic conditions introduced → `# Panics` updated
- Removed error variants or panic paths → sections updated to match

The `missing_errors_doc` lint catches absence, not staleness. If your PR
adds `InvalidParam` to `GenerationError`, every function that returns
`GenerationError` needs its `# Errors` checked.

### 6. Crate API docs updated if public API changed

If this PR adds or removes a `pub` item in `worldgen`, `coords`, or `game` (i.e., changes
the `pub use` block in `lib.rs`), update the corresponding
`documents/crates/<crate>/api.md` file in the same PR. The api docs mirror the
`lib.rs` re-export block and are the first thing read in new sessions.

### 7. documents/architecture/architecture.md updated if workspace structure changed

`documents/architecture/architecture.md` describes the structure of the codebase. Update it
when the structure changes:
- New crate added or removed
- Major module moved between crates
- New entry point (binary) added
- Cross-cutting concern added (e.g., a new shared module like `coords`)

**Not this case:** a new function was added, a system was refactored
internally, a new test was written. These don't change structure.


### 8. Cross-references resolve to real files

If this PR adds or changes a reference to:
- `documents/design/*.md`
- `documents/adr/NNNN-*.md`
- A specific section in another doc (`§3.2`)
- A linked rustdoc item (`[GenerationError]`)

The referenced target exists, is at the path/section/name claimed, and
says what's being claimed. Broken cross-references rot the fastest of
all documentation.

---

## Process

### Self-review before opening the PR

Run through the checklist before clicking "Create pull request." This is
the point where things are easy to fix. After the PR is open, fixes
become amendments and rebase noise.

### Self-review after CI runs

CI may catch things you missed (a new lint failure, a doctest that
doesn't compile, a test that fails on a clean checkout). Fix those
before requesting review (even from yourself).
