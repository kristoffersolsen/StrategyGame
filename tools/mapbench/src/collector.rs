use std::time::Instant;

use bevy::prelude::*;

use crate::{
    camera_path,
    config::{BenchCase, CaseResult},
    reporter::{compute_fps_stats, compute_frame_stats},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BenchPhase {
    WaitingForCamera,
    Measuring,
    Done,
}

/// Bevy resource holding all in-flight measurement state for one case.
#[derive(Resource)]
pub(crate) struct BenchState {
    pub case: BenchCase,
    app_start: Instant,
    startup_ms: Option<f64>,
    measure_start: Option<Instant>,
    frame_times: Vec<f64>,
    rss_mb: Option<f64>,
    rss_sampled: bool,
    pub phase: BenchPhase,
}

impl BenchState {
    pub(crate) fn new(case: BenchCase) -> Self {
        Self {
            case,
            app_start: Instant::now(),
            startup_ms: None,
            measure_start: None,
            frame_times: Vec::with_capacity(2048),
            rss_mb: None,
            rss_sampled: false,
            phase: BenchPhase::WaitingForCamera,
        }
    }

    fn measure_elapsed_secs(&self) -> f64 {
        self.measure_start
            .map_or(0.0, |s| s.elapsed().as_secs_f64())
    }

    pub(crate) fn to_case_result(&self) -> CaseResult {
        let frame_stats = compute_frame_stats(self.frame_times.clone());
        let fps = compute_fps_stats(&frame_stats);
        CaseResult {
            seed: self.case.seed,
            map_size: self.case.size.to_string(),
            renderer: self.case.renderer.to_string(),
            world_gen_ms: self.case.world_gen_ms,
            startup_ms: self.startup_ms.unwrap_or(0.0),
            rss_mb: self.rss_mb,
            frame_time_ms: frame_stats,
            fps,
        }
    }
}

/// Bevy plugin that registers the measurement system for one benchmark case.
pub(crate) struct BenchPlugin {
    pub case: BenchCase,
}

impl Plugin for BenchPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BenchState::new(self.case.clone()))
            .add_systems(PostUpdate, bench_update);
    }
}

/// Drives the camera path and collects frame-time samples; sends `AppExit` when done.
///
/// Scheduled in `PostUpdate` so it runs after `update_orbit_camera` (which is
/// in `Update`) — our transform write is the last word before the GPU frame.
#[allow(clippy::needless_pass_by_value)]
fn bench_update(
    mut state: ResMut<BenchState>,
    time: Res<Time>,
    mut camera_q: Query<&mut Transform, With<Camera3d>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    match state.phase {
        BenchPhase::WaitingForCamera => {
            if !camera_q.is_empty() {
                let startup_ms = state.app_start.elapsed().as_secs_f64() * 1000.0;
                state.startup_ms = Some(startup_ms);
                state.measure_start = Some(Instant::now());
                state.phase = BenchPhase::Measuring;
                tracing::info!(
                    startup_ms,
                    seed = state.case.seed,
                    size = %state.case.size,
                    "camera ready, starting measurement"
                );
            }
        }
        BenchPhase::Measuring => {
            let elapsed = state.measure_elapsed_secs();

            // Raw frame delta — skip dt=0 on the very first tick.
            let dt_ms = time.delta_secs_f64() * 1000.0;
            if dt_ms > 0.0 {
                state.frame_times.push(dt_ms);
            }

            // Sample RSS once at the 5 s mark.
            if !state.rss_sampled && elapsed >= 5.0 {
                state.rss_mb = read_rss_mb();
                state.rss_sampled = true;
            }

            // Drive camera along the scripted path.
            let (pivot, distance) = camera_path::scripted_camera(
                elapsed,
                state.case.map_world_width,
                state.case.map_world_height,
            );
            let transform = camera_path::orbit_transform(pivot, distance);
            for mut t in &mut camera_q {
                *t = transform;
            }

            // Finish when measurement window is full and at least one frame collected.
            let target = f64::from(state.case.measurement_secs);
            if elapsed >= target && !state.frame_times.is_empty() {
                state.phase = BenchPhase::Done;
                let result = state.to_case_result();
                let json =
                    serde_json::to_string(&result).expect("CaseResult serialisation must succeed");
                // stdout carries the JSON; the orchestrator reads it.
                println!("{json}");
                app_exit.write(AppExit::Success);
            }
        }
        BenchPhase::Done => {}
    }
}

/// Reads the resident set size from `/proc/self/status` (Linux only).
fn read_rss_mb() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: f64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024.0);
        }
    }
    None
}
