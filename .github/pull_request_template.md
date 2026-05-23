<!--
This template appears in every PR. See PR_CHECKLIST.md for the full
rationale behind each item. CI checks (formatting, linting, tests,
rustdoc build) are not in this checklist — they're enforced separately.
-->

## Summary

<!-- One paragraph: what does this PR do, and why? -->

## Related

<!-- Optional: links to ADRs, design doc sections, or issues this PR
implements or addresses. Delete this section if not applicable. -->

- ADR:
- Design doc:

## Self-review checklist

Each item below is something CI cannot check. See `PR_CHECKLIST.md` for
why each one matters.

### Documentation

- [ ] **ADR(s) written** for any cross-cutting decisions made in this PR
      (see documentation guide §4.2)
- [ ] **Design doc updated** if design intent changed
- [ ] **Module-level `//!` docs updated** for any module whose
      responsibility changed
- [ ] **Rustdoc on changed items is adequate**, not just present
      (summary actually describes what the function does)
- [ ] **`# Errors` and `# Panics` sections** still accurate after any
      behavior changes
- [ ] **ARCHITECTURE.md updated** if workspace structure changed
- [ ] **Cross-references resolve** to real files / sections / items

### Commit message

- [ ] **Type and scope** match the change (`feat(coords):`, `fix(worldgen):`, etc.)
- [ ] **Subject** is imperative mood, lowercase, ≤72 chars, no trailing period
- [ ] **Body** is present when the *why* isn't obvious from the diff
- [ ] **Breaking changes** marked with `!` and/or `BREAKING CHANGE:` footer

---

<!--
Not in the list because CI handles it:
  - cargo fmt --check
  - cargo clippy -- -D warnings
  - cargo test (incl. rustdoc examples)
  - cargo doc
  - missing_docs / missing_errors_doc / missing_panics_doc lints
-->
