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

- [ ] ** Run and pass ./scripts/check.sh
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


