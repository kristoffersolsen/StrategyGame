//! Per-tile field container.
//!
//! [`Field<T>`] is the workhorse type for every generation stage:
//! a dense, row-major [`Vec<T>`] indexed by [`Coord`] through an
//! associated [`MapBounds`]. Heightmaps, temperature, moisture, biome
//! IDs, and resource markers are all stored as `Field<T>` for some
//! `T`.
//!
//! Storage layout is `(r * width + q)`, so coordinates with the same
//! `r` (same row) are contiguous. This matches the access pattern of
//! the climate-advection inner loop, which iterates east-west within
//! each row.
//!
//! `q` wraps but `r` does not (see `coords` crate for wrap semantics);
//! [`Field::get`] / [`Field::set`] normalize `q` and reject
//! out-of-bounds `r` accordingly.

use std::ops::{Index, IndexMut};

use coords::{Coord, MapBounds};

/// A dense per-tile field of `T`, indexed by [`Coord`] via the
/// [`MapBounds`] supplied at construction.
///
/// # Layout and determinism
///
/// Storage is row-major: tile `(q, r)` sits at index `r * width + q`
/// in the backing vector. The iteration orders documented on each
/// iterator method are guaranteed stable, which generation stages
/// rely on for deterministic numerical reductions (design doc §1.10).
///
/// # Examples
///
/// ```
/// use coords::{Coord, MapBounds};
/// use worldgen::Field;
///
/// let bounds = MapBounds::new(10, 5);
/// let mut elevation = Field::filled(bounds, 0.0_f32);
/// elevation.set(Coord::new(3, 2), 1.5);
/// assert_eq!(elevation.get(Coord::new(3, 2)), Some(&1.5));
///
/// // q wraps; r does not.
/// assert_eq!(elevation.get(Coord::new(13, 2)), Some(&1.5));
/// assert_eq!(elevation.get(Coord::new(3, 5)), None);
/// ```
#[derive(Debug, Clone)]
pub struct Field<T> {
    bounds: MapBounds,
    data: Vec<T>,
}

impl<T> Field<T> {
    /// Constructs a field with every tile initialised to `value`.
    ///
    /// `T: Clone` is required so the fill value can be replicated into
    /// each cell.
    #[must_use]
    pub fn filled(bounds: MapBounds, value: T) -> Self
    where
        T: Clone,
    {
        let len = Self::len_for(bounds);
        Self {
            bounds,
            data: vec![value; len],
        }
    }

    /// Constructs a field by calling `f` for every in-bounds
    /// coordinate.
    ///
    /// `f` is called in row-major order (outer `r`, inner `q`), and is
    /// passed each coordinate exactly once.
    pub fn from_fn<F>(bounds: MapBounds, mut f: F) -> Self
    where
        F: FnMut(Coord) -> T,
    {
        let len = Self::len_for(bounds);
        let mut data = Vec::with_capacity(len);
        // MapBounds enforces width/height <= i32::MAX, so the signed
        // reinterpretation is always lossless. `cast_signed` makes
        // that contract explicit at the use site.
        let w_i32 = bounds.width().cast_signed();
        let h_i32 = bounds.height().cast_signed();
        for r in 0..h_i32 {
            for q in 0..w_i32 {
                data.push(f(Coord::new(q, r)));
            }
        }
        Self { bounds, data }
    }

    /// Returns the map bounds associated with this field.
    #[must_use]
    pub fn bounds(&self) -> MapBounds {
        self.bounds
    }

    /// Returns the total tile count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if this field has zero tiles.
    ///
    /// Always `false` for fields constructed normally — [`MapBounds`]
    /// rejects zero dimensions — but the method exists for clippy
    /// compliance with `len`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a reference to the value at `coord`, normalising `q`
    /// for east-west wrap. Returns `None` if `r` is out of bounds.
    #[must_use]
    pub fn get(&self, coord: Coord) -> Option<&T> {
        let normalized = self.bounds.normalize(coord)?;
        Some(&self.data[Self::raw_index(self.bounds, normalized)])
    }

    /// Returns a mutable reference to the value at `coord`,
    /// normalising `q` for east-west wrap. Returns `None` if `r` is
    /// out of bounds.
    #[must_use]
    pub fn get_mut(&mut self, coord: Coord) -> Option<&mut T> {
        let normalized = self.bounds.normalize(coord)?;
        let idx = Self::raw_index(self.bounds, normalized);
        Some(&mut self.data[idx])
    }

    /// Writes `value` to the cell at `coord`.
    ///
    /// # Panics
    ///
    /// Panics if `coord.r` is out of bounds. Use [`Self::get_mut`] if
    /// the coordinate's validity is uncertain.
    pub fn set(&mut self, coord: Coord, value: T) {
        let normalized = self
            .bounds
            .normalize(coord)
            .expect("Field::set called with out-of-bounds r");
        let idx = Self::raw_index(self.bounds, normalized);
        self.data[idx] = value;
    }

    /// Iterates over `(Coord, &T)` pairs in row-major order.
    ///
    /// The order is deterministic: outer loop walks `r` from `0` to
    /// `height - 1`, inner loop walks `q` from `0` to `width - 1`.
    pub fn iter_coords(&self) -> impl Iterator<Item = (Coord, &T)> + '_ {
        // MapBounds enforces width/height <= i32::MAX.
        let w_i32 = self.bounds.width().cast_signed();
        let h_i32 = self.bounds.height().cast_signed();
        (0..h_i32)
            .flat_map(move |r| (0..w_i32).map(move |q| Coord::new(q, r)))
            .zip(self.data.iter())
    }

    /// Iterates over the field values in row-major order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Iterates over the field values mutably in row-major order.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }

    /// Returns the underlying contiguous slice in row-major order.
    ///
    /// Exposed for callers that need to feed the field directly into a
    /// hash, serializer, or rendering buffer.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    // Compute the linear backing-vector index for an *already
    // normalized* coordinate. Caller must ensure `coord` is in
    // bounds; this is enforced by the public methods.
    fn raw_index(bounds: MapBounds, coord: Coord) -> usize {
        let width = bounds.width() as usize;
        // r and q are non-negative for normalized in-bounds coords,
        // and bounded by MapBounds height/width (both u32). Cast via
        // try_from is the principled form; expect is unreachable
        // given the caller contract.
        let r = usize::try_from(coord.r).expect("normalized coord has non-negative r");
        let q = usize::try_from(coord.q).expect("normalized coord has non-negative q");
        r * width + q
    }

    fn len_for(bounds: MapBounds) -> usize {
        (bounds.width() as usize) * (bounds.height() as usize)
    }
}

impl<'a, T> IntoIterator for &'a Field<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Field<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// Panicking index for hot-path code that knows its coordinates are
// valid. Mirrors `Vec`/`HashMap` idiom: `Index` panics, `get` returns
// `Option`.
impl<T> Index<Coord> for Field<T> {
    type Output = T;

    fn index(&self, coord: Coord) -> &T {
        self.get(coord)
            .expect("Field index: coordinate out of bounds")
    }
}

impl<T> IndexMut<Coord> for Field<T> {
    fn index_mut(&mut self, coord: Coord) -> &mut T {
        self.get_mut(coord)
            .expect("Field index_mut: coordinate out of bounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_bounds() -> MapBounds {
        MapBounds::new(10, 5)
    }

    #[test]
    fn filled_has_correct_length() {
        let f = Field::filled(small_bounds(), 0_u8);
        assert_eq!(f.len(), 50);
    }

    #[test]
    fn from_fn_visits_every_coord_once() {
        let bounds = MapBounds::new(4, 3);
        let f: Field<(i32, i32)> = Field::from_fn(bounds, |c| (c.q, c.r));
        assert_eq!(f.len(), 12);
        // First tile is (0, 0), last is (3, 2).
        assert_eq!(f.as_slice().first(), Some(&(0, 0)));
        assert_eq!(f.as_slice().last(), Some(&(3, 2)));
    }

    #[test]
    fn set_then_get_roundtrips() {
        let mut f = Field::filled(small_bounds(), 0_i32);
        f.set(Coord::new(3, 2), 42);
        assert_eq!(f.get(Coord::new(3, 2)), Some(&42));
    }

    #[test]
    fn q_wraps_on_get() {
        let mut f = Field::filled(small_bounds(), 0_i32);
        f.set(Coord::new(3, 2), 42);
        // q=3 and q=13 are the same column on a width-10 map
        assert_eq!(f.get(Coord::new(13, 2)), Some(&42));
        assert_eq!(f.get(Coord::new(-7, 2)), Some(&42));
    }

    #[test]
    fn r_out_of_bounds_returns_none() {
        let f = Field::filled(small_bounds(), 0_i32);
        assert!(f.get(Coord::new(0, -1)).is_none());
        assert!(f.get(Coord::new(0, 5)).is_none());
    }

    #[test]
    #[should_panic(expected = "out-of-bounds")]
    fn set_panics_on_out_of_bounds_r() {
        let mut f = Field::filled(small_bounds(), 0_i32);
        f.set(Coord::new(0, 5), 1);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn index_panics_on_out_of_bounds_r() {
        let f = Field::filled(small_bounds(), 0_i32);
        let _ = f[Coord::new(0, 99)];
    }

    #[test]
    fn iter_coords_is_row_major() {
        let bounds = MapBounds::new(3, 2);
        let f: Field<i32> = Field::from_fn(bounds, |c| c.q * 10 + c.r);
        let collected: Vec<_> = f.iter_coords().map(|(c, _)| c).collect();
        assert_eq!(
            collected,
            vec![
                Coord::new(0, 0),
                Coord::new(1, 0),
                Coord::new(2, 0),
                Coord::new(0, 1),
                Coord::new(1, 1),
                Coord::new(2, 1),
            ]
        );
    }
}
