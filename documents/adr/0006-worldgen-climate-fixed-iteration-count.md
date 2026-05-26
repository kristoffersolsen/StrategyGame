# ADR-0006: Fixed-Count Advection Iteration and Determinism Strategy

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

The moisture advection loop runs iteratively until the field reaches a
steady state. The question was how to decide when to stop.

The natural answer is a convergence criterion: run until the maximum
change in the moisture field across one iteration falls below some
epsilon. This is how many physical simulations terminate their loops.

The problem is that floating-point convergence detection is platform-
sensitive. The epsilon comparison involves subtracting two nearly-equal
floating-point values; the result is affected by FMA contraction,
compiler optimisation level, SIMD width, and rounding-mode defaults.
Two machines may terminate at different iteration counts for the same
input — violating the byte-identical determinism requirement established
in ADR-0001.

A secondary problem is predictability. With a convergence criterion,
wall-clock time for generation depends on parameter choices in
non-obvious ways. Maps with high `evaporation_rate` and low
`base_precipitation_rate` may take many more iterations to converge
than maps with the reverse. The fixed-count approach makes generation
time a simple linear function of map size.

## Decision

The iteration count is fixed and derived at validation time, not
at loop runtime:

```
iteration_count = max(iteration_count_min,
                      round(iteration_count_factor × map_width))
```

The loop runs for exactly this count with no per-iteration convergence
check. No early exit.

`iteration_count_factor` (default 1.5) ties the count to map width so
that moisture has time to complete roughly 1.5 transits across the map
before the loop ends. `iteration_count_min` (default 100; tuned preset
uses 500) prevents small maps from terminating before latitude bands
become visible.

The iteration order within each step is fixed `(r, q)` row-major for
the same reason: consistent floating-point accumulation order across
machines. The loop is single-threaded; parallelising across tiles would
make accumulation order non-deterministic.

Iteration count is derived, not stored: `ClimateParams::iteration_count`
computes it on demand. `maptest sweep` exposes `iteration_count_factor`
as the sweepable parameter, since the derived count is what users
actually want to vary.

## Consequences

**Positive:**

- Determinism is guaranteed: identical inputs produce identical outputs
  regardless of platform, compiler, or optimisation flags.
- Generation time is predictable and proportional to map size.
- Sweeping `iteration_count_factor` cleanly isolates the effect of
  iteration depth from all other parameters.
- The fixed-count approach naturally validates that results are
  insensitive to additional iterations (a useful sanity check: sweeping
  the factor over `0.8..2.5` and seeing the moisture field stabilise
  above ~1.2 confirms the system has reached effective steady state).

**Negative:**

- On some parameter combinations the loop may over-iterate, wasting
  time, or under-iterate, producing incomplete propagation. The factor
  must be tuned to ensure adequate propagation across all expected
  parameter ranges.
- The `iteration_count_min` value requires separate tuning for small
  maps (default preset 500) vs medium/large maps; a single factor is
  not sufficient for all sizes.

## Alternatives considered

**Convergence criterion (max-delta threshold)** — run until the
maximum per-tile moisture change falls below an epsilon. Rejected
because the comparison is floating-point-sensitive, violating ADR-0001.
Even with a generous epsilon (e.g., `1e-5`), convergence iteration count
varies unpredictably with parameter choices. Additionally, "converged"
does not mean "correct" — the system may converge to a physically
implausible steady state at a bad parameter combination, and the fixed-
count approach surfaces this more visibly.

**Convergence criterion with a deterministic integer iteration cap** —
run until convergence or a hard cap, whichever comes first. Rejected
because the cap would need to be set conservatively high (above the
max expected convergence count), making it effectively fixed-count with
added complexity.

## References

- ADR-0001 — byte-identical determinism requirement
- `crates/worldgen/src/stages/climate/iterate.rs` — loop implementation
- `crates/worldgen/src/params/climate.rs` — `iteration_count` method
- `tools/maptest/src/sweep.rs` — `iteration_count_factor` sweep arm
- documents/planning/status.md §3.2 — key sweeps: `iteration_count_factor` 0.8..2.5 × 5 steps
