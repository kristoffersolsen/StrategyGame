//! Sweep configuration panel.

use eframe::egui;

use crate::sweep::SWEEPABLE_PARAMS;

/// Live sweep configuration held by the tuning app state.
#[derive(Debug, Clone)]
pub(crate) struct SweepConfig {
    pub(crate) param: String,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) steps: usize,
    pub(crate) extra_seeds: Vec<u64>,
}

impl Default for SweepConfig {
    fn default() -> Self {
        Self {
            param: SWEEPABLE_PARAMS[0].to_owned(),
            min: 0.0,
            max: 1.0,
            steps: 5,
            extra_seeds: vec![],
        }
    }
}

/// Draws the sweep configuration form into `ui`.
pub(crate) fn show(ui: &mut egui::Ui, config: &mut SweepConfig) {
    egui::CollapsingHeader::new("Sweep")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Parameter");
                egui::ComboBox::from_id_salt("sweep_param")
                    .selected_text(config.param.as_str())
                    .show_ui(ui, |ui| {
                        for &name in SWEEPABLE_PARAMS {
                            ui.selectable_value(&mut config.param, name.to_owned(), name);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Min");
                ui.add(
                    egui::DragValue::new(&mut config.min)
                        .speed(0.001)
                        .max_decimals(4),
                );
                ui.label("Max");
                ui.add(
                    egui::DragValue::new(&mut config.max)
                        .speed(0.001)
                        .max_decimals(4),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Steps");
                ui.add(egui::DragValue::new(&mut config.steps).range(2..=20));
            });
            ui.horizontal(|ui| {
                ui.label("Extra seeds");
                let mut seed_str = seeds_to_string(&config.extra_seeds);
                if ui
                    .add(egui::TextEdit::singleline(&mut seed_str).desired_width(120.0))
                    .changed()
                {
                    config.extra_seeds = parse_seeds(&seed_str);
                }
            });
        });
}

fn seeds_to_string(seeds: &[u64]) -> String {
    seeds
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_seeds(s: &str) -> Vec<u64> {
    s.split(',')
        .filter_map(|tok| tok.trim().parse::<u64>().ok())
        .collect()
}
