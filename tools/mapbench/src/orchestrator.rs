use std::{
    process::{Command, Stdio},
    time::Instant,
};

use anyhow::{Context, Result};
use worldgen::{BasicParams, WorldParams, generate_world};

use crate::{
    config::{BenchCase, BenchConfig, CaseResult, MapSizeArg},
    reporter::{print_terminal_table, write_json_report},
    single_case::{current_exe, map_world_size},
};

/// Runs the full benchmark matrix: generates each case in-process (for timing),
/// then spawns a subprocess to run the Bevy app and collect rendering metrics.
pub(crate) fn run_bench(
    config: &BenchConfig,
    size_overrides: Option<&[MapSizeArg]>,
    seed_overrides: Option<&[u64]>,
    duration_override: Option<u32>,
    output_dir: &str,
) -> Result<()> {
    let sizes: &[MapSizeArg] = size_overrides.unwrap_or(config.sizes.as_slice());
    let seeds: &[u64] = seed_overrides.unwrap_or(config.seeds.as_slice());
    let measurement_secs = duration_override.unwrap_or(config.measurement_secs);
    let renderer = config.renderer;

    let exe = current_exe()?;
    let git_hash = get_git_hash();
    let total = sizes.len() * seeds.len();
    let mut completed = 0usize;
    let mut results: Vec<CaseResult> = Vec::with_capacity(total);

    for &size in sizes {
        for &seed in seeds {
            completed += 1;

            let world_params = WorldParams {
                seed,
                basic: BasicParams {
                    map_size: size.into(),
                    ..BasicParams::default()
                },
                ..WorldParams::default()
            };

            // Time isolated world generation (no rendering).
            let gen_start = Instant::now();
            let world = generate_world(&world_params)
                .with_context(|| format!("world gen failed for seed={seed} size={size}"))?;
            let world_gen_ms = gen_start.elapsed().as_secs_f64() * 1000.0;

            let (w, h) = (world.bounds.width(), world.bounds.height());
            let (map_world_width, map_world_height) = map_world_size(w, h);

            let case = BenchCase {
                seed,
                size,
                renderer,
                measurement_secs,
                world_gen_ms,
                map_world_width,
                map_world_height,
            };

            let case_json =
                serde_json::to_string(&case).context("failed to serialise bench case")?;

            eprintln!(
                "[{completed}/{total}] seed={seed} size={size} gen={world_gen_ms:.1}ms — launching…"
            );

            let output = Command::new(&exe)
                .arg("--single-case")
                .arg(&case_json)
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .output()
                .with_context(|| {
                    format!("failed to spawn subprocess for seed={seed} size={size}")
                })?;

            if !output.status.success() {
                tracing::warn!(
                    seed,
                    size = %size,
                    status = %output.status,
                    "single-case subprocess failed, skipping"
                );
                continue;
            }

            let result: CaseResult = serde_json::from_slice(&output.stdout).with_context(|| {
                format!("failed to parse result JSON for seed={seed} size={size}")
            })?;
            results.push(result);
        }
    }

    if results.is_empty() {
        tracing::warn!("no results collected — all cases failed");
        return Ok(());
    }

    print_terminal_table(renderer, measurement_secs, &results);

    let filename = write_json_report(&results, renderer, measurement_secs, &git_hash, output_dir)?;
    eprintln!("Results written to: {filename}");

    Ok(())
}

fn get_git_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |s| s.trim().to_owned())
}
