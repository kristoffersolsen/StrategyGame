use std::fs;

use anyhow::Result;

use crate::config::{CaseResult, FpsStats, FrameStats, RendererArg};

/// Computes descriptive statistics from a slice of frame-time samples (in ms).
pub(crate) fn compute_frame_stats(mut samples: Vec<f64>) -> FrameStats {
    if samples.is_empty() {
        return FrameStats {
            mean: 0.0,
            min: 0.0,
            max: 0.0,
            variance: 0.0,
            p95: 0.0,
            p99: 0.0,
            samples: 0,
        };
    }
    // Sample count is bounded by collected frames — well within f64 mantissa range.
    #[allow(clippy::cast_precision_loss)]
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let variance = if samples.len() > 1 {
        samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };

    samples.sort_by(|a, b| a.partial_cmp(b).expect("frame times must be finite"));
    let p95_idx = percentile_idx(samples.len(), 0.95);
    let p99_idx = percentile_idx(samples.len(), 0.99);

    FrameStats {
        mean,
        min,
        max,
        variance,
        p95: samples[p95_idx],
        p99: samples[p99_idx],
        samples: samples.len(),
    }
}

/// Derives FPS stats from frame-time stats (inverse relationship).
pub(crate) fn compute_fps_stats(ft: &FrameStats) -> FpsStats {
    FpsStats {
        mean: 1000.0 / ft.mean,
        min: 1000.0 / ft.max,
        max: 1000.0 / ft.min,
    }
}

/// Prints a summary table to stderr.
pub(crate) fn print_terminal_table(
    renderer: RendererArg,
    measurement_secs: u32,
    results: &[CaseResult],
) {
    eprintln!();
    eprintln!("mapbench results — renderer: {renderer} — {measurement_secs}s per case");
    eprintln!();
    eprintln!(
        "{:<10} {:>6}  {:>8}  {:>9}  {:>8}  {:>9}  {:>9}  {:>9}",
        "Size", "Seed", "GenMs", "StartupMs", "RSS_MB", "FPS_mean", "FPS_min", "FT_p99ms"
    );
    eprintln!("{}", "─".repeat(80));
    for r in results {
        eprintln!(
            "{:<10} {:>6}  {:>8.1}  {:>9.1}  {:>8}  {:>9.1}  {:>9.1}  {:>9.1}",
            r.map_size,
            r.seed,
            r.world_gen_ms,
            r.startup_ms,
            r.rss_mb
                .map_or_else(|| "N/A".to_owned(), |v| format!("{v:.1}")),
            r.fps.mean,
            r.fps.min,
            r.frame_time_ms.p99,
        );
    }
    eprintln!();
}

/// Writes a JSON result file and returns its path.
pub(crate) fn write_json_report(
    results: &[CaseResult],
    renderer: RendererArg,
    measurement_secs: u32,
    git_hash: &str,
    output_dir: &str,
) -> Result<String> {
    fs::create_dir_all(output_dir)?;

    let timestamp = timestamp_iso8601();
    let safe_ts = timestamp.replace(':', "-");
    let filename = format!("{output_dir}/{safe_ts}.json");

    let report = serde_json::json!({
        "timestamp": timestamp,
        "git_hash": git_hash,
        "renderer": renderer.to_string(),
        "measurement_secs": measurement_secs,
        "results": results,
    });

    fs::write(&filename, serde_json::to_string_pretty(&report)?)?;
    Ok(filename)
}

// Percentile index with explicit allows: n is bounded by collected frame count
// (well within f64 precision), ceil is always non-negative.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn percentile_idx(n: usize, p: f64) -> usize {
    ((n as f64 * p).ceil() as usize)
        .saturating_sub(1)
        .min(n - 1)
}

fn timestamp_iso8601() -> String {
    let total_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after Unix epoch")
        .as_secs();

    let sec = (total_secs % 60) as u32;
    let min = ((total_secs / 60) % 60) as u32;
    let hour = ((total_secs / 3_600) % 24) as u32;
    // Days since 1970-01-01; u64 → u32 is safe for the next ~11 million years.
    #[allow(clippy::cast_possible_truncation)]
    let days = (total_secs / 86_400) as u32;
    let (year, month, day) = days_since_epoch_to_date(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Gregorian calendar conversion using the Fliegel–Van Flandern algorithm.
// Single-letter intermediate variables are intrinsic to this algorithm.
#[allow(clippy::many_single_char_names)]
fn days_since_epoch_to_date(days: u32) -> (u32, u32, u32) {
    let jdn = days + 2_440_588;
    let a = jdn + 32_044;
    let b = (4 * a + 3) / 146_097;
    let c = a - (146_097 * b) / 4;
    let d = (4 * c + 3) / 1_461;
    let e = c - (1_461 * d) / 4;
    let mo = (5 * e + 2) / 153;
    let day = e - (153 * mo + 2) / 5 + 1;
    let month = mo + 3 - 12 * (mo / 10);
    let year = 100 * b + d - 4_800 + mo / 10;
    (year, month, day)
}
