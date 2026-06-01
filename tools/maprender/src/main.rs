//! Interactive map-rendering performance gate.
//!
//! `maprender` launches the Bevy renderer against generated world data using
//! editable RON presets. It is intentionally separate from the production game
//! binary: this is a reusable performance harness for P1, P2, P3, and later
//! renderer experiments.

#![allow(missing_docs)] // binary crate; CLI help and presets are the user docs.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bevy::{asset::AssetPlugin, log::LogPlugin, prelude::*};
use clap::{Parser, ValueEnum};
use game::rendering::{
    RenderStartupConfig, RenderingPlugin, terrain_surface::build_corner_prototype_suite_data,
};
use game::sim::{SimPlugin, SimStartupConfig};
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
    if let Some(PrototypeArg::Corners) = args.prototype {
        info!("starting R1 three-tier corner prototype harness");
        run_corner_prototype();
        return Ok(());
    }

    let preset = RenderPreset::load(&args.preset)?;
    let (sim_config, render_config) = preset.into_startup_configs(args.seed, args.size);

    info!(
        preset = %args.preset.display(),
        seed = sim_config.world_params.seed,
        "starting map renderer"
    );

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
        .run();

    Ok(())
}

fn run_corner_prototype() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: workspace_asset_path(),
                    ..Default::default()
                })
                .build()
                .disable::<LogPlugin>(),
        )
        .add_systems(Startup, spawn_corner_prototype)
        .run();
}

fn spawn_corner_prototype(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(build_corner_prototype_suite_data().into_mesh());
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        ..Default::default()
    });
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
    commands.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            shadows_enabled: false,
            ..Default::default()
        },
        Transform::from_xyz(-120.0, -160.0, 220.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(150.0, -300.0, 220.0)
            .looking_at(Vec3::new(150.0, -130.0, 4.0), Vec3::Z),
    ));
}

fn workspace_asset_path() -> String {
    format!("{}/../../assets", env!("CARGO_MANIFEST_DIR"))
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

    /// Launch an isolated prototype harness instead of a generated map.
    #[arg(long)]
    prototype: Option<PrototypeArg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct RenderPreset {
    seed: u64,
    map_size: MapSizeArg,
    tuning: TuningParams,
}

impl RenderPreset {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read render preset: {}", path.display()))?;
        ron::from_str(&content)
            .with_context(|| format!("failed to parse render preset: {}", path.display()))
    }

    fn into_startup_configs(
        self,
        seed_override: Option<u64>,
        size_override: Option<MapSizeArg>,
    ) -> (SimStartupConfig, RenderStartupConfig) {
        let seed = seed_override.unwrap_or(self.seed);
        let map_size = size_override.unwrap_or(self.map_size);

        let sim_config = SimStartupConfig {
            world_params: WorldParams {
                seed,
                basic: BasicParams {
                    map_size: map_size.into(),
                    ..BasicParams::default()
                },
                tuning: self.tuning,
                ..WorldParams::default()
            },
        };
        let render_config = RenderStartupConfig;
        (sim_config, render_config)
    }
}

impl Default for RenderPreset {
    fn default() -> Self {
        Self {
            seed: 1,
            map_size: MapSizeArg::Medium,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PrototypeArg {
    Corners,
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
        let (sim_config, _render_config) = preset.into_startup_configs(None, None);

        assert_eq!(sim_config.world_params.seed, 1);
        assert_eq!(
            sim_config.world_params.basic.map_size.dimensions(),
            MapSize::Medium.dimensions()
        );
    }
}
