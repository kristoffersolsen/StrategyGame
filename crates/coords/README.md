# coords

Wrap-aware hexagonal coordinate math for the strategy game project.

Provides [`Coord`] (axial `q`/`r` representation) and [`MapBounds`]
(cylindrical map dimensions), with operations for neighbor enumeration,
wrap normalization, distance, and range queries.

The east-west axis wraps toroidally; the north-south axis is bounded.
All spatial operations in `worldgen` and `game` are built on this crate.

## Coordinate convention

The map uses pointy-top hex tiles stored as axial `Coord { q, r }`
values. `q` is the east-west wrap axis and `r` is the bounded
north-south axis.

The canonical neighbor offsets are defined by `Direction::offset`:

| Direction | Offset |
|-----------|--------|
| `East` | `(+1, 0)` |
| `West` | `(-1, 0)` |
| `NorthEast` | `(+1, -1)` |
| `NorthWest` | `(0, -1)` |
| `SouthEast` | `(0, +1)` |
| `SouthWest` | `(-1, +1)` |

East and west normalize through the wrap seam. Northward and southward
steps that would leave the map return `None`.

## Starting point

See [`MapBounds`] in `src/bounds.rs` — most operations live there,
taking an explicit `MapBounds` alongside a `Coord`.

## Further reading

- `docs/design/overview.md` §1.4 — coordinate system rationale
- `docs/design/overview.md` §1.2 — cylindrical wrap decision
