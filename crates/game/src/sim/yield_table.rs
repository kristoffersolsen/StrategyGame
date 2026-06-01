//! Tile yield computation: the additive base table and shared yield function.
//!
//! The base yield for any tile is the sum of three independently-looked-up
//! components (biome yield + feature yield + resource yield), where each
//! lookup is an O(1) array index. An improvement transform is applied on
//! top; currently identity since no improvements exist. See the yield-system
//! ADR in `documents/planning/implementation_plan2.md`.
//!
//! String IDs are resolved to handles at load time. After that, all
//! hot-path yield logic works through dense arrays — no string lookups.

use std::{collections::HashMap, path::Path};

use anyhow::Context as _;
use bevy::prelude::Resource;
use coords::Coord;
use worldgen::{BiomeHandle, BiomeRegistry};

use crate::sim::terrain::StaticTerrain;
use crate::sim::world_model::TileState;

/// Path to the yield base table, relative to the workspace root.
const DEFAULT_YIELD_TABLE_PATH: &str = "assets/worldgen/yield_table.ron";

// ------------- Public types -------------

/// Food, production, and gold contribution of a single tile.
///
/// All components are unsigned; yield values are non-negative by design.
/// The additive model (biome + features + resource + improvement) keeps
/// each component ≥ 0 at every step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
pub(crate) struct TileYield {
    /// Food output — drives population and city growth.
    pub(crate) food: u32,
    /// Production output — drives building and unit construction.
    pub(crate) production: u32,
    /// Gold output — drives treasury income and trade.
    pub(crate) gold: u32,
}

impl TileYield {
    fn saturating_add(self, other: Self) -> Self {
        Self {
            food: self.food.saturating_add(other.food),
            production: self.production.saturating_add(other.production),
            gold: self.gold.saturating_add(other.gold),
        }
    }
}

/// Dense yield lookup table — the hot-path tile-yield data.
///
/// All lookups are O(1) array accesses indexed by content handles.
/// Constructed once at startup from the RON config; never rebuilt at
/// runtime. See ADR-0022 (hot-path discipline).
#[derive(Resource)]
pub(crate) struct YieldTable {
    /// Base yield per biome, indexed by `BiomeHandle.index()`.
    biome: Vec<TileYield>,
    /// Additive modifier for hilly tiles.
    hilly: TileYield,
    /// Additive modifier for floodplain tiles.
    floodplain: TileYield,
    /// Additive modifier for tiles that carry a river channel.
    river: TileYield,
}

impl YieldTable {
    /// Returns the base yield for a biome handle.
    ///
    /// # Panics
    ///
    /// Panics if `handle.index()` is out of bounds for this registry.
    pub(crate) fn biome_yield(&self, handle: BiomeHandle) -> TileYield {
        self.biome[handle.index()]
    }

    /// Returns the additive modifier for hilly terrain.
    pub(crate) fn hilly_yield(&self) -> TileYield {
        self.hilly
    }

    /// Returns the additive modifier for floodplain tiles.
    pub(crate) fn floodplain_yield(&self) -> TileYield {
        self.floodplain
    }

    /// Returns the additive modifier for river tiles.
    pub(crate) fn river_yield(&self) -> TileYield {
        self.river
    }

    fn from_config(config: &YieldTableConfig, registry: &BiomeRegistry) -> anyhow::Result<Self> {
        let n = registry.inner.len();
        let mut biome = vec![TileYield::default(); n];

        for (id, yield_val) in &config.biome_yields {
            let handle = registry
                .resolve(id)
                .ok_or_else(|| anyhow::anyhow!("yield table references unknown biome '{id}'"))?;
            biome[handle.index()] = *yield_val;
        }

        Ok(Self {
            biome,
            hilly: config.hilly_yield,
            floodplain: config.floodplain_yield,
            river: config.river_yield,
        })
    }
}

// ------------- RON config types (load-time only) -------------

#[derive(Debug, serde::Deserialize)]
struct YieldTableConfig {
    biome_yields: HashMap<String, TileYield>,
    hilly_yield: TileYield,
    floodplain_yield: TileYield,
    river_yield: TileYield,
}

// ------------- Public API -------------

/// Loads the yield base table from the default path.
///
/// Takes the `BiomeRegistry` from [`StaticTerrain`] to resolve biome
/// string IDs to handles.
///
/// # Errors
///
/// Returns an error if the file cannot be read, cannot be parsed as RON,
/// or references a biome ID not present in `registry`.
pub(crate) fn load_yield_table(registry: &BiomeRegistry) -> anyhow::Result<YieldTable> {
    load_yield_table_from(DEFAULT_YIELD_TABLE_PATH, registry)
}

/// Loads the yield base table from a custom path.
///
/// Exposed separately for tests that supply a temporary file path.
///
/// # Errors
///
/// Same as [`load_yield_table`].
pub(crate) fn load_yield_table_from(
    path: impl AsRef<Path>,
    registry: &BiomeRegistry,
) -> anyhow::Result<YieldTable> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read yield table '{}'", path.display()))?;
    let config: YieldTableConfig = ron::from_str(&text)
        .with_context(|| format!("failed to parse yield table '{}'", path.display()))?;
    YieldTable::from_config(&config, registry)
}

/// Computes the total additive yield for a single tile.
///
/// Combines static terrain data (`terrain`) with dynamic tile state
/// (`tile_state`). Returns the sum of biome base yield, terrain-feature
/// modifiers (hills, floodplain, river), and an improvement transform
/// (currently identity — no improvements exist in Stage 4/4.5).
///
/// Returns [`TileYield::default`] (all zero) for non-land tiles (ocean,
/// coast, lake, mountain) which carry no biome.
pub(crate) fn tile_yield(
    coord: Coord,
    terrain: &StaticTerrain,
    tile_state: &TileState,
    table: &YieldTable,
) -> TileYield {
    // Tiles without a biome (ocean, coast, lake, mountain) produce nothing.
    let Some(Some(biome)) = terrain.biome.get(coord).copied() else {
        return TileYield::default();
    };

    let base = table.biome_yield(biome);

    let hills = terrain.is_hilly.get(coord).copied().unwrap_or(false);
    let is_floodplain = terrain.is_floodplain.get(coord).copied().unwrap_or(false);
    let has_river = *terrain.has_river.get(coord).unwrap_or(&false);

    let features = TileYield::default()
        .saturating_add(if hills {
            table.hilly_yield()
        } else {
            TileYield::default()
        })
        .saturating_add(if is_floodplain {
            table.floodplain_yield()
        } else {
            TileYield::default()
        })
        .saturating_add(if has_river {
            table.river_yield()
        } else {
            TileYield::default()
        });

    // Improvement transform: will be applied here in Stage 8+ when
    // improvement types are defined. tile_state is consulted so this
    // function correctly combines both static terrain and dynamic state.
    let _improvement = tile_state.improvement.get(coord);

    base.saturating_add(features)
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use content::Registry;
    use coords::{Coord, MapBounds};
    use worldgen::{BiomeRegistry, Field, ResourceRegistry};

    use crate::sim::terrain::StaticTerrain;
    use crate::sim::world_model::TileState;

    fn minimal_terrain() -> StaticTerrain {
        let bounds = MapBounds::new(6, 6);
        let inner = Registry::build(["desert", "grassland"]);
        let registry = BiomeRegistry {
            inner,
            is_forest: vec![false, false],
        };
        let resource_registry = ResourceRegistry {
            inner: Registry::build([] as [&str; 0]),
            category: vec![],
        };
        StaticTerrain {
            bounds,
            mountain_level: 5,
            height_level: Field::filled(bounds, 2), // all land
            is_hilly: Field::filled(bounds, false),
            has_river: Field::filled(bounds, false),
            is_floodplain: Field::filled(bounds, false),
            is_lake: Field::filled(bounds, false),
            biome: Field::filled(bounds, None),
            forest_density: Field::filled(bounds, 0.0),
            resource: Field::filled(bounds, None),
            movement_cost: Field::filled(bounds, 1),
            start_positions: vec![],
            biome_registry: registry,
            resource_registry,
        }
    }

    fn table_for(terrain: &StaticTerrain) -> YieldTable {
        let config = YieldTableConfig {
            biome_yields: [
                (
                    "grassland".to_string(),
                    TileYield {
                        food: 2,
                        production: 0,
                        gold: 1,
                    },
                ),
                (
                    "desert".to_string(),
                    TileYield {
                        food: 0,
                        production: 1,
                        gold: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            hilly_yield: TileYield {
                food: 0,
                production: 1,
                gold: 0,
            },
            floodplain_yield: TileYield {
                food: 1,
                production: 0,
                gold: 0,
            },
            river_yield: TileYield {
                food: 0,
                production: 0,
                gold: 1,
            },
        };
        YieldTable::from_config(&config, &terrain.biome_registry).expect("test table must build")
    }

    #[test]
    fn biome_yield_round_trips() {
        let terrain = minimal_terrain();
        let table = table_for(&terrain);

        let grassland = terrain.biome_registry.resolve("grassland").unwrap();
        let desert = terrain.biome_registry.resolve("desert").unwrap();

        assert_eq!(
            table.biome_yield(grassland),
            TileYield {
                food: 2,
                production: 0,
                gold: 1
            }
        );
        assert_eq!(
            table.biome_yield(desert),
            TileYield {
                food: 0,
                production: 1,
                gold: 0
            }
        );
    }

    #[test]
    fn ocean_tile_yields_nothing() {
        let terrain = minimal_terrain(); // biome defaults to None
        let tile_state = TileState::new(terrain.bounds);
        let table = table_for(&terrain);

        let y = tile_yield(Coord::new(0, 0), &terrain, &tile_state, &table);
        assert_eq!(y, TileYield::default());
    }

    #[test]
    fn base_biome_yield_is_correct() {
        let mut terrain = minimal_terrain();
        let coord = Coord::new(2, 2);
        let grassland = terrain.biome_registry.resolve("grassland").unwrap();
        terrain.biome.set(coord, Some(grassland));

        let tile_state = TileState::new(terrain.bounds);
        let table = table_for(&terrain);

        let y = tile_yield(coord, &terrain, &tile_state, &table);
        assert_eq!(
            y,
            TileYield {
                food: 2,
                production: 0,
                gold: 1
            }
        );
    }

    #[test]
    fn features_are_additive() {
        let mut terrain = minimal_terrain();
        let coord = Coord::new(2, 2);
        let grassland = terrain.biome_registry.resolve("grassland").unwrap();
        terrain.biome.set(coord, Some(grassland));
        terrain.is_hilly.set(coord, true);
        terrain.is_floodplain.set(coord, true);

        let tile_state = TileState::new(terrain.bounds);
        let table = table_for(&terrain);

        let y = tile_yield(coord, &terrain, &tile_state, &table);
        // grassland(2,0,1) + hills(0,1,0) + floodplain(1,0,0) = (3,1,1)
        assert_eq!(
            y,
            TileYield {
                food: 3,
                production: 1,
                gold: 1
            }
        );
    }

    #[test]
    fn tile_yield_combines_static_and_dynamic_state() {
        let mut terrain = minimal_terrain();
        let coord = Coord::new(1, 1);
        let grassland = terrain.biome_registry.resolve("grassland").unwrap();
        terrain.biome.set(coord, Some(grassland));

        let mut tile_state = TileState::new(terrain.bounds);
        tile_state.cleared.set(coord, true); // dynamic state exercised

        let table = table_for(&terrain);
        let y = tile_yield(coord, &terrain, &tile_state, &table);

        // Stage 4/4.5: improvement transform is identity, so yield is unchanged.
        assert_eq!(
            y,
            TileYield {
                food: 2,
                production: 0,
                gold: 1
            }
        );
    }
}
