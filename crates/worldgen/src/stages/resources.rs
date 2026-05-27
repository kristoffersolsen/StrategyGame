//! Resource placement (Stage 3.5, design §1.13).
//!
//! Places one optional resource per tile from a designer-editable RON
//! config. Bonus resources use uniform scatter, strategic resources use
//! a baseline plus plate-weighted concentration pass, and luxury
//! resources use baseline scatter plus clustered plate-weighted deposits.

mod bonus;
pub(crate) mod config;
mod filter;
mod luxury;
mod scatter;
mod strategic;

pub(crate) use config::ResourceConfig;

use coords::MapBounds;

use crate::params::{ResourceAbundance, ResourceParams};
use crate::rng::StageRng;
use crate::world_data::{BiomeData, HydrologyData, ResourceData, TectonicData};

/// Borrowed inputs consumed by the resource stage.
#[derive(Clone, Copy)]
pub(crate) struct ResourceInputs<'a> {
    /// Tectonic output used for plate mineral weighting.
    pub(crate) tectonics: &'a TectonicData,
    /// Hydrology output used for terrain and water filtering.
    pub(crate) hydrology: &'a HydrologyData,
    /// Biome output used for land-resource filtering.
    pub(crate) biomes: &'a BiomeData,
    /// Designer-authored resource placement config.
    pub(crate) config: &'a ResourceConfig,
}

pub(super) struct PlacementContext<'a> {
    bounds: MapBounds,
    tectonics: &'a TectonicData,
    config: &'a ResourceConfig,
    params: &'a ResourceParams,
    abundance: f32,
}

/// Runs the resource placement stage end-to-end.
pub(crate) fn run(
    inputs: ResourceInputs<'_>,
    params: &ResourceParams,
    abundance: ResourceAbundance,
    rng: &mut StageRng,
) -> ResourceData {
    let bounds = inputs.hydrology.height_level.bounds();
    let eligibility = filter::build_eligibility(
        bounds,
        inputs.tectonics,
        inputs.hydrology,
        inputs.biomes,
        inputs.config,
    );
    let mut resources = ResourceData::empty(bounds);
    let context = PlacementContext {
        bounds,
        tectonics: inputs.tectonics,
        config: inputs.config,
        params,
        abundance: abundance_multiplier(abundance),
    };

    bonus::place_bonus_resources(&context, &eligibility, &mut resources, rng);
    strategic::place_strategic_resources(&context, &eligibility, &mut resources, rng);
    luxury::place_luxury_resources(&context, &eligibility, &mut resources, rng);

    resources
}

fn abundance_multiplier(abundance: ResourceAbundance) -> f32 {
    match abundance {
        ResourceAbundance::Scarce => 0.6,
        ResourceAbundance::Standard => 1.0,
        ResourceAbundance::Abundant => 1.5,
    }
}
