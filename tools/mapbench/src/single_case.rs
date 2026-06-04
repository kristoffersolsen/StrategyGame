use bevy::{asset::AssetPlugin, log::LogPlugin, prelude::*};
use game::rendering::{RenderStartupConfig, RenderingPlugin};
use game::sim::{SimPlugin, SimStartupConfig};
use worldgen::{BasicParams, WorldParams};

use crate::{collector::BenchPlugin, config::BenchCase};

/// Launches the Bevy app for one benchmark case.
///
/// The app runs to completion: when the measurement window closes, the
/// `bench_update` system prints a JSON `CaseResult` to stdout and exits.
pub(crate) fn run_single_case(case: BenchCase) {
    let sim_config = SimStartupConfig {
        world_params: WorldParams {
            seed: case.seed,
            basic: BasicParams {
                map_size: case.size.into(),
                ..BasicParams::default()
            },
            ..WorldParams::default()
        },
    };
    let render_config = RenderStartupConfig;

    App::new()
        .insert_resource(sim_config)
        .insert_resource(render_config)
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: workspace_asset_path(),
                    ..Default::default()
                })
                .build()
                .disable::<LogPlugin>(),
        )
        .add_plugins(SimPlugin)
        .add_plugins(RenderingPlugin)
        .add_plugins(BenchPlugin { case })
        .run();
}

/// Returns the absolute path to the workspace `assets/` directory.
///
/// `tools/mapbench` is two levels below the workspace root, so we step up twice.
fn workspace_asset_path() -> String {
    format!("{}/../../assets", env!("CARGO_MANIFEST_DIR"))
}

/// Returns the absolute path of the running executable for subprocess spawning.
pub(crate) fn current_exe() -> anyhow::Result<std::path::PathBuf> {
    std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("failed to resolve current executable: {e}"))
}

/// Pre-computes the world-space map dimensions from a generated world's tile bounds.
///
/// Replicates `map_world_size` from `game::rendering::hex_mesh`, which is `pub(crate)`.
/// Constants (`HEX_SIZE` = 16, formula) must stay in sync with that function.
pub(crate) fn map_world_size(tile_width: u32, tile_height: u32) -> (f32, f32) {
    const HEX_SIZE: f32 = 16.0;
    const SQRT_3: f32 = 1.732_050_8_f32;
    // u32 → f32: map tile counts are at most 400, precision loss is negligible.
    #[allow(clippy::cast_precision_loss)]
    let w = HEX_SIZE * SQRT_3 * (tile_width as f32 + 0.5);
    #[allow(clippy::cast_precision_loss)]
    let h = HEX_SIZE * 1.5 * (tile_height as f32 + 1.0 / 3.0);
    (w, h)
}
