# coords

Wrap-aware hexagonal coordinate math for the strategy game project.

Provides [`Coord`] (axial `q`/`r` representation) and [`MapBounds`]
(cylindrical map dimensions), with operations for neighbor enumeration,
wrap normalization, distance, and range queries.

The east-west axis wraps toroidally; the north-south axis is bounded.
All spatial operations in `worldgen` and `game` are built on this crate.

## Starting point

See [`MapBounds`] in `src/bounds.rs` — most operations live there,
taking an explicit `MapBounds` alongside a `Coord`.

## Further reading

- `docs/design/overview.md` §1.4 — coordinate system rationale
- `docs/design/overview.md` §1.2 — cylindrical wrap decision