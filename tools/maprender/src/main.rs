//! Interactive map-rendering performance gate.
//!
//! `maprender` launches the Bevy renderer against generated world data using
//! editable RON presets. It is intentionally separate from the production game
//! binary: this is a reusable performance harness for P1, P2, P3, and later
//! renderer experiments.

#![allow(missing_docs)] // binary crate; CLI help and presets are the user docs.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use game::rendering::{RenderStartupConfig, RenderingPlugin, TerrainRenderer};
use serde::Deserialize;
use tracing::info;
use worldgen::{BasicParams, MapSize, TuningParams, WorldParams};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();
    let preset = RenderPreset::load(&args.preset)?;
    let config = preset.into_startup_config(args.seed, args.size, args.renderer);

    info!(
        preset = %args.preset.display(),
        seed = config.world_params.seed,
        renderer = ?config.terrain_renderer,
        "starting map renderer performance gate"
    );

    App::new()
        .insert_resource(config)
        .add_plugins(DefaultPlugins)
        .add_plugins(RenderingPlugin)
        .run();

    Ok(())
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// RON file containing a `RenderPreset`.
    #[arg(long, default_value = "tools/maprender/presets/default.ron")]
    preset: PathBuf,

    /// Override the preset seed.
    #[arg(long)]
    seed: Option<u64>,

    /// Override the preset map size.
    #[arg(long)]
    size: Option<MapSizeArg>,

    /// Override the preset renderer.
    #[arg(long)]
    renderer: Option<RendererArg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct RenderPreset {
    seed: u64,
    map_size: MapSizeArg,
    renderer: RendererArg,
    tuning: TuningParams,
}

impl RenderPreset {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read render preset: {}", path.display()))?;
        ron::from_str(&content)
            .with_context(|| format!("failed to parse render preset: {}", path.display()))
    }

    fn into_startup_config(
        self,
        seed_override: Option<u64>,
        size_override: Option<MapSizeArg>,
        renderer_override: Option<RendererArg>,
    ) -> RenderStartupConfig {
        let seed = seed_override.unwrap_or(self.seed);
        let map_size = size_override.unwrap_or(self.map_size);
        let renderer = renderer_override.unwrap_or(self.renderer);

        RenderStartupConfig {
            world_params: WorldParams {
                seed,
                basic: BasicParams {
                    map_size: map_size.into(),
                    ..BasicParams::default()
                },
                tuning: self.tuning,
                ..WorldParams::default()
            },
            terrain_renderer: renderer.into(),
        }
    }
}

impl Default for RenderPreset {
    fn default() -> Self {
        Self {
            seed: 1,
            map_size: MapSizeArg::Medium,
            renderer: RendererArg::VertexColor,
            tuning: TuningParams::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
enum MapSizeArg {
    Small,
    Standard,
    Medium,
    Large,
    Huge,
}

impl From<MapSizeArg> for MapSize {
    fn from(value: MapSizeArg) -> Self {
        match value {
            MapSizeArg::Small => Self::Small,
            MapSizeArg::Standard => Self::Standard,
            MapSizeArg::Medium => Self::Medium,
            MapSizeArg::Large => Self::Large,
            MapSizeArg::Huge => Self::Huge,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
enum RendererArg {
    VertexColor,
    TextureArray,
}

impl From<RendererArg> for TerrainRenderer {
    fn from(value: RendererArg) -> Self {
        match value {
            RendererArg::VertexColor => Self::VertexColor,
            RendererArg::TextureArray => Self::TextureArray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_loads() {
        let preset = RenderPreset::load(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/presets/default.ron"
        )))
        .expect("default render preset should parse");
        let config = preset.into_startup_config(None, None, None);

        assert_eq!(config.world_params.seed, 1);
        assert_eq!(config.terrain_renderer, TerrainRenderer::VertexColor);
        assert_eq!(
            config.world_params.basic.map_size.dimensions(),
            MapSize::Medium.dimensions()
        );
    }
}
