mod editor;
mod audio;

use eframe::egui;
use egui::Widget;
use editor::WaveformEditor;
use audio::AudioEngine;

/// The main application struct for Audio Tinkertoy.
pub struct AudioTinkertoyApp {
    editor: WaveformEditor,
    audio_engine: Option<AudioEngine>,
    is_playing: bool,
}

impl AudioTinkertoyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            editor: WaveformEditor::new(),
            audio_engine: None,
            is_playing: false,
        }
    }
}

impl eframe::App for AudioTinkertoyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Top panel with title
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            ui.heading("Audio Tinkertoy");
        });

        // Left controls panel
        egui::Panel::left("controls_panel")
            .default_size(220.0)
            .show_inside(ui, |ui| {
                ui.heading("Controls");
                ui.separator();

                // Sample count control
                ui.label("Sample Points:");
                let old_count = self.editor.sample_count;
                ui.add(egui::Slider::new(&mut self.editor.sample_count, 2..=1024)
                    .text("samples"));
                if self.editor.sample_count != old_count {
                    self.editor.update_sample_count();
                }

                // Frequency control
                ui.label("Frequency (Hz):");
                ui.add(egui::Slider::new(&mut self.editor.playback_frequency, 20.0..=20000.0)
                    .text("Hz"));

                // Volume control
                ui.label("Volume:");
                ui.add(egui::Slider::new(&mut self.editor.volume, 0.0..=1.0)
                    .text("volume"));

                ui.separator();

                // Play/Stop buttons
                if self.is_playing {
                    if ui.button("\u{23F9} Stop").clicked() {
                        if let Some(ref mut engine) = self.audio_engine {
                            engine.stop();
                        }
                        self.is_playing = false;
                    }
                } else {
                    if ui.button("\u{25B6} Play").clicked() {
                        self.audio_engine = Some(AudioEngine::new());
                        if let Some(ref mut engine) = self.audio_engine {
                            engine.start(&self.editor.waveform, self.editor.playback_frequency, self.editor.volume);
                        }
                        self.is_playing = true;
                    }
                }

                if ui.button("\u{21BA} Reset to Sine").clicked() {
                    self.editor.reset_to_sine();
                }

                ui.separator();

                // Display current frequency
                ui.label(format!("Frequency: {:.1} Hz", self.editor.playback_frequency));
                ui.label(format!("Samples: {}", self.editor.sample_count));
                ui.label(format!("Volume: {:.2}", self.editor.volume));

                // Playback status
                if self.is_playing {
                    ui.label(egui::RichText::new("{25B6} Playing...").color(egui::Color32::from_rgba_premultiplied(0, 220, 100, 220)));
                } else {
                    ui.label(egui::RichText::new("{23F8} Stopped").color(egui::Color32::from_white_alpha(120)));
                }
            });

        // Main canvas panel
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Update audio in real-time if playing, passing the latest
            // frequency and volume so changes are reflected immediately.
            if self.is_playing {
                if let Some(ref engine) = self.audio_engine {
                    engine.update_waveform(&self.editor.waveform, self.editor.playback_frequency, self.editor.volume);
                }
            }

            (&mut self.editor).ui(ui);
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([600.0, 500.0])
            .with_title("Audio Tinkertoy"),
        ..Default::default()
    };

    eframe::run_native(
        "Audio Tinkertoy",
        native_options,
        Box::new(|cc| Ok(Box::new(AudioTinkertoyApp::new(cc)))),
    )
}
