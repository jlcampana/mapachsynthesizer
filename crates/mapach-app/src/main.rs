//! Mapachsynthesizer: sintetizador para niños.

use eframe::egui;
use mapach_engine::Command;
use mapach_engine::audio::{self, AudioOutput};

fn main() -> eframe::Result {
    let audio = match audio::start() {
        Ok(a) => {
            eprintln!(
                "Audio: {} · {} Hz · {} canales · bloque {}",
                a.device_name,
                a.sample_rate,
                a.channels,
                a.requested_buffer
                    .map_or("de serie".to_string(), |b| b.to_string())
            );
            Some(a)
        }
        Err(e) => {
            eprintln!("Sin audio: {e:#}");
            None
        }
    };

    let fullscreen = !std::env::args().any(|a| a == "--ventana");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Mapachsynthesizer")
            .with_fullscreen(fullscreen)
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Mapachsynthesizer",
        options,
        Box::new(|_cc| Ok(Box::new(App { audio, sonando: false }))),
    )
}

struct App {
    audio: Option<AudioOutput>,
    sonando: bool,
}

impl App {
    fn send(&mut self, cmd: Command) {
        if let Some(a) = &mut self.audio {
            a.send(cmd);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let events = ui.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Key {
                key,
                pressed,
                repeat: false,
                ..
            } = ev
            {
                match key {
                    egui::Key::A if pressed => {
                        self.sonando = true;
                        self.send(Command::NoteOn { note: 69 });
                    }
                    egui::Key::A => {
                        self.sonando = false;
                        self.send(Command::NoteOff { note: 69 });
                    }
                    egui::Key::Escape if pressed => {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    _ => {}
                }
            }
        }

        let fondo = if self.sonando {
            egui::Color32::from_rgb(0xFF, 0x3B, 0x3B)
        } else {
            egui::Color32::from_rgb(0xFF, 0xC2, 0x1A)
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(fondo))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.35);
                    ui.label(
                        egui::RichText::new("¡Pulsa A!")
                            .size(96.0)
                            .color(egui::Color32::from_rgb(0x1C, 0x12, 0x30)),
                    );
                    if let Some(a) = &self.audio {
                        ui.label(format!(
                            "{} Hz · bloque {} · latencia {:.1} ms",
                            a.sample_rate,
                            a.block_frames(),
                            a.latency_ms()
                        ));
                    } else {
                        ui.label("Sin audio");
                    }
                });
            });
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
}
