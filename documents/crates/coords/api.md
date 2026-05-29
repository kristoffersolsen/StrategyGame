# coords public API

Wrap-aware hexagonal coordinate math. No dependencies. Used by both `worldgen` and `game`.

---

## Wrap model

`q` (east-west) wraps toroidally — coordinates that step off one edge reappear on the other.
`r` (north-south) is bounded — stepping off the top or bottom returns `None`.

A `Coord` carries no map context. Wrap and bounds questions require a `MapBounds`.

---

## Coord

```rust
pub struct Coord { pub q: i32, pub r: i32 }  // Copy, Clone, Eq, Hash, Display
```

```rust
Coord::new(q: i32, r: i32) -> Coord  // const; no validation — may be outside any map's bounds
```

Negative or out-of-range values are valid during intermediate calculations. Normalize back
into bounds with `MapBounds::normalize`.

---

## MapBounds

```rust
pub struct MapBounds { width: u32, height: u32 }  // Copy, Clone, Eq, Hash
```

```rust
MapBounds::new(width: u32, height: u32) -> MapBounds
    // Panics: width < 2 (cylindrical degeneracy), height < 1

bounds.width() -> u32
bounds.height() -> u32

bounds.contains(coord: Coord) -> bool
    // true if r ∈ [0, height); q always wraps so it's never out of bounds

bounds.normalize(coord: Coord) -> Option<Coord>
    // wraps q into [0, width); returns None if r ∉ [0, height)

bounds.neighbor(coord: Coord, dir: Direction) -> Option<Coord>
    // returns the wrapped neighbor, or None if it would be off the N/S edge
```

---

## Direction

```rust
pub enum Direction { East, West, NorthEast, NorthWest, SouthEast, SouthWest }
// Copy, Clone, Eq, Hash
```

```rust
Direction::ALL: [Direction; 6]  // stable order: E, W, NE, NW, SE, SW
```

On a pointy-top hex grid: East = +q same r; NorthEast = +q −r; NorthWest = same q −r;
SouthEast = same q +r; SouthWest = −q +r; West = −q same r.

East and West wrap; the other four are bounded at the poles.
