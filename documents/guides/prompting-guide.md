# Prompting guide

How to write prompts for this codebase that get good results with minimal token waste.
"Good results" means: accurate answer, narrow file reads, no exploratory detours.

---

## Session starters that work

Give the AI three things upfront: **task type**, **crate/stage scope**, and **what you've
already read**.

```
Good: "In worldgen's hydrology stage, add a tuning param for minimum canyon depth.
       I've read the hydrology-parameters doc and the existing HydrologyParams struct."

Good: "Implement step 3.6.1 from documents/planning/implementation-plan.md
       (starting position viability scoring)."

Good: "In maptest's sweep subcommand (tools/maptest/src/sweep.rs), expose
       the new hydrology param `min_canyon_depth` added in the previous PR."

Bad:  "How is hydrology implemented?"   ← triggers reads of 10+ source files
Bad:  "Clean up the worldgen code."     ← no scope, no specific concern
```

The more scope you give, the fewer files the AI reads. Crate + module + struct is ideal.

---

## Patterns that work well for this codebase

**Reference the implementation plan step number.** The AI reads one doc and knows exactly
what's required, what gate conditions exist, and what tuning integration is expected.

```
"Implement step 3.5.3 from the implementation plan." → reads one doc, produces scoped plan
```

**Reference an ADR number for design decisions.** Avoids re-litigating settled choices.

```
"Per ADR-0012 (canyon walls), add a canyon_wall tile to the terrain lookup table."
```

**Reference existing patterns.** The AI mirrors what's already there instead of inventing.

```
"Add a ResourceParams field for X, following the same pattern as the existing
 cluster_expansion_attempts field."
```

**Name the tool when asking about tools.** There are five tools; naming one narrows scope
immediately.

```
"In maptune, add a slider for the new hydrology param."
"In mapbench, add --warmup-frames flag."
```

---

## Anti-patterns (cost tokens, produce vague results)

| Bad prompt | Why it's expensive | Better version |
|---|---|---|
| "How is X implemented?" | Reads many source files to find X | "In `worldgen/stages/climate.rs`, how does moisture advection work?" |
| "Check if this is correct" | No scope → reads everything nearby | "Does this match the `Field::from_fn` contract in `documents/crates/worldgen/api.md`?" |
| "Clean up the code" | No scope → might touch anything | "Remove the unused `_raw` variant from `RiverSize` in `world_data.rs`." |
| "What's the best way to do X?" | Open-ended analysis across codebase | "In worldgen, should I add this to TectonicsParams or AdvancedParams? See §1.15 split rationale." |
| "Does this codebase have X?" | Triggers wide grep | "Is there an existing X in `crates/worldgen/src/field.rs`?" |

---

## When to use /plan vs. diving straight in

Use `/plan` for:
- Multi-file or multi-crate changes
- New pipeline stages (multiple steps, gate conditions, tool integration)
- Architectural decisions that might need an ADR
- Anything where the approach is unclear

Dive straight in for:
- Single-file changes (add a field, fix a bug, rename something)
- Adding a tuning param following an existing pattern
- Doc updates
- Tool CLI flag additions

---

## Token budget tips

**Scope the read.** "Read only the public API in lib.rs, not the tests" cuts a 300-line
file to 70 lines.

**Reference api docs instead of source.** "Per `documents/crates/worldgen/api.md`, the
`Field::get` method..." avoids reading field.rs entirely.

**Specify the stage.** "In the climate stage" is much cheaper than "in worldgen" — the
latter may trigger reads of all 47 files; the former scopes to `stages/climate/`.

**Give the file path when you know it.** "Look at `crates/worldgen/src/params/climate.rs`"
is a single targeted read. "Find the climate params" is a search.

**Pre-state what you know.** "I've confirmed that HydrologyParams doesn't yet have a
canyon_depth field" saves a verification read.

---

## Providing feedback that sticks

When something goes wrong or unexpectedly right, say so explicitly. The AI records
feedback for future sessions.

```
"Don't use tracing::debug here, use tracing::trace — debug is too noisy at this call site."
"Yes, exactly — adding the validation in WorldParams::validate is the right place, not in the stage."
```

Short, specific, with a reason. "No, wrong approach because X" is more useful than "no."
