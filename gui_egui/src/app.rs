use assert_no_alloc::*;

#[cfg(debug_assertions)]
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

use core::{
    engine::Engine,
    modules::{Adsr, Biquad, Gain, NotePitch, Saw, SmoothCtrl},
};

use eframe::{egui, epi};

pub struct TemplateApp {
    engine: Engine,
    stream: cpal::Stream,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (mut engine, stream) = audio::start_engine();
        Self { engine, stream }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "ReDAW"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let Self { engine, .. } = self;
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!(
                "Bars: {} Beats: {}",
                engine.tempo.current_bars() as u64,
                engine.tempo.current_beats() as u64
            ));
            if ui.button("Play").clicked() {
                engine.set_play();
            }
            if ui.button("Pause").clicked() {
                engine.set_pause();
            }
        });

        engine.run_step();
        if engine.tempo.playing {
            ctx.request_repaint();
        }
    }
}
