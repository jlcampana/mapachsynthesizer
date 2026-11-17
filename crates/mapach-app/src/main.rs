//! Mapachsynthesizer: sintetizador para niños.

mod ajustes;
mod captura;
mod keyboard;
mod state;
mod theme;
mod ui;

use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use mapach_engine::Command;
use mapach_engine::audio::{self, AudioOutput};
use mapach_input::{Control, Event, Knob, mando};

use keyboard::KeyAction;
use state::State;

fn main() -> eframe::Result {
    let captura = captura::Captura::from_args();
    let audio = match if captura.is_some() {
        Err(anyhow::anyhow!("modo captura"))
    } else {
        audio::start()
    } {
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
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Mapachsynthesizer")
            .with_app_id("mapachsynthesizer")
            .with_fullscreen(fullscreen)
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    if captura.is_some() {
        // En modo captura no esperamos al compositor (con la pantalla bloqueada no pinta).
        options.wgpu_options.surface.present_mode = eframe::wgpu::PresentMode::AutoNoVsync;
    }
    eframe::run_native(
        "Mapachsynthesizer",
        options,
        Box::new(|cc| {
            theme::fuentes(&cc.egui_ctx);
            Ok(Box::new(App::new(audio, cc.egui_ctx.clone(), captura)))
        }),
    )
}

struct App {
    audio: Option<AudioOutput>,
    state: State,
    mando_rx: Receiver<Event>,
    mando_status_rx: Receiver<mando::Status>,
    mando_status: mando::Status,
    papa: bool,
    /// Tecla de acorde pulsada con el ratón o el dedo.
    pointer_chord: Option<u8>,
    focused: bool,
    /// Giro acumulado de la rueda del ratón.
    wheel: f32,
    captura: Option<captura::Captura>,
    /// Últimos ajustes guardados y cuándo se miró por última vez.
    saved: ajustes::Ajustes,
    last_save_check: f64,
}

impl App {
    fn new(
        audio: Option<AudioOutput>,
        ctx: egui::Context,
        captura: Option<captura::Captura>,
    ) -> Self {
        let (tx, mando_rx) = mpsc::channel();
        let (stx, mando_status_rx) = mpsc::channel();
        mando::spawn(tx, stx, move || ctx.request_repaint());
        let mut state = State::default();
        // En modo captura no se leen ni se guardan los ajustes del niño.
        if captura.is_none()
            && let Some(a) = ajustes::Ajustes::load()
        {
            a.apply(&mut state);
        }
        let saved = ajustes::Ajustes::from_state(&state);
        let mut app = Self {
            audio,
            state,
            mando_rx,
            mando_status_rx,
            mando_status: mando::Status::Buscando,
            papa: false,
            pointer_chord: None,
            focused: true,
            wheel: 0.0,
            captura,
            saved,
            last_save_check: 0.0,
        };
        let cmds = app.state.initial_commands();
        app.send_all(cmds);
        app
    }

    fn send_all(&mut self, cmds: Vec<Command>) {
        if let Some(a) = &mut self.audio {
            for c in cmds {
                a.send(c);
            }
        }
    }

    fn event(&mut self, ev: Event, now: f64) {
        let cmds = self.state.handle(ev, now);
        self.send_all(cmds);
    }

    /// Suelta todo (al perder el foco, para que no se queden notas colgadas).
    fn release_all(&mut self) {
        self.state.release_all();
        self.pointer_chord = None;
        self.send_all(vec![Command::AllNotesOff]);
    }

    /// Guarda los ajustes si han cambiado (como mucho cada 2 s).
    fn autosave(&mut self, now: f64, force: bool) {
        if self.captura.is_some() || (!force && now - self.last_save_check < 2.0) {
            return;
        }
        self.last_save_check = now;
        let a = ajustes::Ajustes::from_state(&self.state);
        if a != self.saved {
            match a.save() {
                Ok(()) => self.saved = a,
                Err(e) => eprintln!("No se pudieron guardar los ajustes: {e}"),
            }
        }
    }

    fn papa_lines(&self) -> Vec<String> {
        let mut v = Vec::new();
        match &self.audio {
            Some(a) => {
                v.push(format!("Audio: {}", a.device_name));
                v.push(format!(
                    "{} Hz · bloque {} · latencia {:.1} ms",
                    a.sample_rate,
                    a.block_frames(),
                    a.latency_ms()
                ));
                v.push(format!("Voces sonando: {}", a.voices()));
            }
            None => v.push("Sin audio".into()),
        }
        v.push(format!("Mando: {}", status_text(self.mando_status)));
        v.push(format!(
            "Volumen máximo: {} dBFS",
            mapach_engine::DEFAULT_MAX_DB
        ));
        v.push(format!("Versión {}", env!("CARGO_PKG_VERSION")));
        v
    }
}

fn status_text(s: mando::Status) -> &'static str {
    match s {
        mando::Status::Buscando => "no está",
        mando::Status::Conectado => "conectado",
        mando::Status::SinPermiso => "sin permiso (falta la regla udev)",
    }
}

impl eframe::App for App {
    fn on_exit(&mut self) {
        self.autosave(f64::MAX, true);
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.captura.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);

        if let Some(cap) = &mut self.captura
            && cap.tick(&ctx)
        {
            for ev in cap.events() {
                self.state.handle(ev, now);
            }
        }

        while let Ok(s) = self.mando_status_rx.try_recv() {
            self.mando_status = s;
        }
        while let Ok(ev) = self.mando_rx.try_recv() {
            if !self.papa {
                self.event(ev, now);
            }
        }

        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if self.focused && !focused {
            self.release_all();
        }
        self.focused = focused;

        let events = ctx.input(|i| i.events.clone());
        let mut scroll = 0.0;
        for ev in events {
            if let egui::Event::MouseWheel { unit, delta, .. } = ev {
                scroll += match unit {
                    egui::MouseWheelUnit::Point => delta.y / 40.0,
                    egui::MouseWheelUnit::Line => delta.y,
                    egui::MouseWheelUnit::Page => delta.y * 4.0,
                };
            }
            if let egui::Event::Key {
                key,
                pressed,
                repeat: false,
                modifiers,
                ..
            } = ev
            {
                if self.papa {
                    if pressed && key == egui::Key::Q {
                        self.autosave(now, true);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if pressed
                        && (key == egui::Key::Escape
                            || (key == egui::Key::P && modifiers.ctrl && modifiers.alt))
                    {
                        self.papa = false;
                    }
                    continue;
                }
                match keyboard::translate(key, modifiers, pressed) {
                    Some(KeyAction::Papa) => {
                        self.release_all();
                        self.papa = true;
                    }
                    Some(KeyAction::Event(e)) => self.event(e, now),
                    None => {}
                }
            }
        }

        // Rueda del ratón = perilla mágica.
        self.wheel += scroll;
        while self.wheel.abs() >= 1.0 {
            let d = self.wheel.signum();
            self.wheel -= d;
            if !self.papa {
                self.event(Event::Turn(Knob::Magica, d as i32), now);
            }
        }

        let screen = ui.max_rect();
        let painter = ui.painter_at(screen);
        let canvas = ui::Canvas::new(&painter, screen);

        // Ratón o dedo sobre las teclas de acorde.
        let (down, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));
        let target = if down && !self.papa {
            pos.and_then(|p| ui::chord_at(canvas.to_canvas(p)))
        } else {
            None
        };
        if target != self.pointer_chord {
            if let Some(b) = self.pointer_chord {
                self.event(Event::Release(Control::Chord(b)), now);
            }
            if let Some(b) = target {
                self.event(Event::Press(Control::Chord(b)), now);
            }
            self.pointer_chord = target;
        }

        let (peak, status) = match &self.audio {
            Some(a) => (
                a.peak(),
                format!(
                    "mando: {} · {:.1} ms",
                    status_text(self.mando_status),
                    a.latency_ms()
                ),
            ),
            None => (0.0, "sin audio".into()),
        };
        ui::draw(
            &canvas,
            &self.state,
            &ui::Extra {
                time: now,
                peak,
                status: &status,
            },
        );
        if self.papa {
            ui::draw_papa(&canvas, &self.papa_lines());
        }

        self.autosave(now, false);

        // Animación del osciloscopio mientras suena algo.
        if peak > 1e-4 || self.state.current.is_some() || !self.state.melody.is_empty() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}
