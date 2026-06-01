use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use worldgen::MapSize;

/// Top-level configuration loaded from `bench.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(crate) struct BenchConfig {
    /// Seeds to run each map size with.
    pub seeds: Vec<u64>,
    /// Map sizes to benchmark.
    pub sizes: Vec<MapSizeArg>,
    /// Terrain renderer to test.
    pub renderer: RendererArg,
    /// Seconds of steady-state camera movement to measure per case.
    pub measurement_secs: u32,
    /// Directory where JSON result files are written.
    pub output_dir: String,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            seeds: vec![1, 2, 3, 4],
            sizes: vec![
                MapSizeArg::Small,
                MapSizeArg::Standard,
                MapSizeArg::Medium,
                MapSizeArg::Large,
                MapSizeArg::Huge,
            ],
            renderer: RendererArg::Terrain3d,
            measurement_secs: 30,
            output_dir: "bench-results".to_owned(),
        }
    }
}

/// All parameters the orchestrator passes to a single-case subprocess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchCase {
    /// World RNG seed.
    pub seed: u64,
    /// Map size.
    pub size: MapSizeArg,
    /// Renderer implementation.
    pub renderer: RendererArg,
    /// Seconds of steady-state measurement.
    pub measurement_secs: u32,
    /// Pre-computed CPU-only world generation time (ms), measured by the orchestrator.
    pub world_gen_ms: f64,
    /// Pre-computed world-space map width (px), for the scripted camera path.
    pub map_world_width: f32,
    /// Pre-computed world-space map height (px), for the scripted camera path.
    pub map_world_height: f32,
}

/// Result from one completed benchmark case.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CaseResult {
    /// World seed.
    pub seed: u64,
    /// Map size name.
    pub map_size: String,
    /// Renderer name.
    pub renderer: String,
    /// Isolated CPU-only world generation time (ms).
    pub world_gen_ms: f64,
    /// Time from `App::run()` to first camera frame (ms). Includes asset loading, mesh build, GPU upload.
    pub startup_ms: f64,
    /// Resident set size at 5 s into measurement (MB), or `null` if unavailable.
    pub rss_mb: Option<f64>,
    /// Frame-time statistics over the measurement window.
    pub frame_time_ms: FrameStats,
    /// FPS statistics derived from frame times.
    pub fps: FpsStats,
}

/// Descriptive statistics over a sample of frame durations (in ms).
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FrameStats {
    /// Arithmetic mean.
    pub mean: f64,
    /// Fastest observed frame.
    pub min: f64,
    /// Slowest observed frame.
    pub max: f64,
    /// Unbiased sample variance.
    pub variance: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
    /// Total number of frames sampled.
    pub samples: usize,
}

/// FPS statistics derived from frame-time samples.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FpsStats {
    /// Mean FPS (1000 / `mean_frame_time_ms`).
    pub mean: f64,
    /// Minimum FPS (1000 / `max_frame_time_ms`).
    pub min: f64,
    /// Maximum FPS (1000 / `min_frame_time_ms`).
    pub max: f64,
}

/// Map size, mirroring `worldgen::MapSize` for CLI and RON usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum MapSizeArg {
    /// 120 × 80 tiles (~9.6 k).
    Small,
    /// 180 × 120 tiles (~21.6 k).
    Standard,
    /// 240 × 150 tiles (~36 k).
    Medium,
    /// 320 × 200 tiles (~64 k).
    Large,
    /// 400 × 250 tiles (~100 k).
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

impl std::fmt::Display for MapSizeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Small => "Small",
            Self::Standard => "Standard",
            Self::Medium => "Medium",
            Self::Large => "Large",
            Self::Huge => "Huge",
        })
    }
}

/// Historical renderer selection kept for benchmark result metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub(crate) enum RendererArg {
    /// P3 renderer: Bevy 3D PBR mesh path with elevation.
    #[value(name = "terrain-3d", alias = "terrain3d")]
    Terrain3d,
    /// P1 renderer: terrain identity as mesh vertex colour.
    VertexColor,
    /// P2 renderer: terrain identity as texture-array layer indices.
    TextureArray,
}

impl std::fmt::Display for RendererArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Terrain3d => "Terrain3d",
            Self::VertexColor => "VertexColor",
            Self::TextureArray => "TextureArray",
        })
    }
}
