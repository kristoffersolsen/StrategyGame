//! Top-level application state and frame loop.

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;
use worldgen::{BasicParams, MapSize, TuningParams, WorldParams};

use crate::figure_panel;
use crate::figures::ALL_FIGURES;
use crate::params_panel;
use crate::runner::{RunRequest, Runner, RunnerResult, WorldOutput};
use crate::sweep_panel::{self, SweepConfig};

/// Root application struct registered with eframe.
pub(crate) struct MaptuneApp {
    state: AppState,
}

struct AppState {
    params: TuningParams,
    seed: u64,
    map_size: MapSize,
    hex_size: u32,
    sweep_config: SweepConfig,
    show_sweep: bool,
    selected_figure: usize,
    last_result: Option<WorldOutput>,
    sweep_results: Vec<(f64, WorldOutput)>,
    /// Cached textures for the single-run view, one slot per figure.
    single_textures: Vec<Option<egui::TextureHandle>>,
    /// Cached textures for the sweep view, one slot per sweep step.
    sweep_textures: Vec<Option<egui::TextureHandle>>,
    status: String,
    running: bool,
    run_start: Option<Instant>,
    runner: Runner,
}

impl MaptuneApp {
    /// Creates the app and spawns the background worker thread.
    pub(crate) fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState {
                params: TuningParams::defaults(),
                seed: 1,
                map_size: MapSize::Medium,
                hex_size: 8,
                sweep_config: SweepConfig::default(),
                show_sweep: false,
                selected_figure: 0,
                last_result: None,
                sweep_results: vec![],
                single_textures: vec![],
                sweep_textures: vec![],
                status: "Ready".to_owned(),
                running: false,
                run_start: None,
                runner: Runner::new(),
            },
        }
    }
}

impl eframe::App for MaptuneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let s = &mut self.state;

        // Poll worker results.
        if let Some(result) = s.runner.try_recv() {
            let elapsed = s
                .run_start
                .take()
                .map_or(0.0, |t| t.elapsed().as_secs_f32());
            s.running = false;
            s.single_textures.clear();
            s.sweep_textures.clear();

            match result {
                RunnerResult::Single(boxed) => match *boxed {
                    Ok(output) => {
                        s.sweep_results.clear();
                        s.status = format!("Done in {elapsed:.2}s");
                        s.last_result = Some(output);
                    }
                    Err(e) => {
                        s.status = format!("Error: {e}");
                    }
                },
                RunnerResult::Sweep(outputs) => {
                    s.last_result = None;
                    let mut had_error = false;
                    s.sweep_results = outputs
                        .into_iter()
                        .filter_map(|(val, res)| match res {
                            Ok(o) => Some((val, o)),
                            Err(e) => {
                                had_error = true;
                                s.status = format!("Sweep error: {e}");
                                None
                            }
                        })
                        .collect();
                    if !had_error {
                        s.status = format!(
                            "Sweep done in {elapsed:.2}s ({} steps)",
                            s.sweep_results.len()
                        );
                    }
                }
            }
        }

        // Re-request repaint while a run is in flight.
        if s.running {
            ctx.request_repaint();
        }

        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                left_panel(s, ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let fig_changed = figure_panel::show(
                ui,
                &mut s.selected_figure,
                s.last_result.as_ref(),
                &s.sweep_results,
                &mut s.single_textures,
                &mut s.sweep_textures,
            );
            if fig_changed {
                s.single_textures.clear();
                s.sweep_textures.clear();
            }
        });
    }
}

fn left_panel(s: &mut AppState, ui: &mut egui::Ui) {
    // Figure selector — narrow vertical list.
    ui.vertical(|ui| {
        egui::ScrollArea::vertical()
            .id_salt("left_panel_scroll")
            .max_height(ui.available_height() - 160.0)
            .show(ui, |ui| {
                params_panel::show(
                    ui,
                    &mut s.params,
                    &mut s.seed,
                    &mut s.map_size,
                    &mut s.hex_size,
                );
                ui.separator();
                ui.checkbox(&mut s.show_sweep, "Sweep mode");
                if s.show_sweep {
                    sweep_panel::show(ui, &mut s.sweep_config);
                }
            });

        ui.separator();
        action_buttons(s, ui);
        ui.label(&s.status);
    });
}

fn action_buttons(s: &mut AppState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let run_label = if s.running { "Running…" } else { "Run" };
        if ui
            .add_enabled(!s.running, egui::Button::new(run_label))
            .clicked()
        {
            start_single_run(s);
        }

        if s.show_sweep
            && ui
                .add_enabled(!s.running, egui::Button::new("Run Sweep"))
                .clicked()
        {
            start_sweep_run(s);
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Export RON").clicked() {
            export_ron(s);
        }
        if ui.button("Load RON").clicked() {
            load_ron(s);
        }
        if ui.button("Reset all").clicked() {
            s.params = TuningParams::defaults();
        }
    });
}

fn make_world_params(s: &AppState) -> WorldParams {
    let mut p = WorldParams::defaults();
    p.seed = s.seed;
    p.basic = BasicParams {
        map_size: s.map_size,
        ..BasicParams::default()
    };
    p.tuning = s.params.clone();
    p
}

fn start_single_run(s: &mut AppState) {
    let params = make_world_params(s);
    s.runner.send(RunRequest::Single {
        params,
        hex_size: s.hex_size,
    });
    s.running = true;
    s.run_start = Some(Instant::now());
    "Running…".clone_into(&mut s.status);
    s.single_textures.clear();
    s.sweep_textures.clear();
    s.sweep_results.clear();
}

fn start_sweep_run(s: &mut AppState) {
    let base = make_world_params(s);
    let sc = &s.sweep_config;
    s.runner.send(RunRequest::Sweep {
        base_params: base,
        param_name: sc.param.clone(),
        min: sc.min,
        max: sc.max,
        steps: sc.steps,
        extra_seeds: sc.extra_seeds.clone(),
        hex_size: s.hex_size,
    });
    s.running = true;
    s.run_start = Some(Instant::now());
    s.status = format!("Sweeping {}…", sc.param);
    s.single_textures.clear();
    s.sweep_textures.clear();
    s.last_result = None;
}

fn export_ron(s: &AppState) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name("params.ron")
        .add_filter("RON", &["ron"])
        .save_file()
    else {
        return;
    };
    match ron::to_string(&s.params) {
        Ok(text) => {
            if let Err(e) = std::fs::write(&path, &text) {
                eprintln!("export RON failed: {e}");
            }
        }
        Err(e) => eprintln!("RON serialise failed: {e}"),
    }
}

fn load_ron(s: &mut AppState) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("RON", &["ron"])
        .pick_file()
    else {
        return;
    };
    load_ron_from_path(s, &path);
}

fn load_ron_from_path(s: &mut AppState, path: &PathBuf) {
    match std::fs::read_to_string(path) {
        Ok(text) => match ron::from_str::<TuningParams>(&text) {
            Ok(loaded) => {
                s.params = loaded;
                s.single_textures.clear();
                s.sweep_textures.clear();
                s.status = format!("Loaded {}", path.display());
            }
            Err(e) => {
                s.status = format!("RON parse error: {e}");
            }
        },
        Err(e) => {
            s.status = format!("Read error: {e}");
        }
    }
}

/// Returns the label for figure `idx` (used in window title etc.).
#[allow(dead_code)]
fn figure_label(idx: usize) -> &'static str {
    ALL_FIGURES.get(idx).map_or("", |f| f.label)
}
