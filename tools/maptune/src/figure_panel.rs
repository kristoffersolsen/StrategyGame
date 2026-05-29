//! Figure selector and inline image display.

use eframe::egui;

use crate::figures::ALL_FIGURES;
use crate::runner::WorldOutput;

/// Draws the figure selector tabs and the rendered image.
///
/// Returns `true` if the selected figure index changed (caller should
/// invalidate cached textures).
pub(crate) fn show(
    ui: &mut egui::Ui,
    selected: &mut usize,
    single_result: Option<&WorldOutput>,
    sweep_results: &[(f64, WorldOutput)],
    textures: &mut Vec<Option<egui::TextureHandle>>,
    sweep_textures: &mut Vec<Option<egui::TextureHandle>>,
) -> bool {
    let mut changed = false;

    // Figure selector row at the top.
    egui::ScrollArea::horizontal()
        .id_salt("fig_tabs")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, fig) in ALL_FIGURES.iter().enumerate() {
                    if ui.selectable_label(*selected == i, fig.label).clicked() && *selected != i {
                        *selected = i;
                        changed = true;
                    }
                }
            });
        });

    ui.separator();

    if sweep_results.is_empty() {
        show_single(ui, *selected, single_result, textures);
    } else {
        show_sweep(ui, *selected, sweep_results, sweep_textures);
    }

    changed
}

fn show_single(
    ui: &mut egui::Ui,
    selected: usize,
    result: Option<&WorldOutput>,
    textures: &mut Vec<Option<egui::TextureHandle>>,
) {
    let Some(world) = result else {
        ui.centered_and_justified(|ui| {
            ui.label("Press Run to generate a world.");
        });
        return;
    };

    // Ensure the textures vec is large enough.
    if textures.len() < ALL_FIGURES.len() {
        textures.resize_with(ALL_FIGURES.len(), || None);
    }

    // Lazy-render the selected figure.
    if textures[selected].is_none() {
        let fig = &ALL_FIGURES[selected];
        let rgb = (fig.render)(world);
        let [w, h] = [rgb.width() as usize, rgb.height() as usize];
        let pixels: Vec<egui::Color32> = rgb
            .pixels()
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        let color_image = egui::ColorImage {
            size: [w, h],
            pixels,
        };
        textures[selected] = Some(ui.ctx().load_texture(
            ALL_FIGURES[selected].id,
            color_image,
            egui::TextureOptions::LINEAR,
        ));
    }

    if let Some(tex) = &textures[selected] {
        let available = ui.available_size();
        ui.add(
            egui::Image::new(tex)
                .fit_to_exact_size(available)
                .sense(egui::Sense::hover()),
        );
    }
}

fn show_sweep(
    ui: &mut egui::Ui,
    selected: usize,
    results: &[(f64, WorldOutput)],
    textures: &mut Vec<Option<egui::TextureHandle>>,
) {
    // Ensure textures vec covers all sweep steps.
    if textures.len() < results.len() {
        textures.resize_with(results.len(), || None);
    }

    // Lazy-render each step.
    for (i, (_val, world)) in results.iter().enumerate() {
        if textures[i].is_none() {
            let fig = &ALL_FIGURES[selected];
            let rgb = (fig.render)(world);
            let [w, h] = [rgb.width() as usize, rgb.height() as usize];
            let pixels: Vec<egui::Color32> = rgb
                .pixels()
                .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                .collect();
            let color_image = egui::ColorImage {
                size: [w, h],
                pixels,
            };
            textures[i] = Some(ui.ctx().load_texture(
                format!("{}_sweep_{i}", ALL_FIGURES[selected].id),
                color_image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    // Scrollable grid.
    let cell_size = 320.0_f32;
    egui::ScrollArea::both()
        .id_salt("sweep_scroll")
        .show(ui, |ui| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let cols = ((ui.available_width() / cell_size) as usize).max(1);
            egui::Grid::new("sweep_grid")
                .num_columns(cols)
                .spacing([4.0, 4.0])
                .show(ui, |ui| {
                    for (i, (val, _world)) in results.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.label(format!("{val:.4}"));
                            if let Some(tex) = &textures[i] {
                                ui.add(
                                    egui::Image::new(tex)
                                        .fit_to_exact_size(egui::Vec2::splat(cell_size))
                                        .sense(egui::Sense::hover()),
                                );
                            }
                        });
                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
}
