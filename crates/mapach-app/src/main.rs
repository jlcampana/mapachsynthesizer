//! Mapachsynthesizer: sintetizador para niños.

mod ajustes;
mod aprender;
mod canciones;
mod captura;
mod keyboard;
mod papa;
mod state;
mod taller;
mod theme;
mod ui;

use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use mapach_engine::Command;
use mapach_engine::audio::{self, AudioOutput};
use mapach_input::{Control, Event, Knob, mando};

use keyboard::KeyAction;
use state::{Action, State};

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
    mando_shared: std::sync::Arc<std::sync::Mutex<mando::Shared>>,
    mando_raw: Receiver<u16>,
    /// Asistente "Aprender mando" en marcha.
    learn: Option<aprender::Learn>,
    papa: bool,
    papa_sel: usize,
    /// Tecla de acorde pulsada con el ratón o el dedo.
    pointer_chord: Option<u8>,
    focused: bool,
    /// Giro acumulado de la rueda del ratón.
    wheel: f32,
    captura: Option<captura::Captura>,
    /// Últimos ajustes guardados y cuándo se miró por última vez.
    saved: ajustes::Ajustes,
    last_save_check: f64,
    /// Canciones (solo si hay audio: sin motor no hay pistas que guardar).
    songs: Option<canciones::Songs>,
    taller: taller::Taller,
}

/// Para el taller sin tarjeta de sonido: las órdenes se pierden.
struct NoAudio;

impl canciones::Link for NoAudio {
    fn send(&mut self, _: Command) {}
    fn reply(&mut self) -> Option<mapach_engine::Reply> {
        None
    }
    fn capacity(&self) -> usize {
        0
    }
}

impl App {
    fn new(
        audio: Option<AudioOutput>,
        ctx: egui::Context,
        captura: Option<captura::Captura>,
    ) -> Self {
        let (tx, mando_rx) = mpsc::channel();
        let (stx, mando_status_rx) = mpsc::channel();
        let (rtx, mando_raw) = mpsc::channel();
        let mando_shared = std::sync::Arc::new(std::sync::Mutex::new(mando::Shared {
            mapping: aprender::load(),
            learning: false,
        }));
        mando::spawn(tx, stx, rtx, mando_shared.clone(), move || {
            ctx.request_repaint()
        });
        let mut state = State::default();
        let mut last_song = None;
        let mut instrument = None;
        // En modo captura no se leen ni se guardan los ajustes del niño.
        if captura.is_none()
            && let Some(a) = ajustes::Ajustes::load()
        {
            a.apply(&mut state);
            if !a.cancion.is_empty() {
                last_song = Some(std::path::PathBuf::from(&a.cancion));
            }
            if !a.mi_sample.is_empty() {
                instrument = Some(std::path::PathBuf::from(&a.mi_sample));
            }
        }
        let saved = ajustes::Ajustes::from_state(&state);
        let rate = audio.as_ref().map_or(48_000, |a| a.sample_rate);
        let mut songs = Some(canciones::Songs::new(
            canciones::Current::new_random(),
            rate,
        ));
        if let (Some(cap), Some(songs)) = (&captura, &mut songs) {
            cap.fake_songs(songs);
        }
        // Al arrancar se abre la última canción.
        if let (Some(songs), Some(dir), true) = (&mut songs, last_song, audio.is_some())
            && dir.join("cancion.ron").exists()
        {
            songs.load_folder(dir);
        }
        let mut app = Self {
            audio,
            state,
            mando_rx,
            mando_status_rx,
            mando_status: mando::Status::Buscando,
            mando_shared,
            mando_raw,
            learn: None,
            papa: false,
            papa_sel: 0,
            pointer_chord: None,
            focused: true,
            wheel: 0.0,
            captura,
            saved,
            last_save_check: 0.0,
            songs,
            taller: taller::Taller::new(),
        };
        if let Some(cap) = &app.captura {
            cap.fake_taller(&mut app.taller);
            app.papa = cap.demo == 14;
        }
        // Los samples puestos en la batería.
        let golpes = app.state.drum_samples.clone();
        for (d, path) in golpes.iter().enumerate() {
            if let Some(p) = path {
                match taller::load_instrument(std::path::Path::new(p)) {
                    Some((data, _)) => app.send_all(vec![Command::SetDrumSample {
                        drum: d as u8,
                        data: Some(data),
                    }]),
                    None => app.state.drum_samples[d] = None,
                }
            }
        }
        // El sample que se usaba como instrumento.
        if let Some(path) = instrument
            && let Some((data, root)) = taller::load_instrument(&path)
        {
            app.state.instrument = Some(path.to_string_lossy().into_owned());
            app.send_all(vec![Command::SetSample {
                slot: 0,
                data,
                root,
            }]);
        }
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
        let mut a = ajustes::Ajustes::from_state(&self.state);
        if let Some(dir) = self.songs.as_ref().and_then(|s| s.current.carpeta.as_ref()) {
            a.cancion = dir.to_string_lossy().into_owned();
        }
        if a != self.saved {
            match a.save() {
                Ok(()) => self.saved = a,
                Err(e) => eprintln!("No se pudieron guardar los ajustes: {e}"),
            }
        }
    }

    /// Canciones: respuestas del motor, resultados de los hilos, acciones y autoguardado.
    fn songs_tick(&mut self, now: f64) {
        let (taller_actions, actions): (Vec<Action>, Vec<Action>) =
            self.state.take_actions().into_iter().partition(|a| {
                matches!(
                    a,
                    Action::OpenTaller
                        | Action::SelectSample(_)
                        | Action::MicToggle
                        | Action::PreviewSample
                        | Action::Destroy(_)
                        | Action::UndoSample
                        | Action::UseSample
                        | Action::DrumSample(_)
                )
            });
        for a in taller_actions {
            self.taller_action(a, now);
        }
        if let Some(i) = self.taller.tick(now) {
            self.state.sample_count = self.taller.list.len();
            self.state.sample_sel = i;
            self.state.show_toast("🎤 ¡Grabado!");
        }
        let (Some(songs), Some(audio)) = (&mut self.songs, &mut self.audio) else {
            for a in actions {
                match a {
                    Action::OpenSongs => {
                        if let Some(s) = &self.songs {
                            self.state.song_count = s.list.len();
                        }
                    }
                    _ => self
                        .state
                        .show_toast("Sin audio no se pueden guardar canciones"),
                }
            }
            return;
        };
        Self::songs_actions(&mut self.state, songs, audio, actions, now);
    }

    fn songs_actions(
        state: &mut State,
        songs: &mut canciones::Songs,
        audio: &mut AudioOutput,
        actions: Vec<Action>,
        now: f64,
    ) {
        songs.pump_replies(audio);
        while let Some(done) = songs.poll() {
            match done {
                canciones::Done::Loaded(song, dir) => {
                    let name = song.nombre.clone();
                    songs.apply_loaded(*song, dir, state, audio);
                    state.show_toast(format!("🎵 {name}"));
                }
                canciones::Done::Saved(_) => songs.last_saved = Some(now),
                canciones::Done::Exported(p) => {
                    let file = p.file_name().map(|f| f.to_string_lossy().into_owned());
                    state.show_toast(format!(
                        "✔ {} en Música › Exportadas",
                        file.unwrap_or_default()
                    ));
                }
                canciones::Done::Failed(e) => {
                    eprintln!("Canciones: {e}");
                    state.show_toast("Algo ha fallado al guardar 😕");
                }
            }
        }
        for a in actions {
            match a {
                Action::OpenSongs => {
                    songs.refresh_list();
                    state.song_count = songs.list.len();
                }
                Action::Load(i) => songs.begin_load(i),
                Action::New => {
                    songs.new_song(state, audio);
                    state.show_toast(format!("✨ {}", songs.current.nombre));
                }
                Action::Export(f) => {
                    if state.looper.is_empty() {
                        state.show_toast("Primero graba algo en el looper (Espacio)");
                    } else if !songs.is_busy() {
                        songs.begin_save(state, audio, Some(f));
                    }
                }
                // Las del taller van por otro lado.
                _ => {}
            }
        }
        songs.autosave(now, state.looper, state, audio);
    }

    /// Acciones del taller de samples.
    fn taller_action(&mut self, a: Action, now: f64) {
        let mut no_audio = NoAudio;
        let link: &mut dyn canciones::Link = match &mut self.audio {
            Some(a) => a,
            None => &mut no_audio,
        };
        let t = &mut self.taller;
        match a {
            Action::OpenTaller => {
                if let Some(msg) = t.open() {
                    self.state.show_toast(msg);
                }
                self.state.sample_count = t.list.len();
                self.state.sample_sel = 0;
                t.select(0);
            }
            Action::SelectSample(i) => {
                t.select(i);
                t.preview(link);
            }
            Action::PreviewSample => {
                if self.state.screen == state::Screen::Taller {
                    t.preview(link);
                } else {
                    // Al salir del taller, deja de sonar.
                    t.stop_preview(link);
                    if t.is_recording() {
                        t.mic_toggle(now).ok();
                    }
                }
            }
            Action::Destroy(b) => match t.destroy(b, link) {
                Some(name) => self.state.show_toast(format!("💥 {name}")),
                None => self.state.show_toast("Primero elige o graba un sonido"),
            },
            Action::UndoSample => {
                if t.undo(link) {
                    self.state.show_toast("↶ Deshecho");
                }
            }
            Action::MicToggle => match t.mic_toggle(now) {
                Ok(Some(i)) => {
                    self.state.sample_count = t.list.len();
                    self.state.sample_sel = i;
                    self.state.show_toast("🎤 ¡Grabado!");
                }
                Ok(None) if t.is_recording() => {
                    self.state.show_toast("🔴 Grabando… (Espacio para parar)")
                }
                Ok(None) => self.state.show_toast("No se ha oído nada 🤫"),
                Err(e) => {
                    eprintln!("Micro: {e}");
                    self.state.show_toast("No encuentro el micrófono 🎤");
                }
            },
            Action::DrumSample(b) => {
                let Some((data, _, name, path)) = t.for_instrument() else {
                    self.state.show_toast("Primero elige o graba un sonido");
                    return;
                };
                let path = path.to_string_lossy().into_owned();
                let slot = &mut self.state.drum_samples[b as usize];
                if slot.as_deref() == Some(&path) {
                    // Otra vez el mismo: vuelve el golpe de serie.
                    *slot = None;
                    link.send(Command::SetDrumSample {
                        drum: b,
                        data: None,
                    });
                    self.state
                        .show_toast(format!("🥁 La tecla {} vuelve a su golpe", b + 1));
                } else {
                    *slot = Some(path);
                    link.send(Command::SetDrumSample {
                        drum: b,
                        data: Some(data),
                    });
                    self.state.show_toast(format!(
                        "🥁 La tecla {} de la batería suena a «{name}»",
                        b + 1
                    ));
                }
            }
            Action::UseSample => {
                if let Some((data, root, name, path)) = t.for_instrument() {
                    link.send(Command::SetSample {
                        slot: 0,
                        data,
                        root,
                    });
                    let cmds = self
                        .state
                        .use_instrument(path.to_string_lossy().into_owned());
                    for c in cmds {
                        link.send(c);
                    }
                    self.state
                        .show_toast(format!("🎤 Tus acordes suenan a «{name}»"));
                } else {
                    self.state.show_toast("Primero elige o graba un sonido");
                }
            }
            _ => {}
        }
    }

    /// Empieza o termina el asistente "Aprender mando".
    fn set_learning(&mut self, on: bool, mapping: Option<mando::Mapping>) {
        if let Ok(mut sh) = self.mando_shared.lock() {
            sh.learning = on;
            if let Some(m) = mapping {
                sh.mapping = m;
            }
        }
        self.learn = on.then(aprender::Learn::new);
        if on {
            self.papa = false;
        }
    }

    /// Teclas en el modo papá: ↑↓ eligen, ←→ cambian.
    fn papa_key(&mut self, key: egui::Key) {
        let n = papa::PapaItem::ALL.len();
        let d = match key {
            egui::Key::ArrowUp => {
                self.papa_sel = (self.papa_sel + n - 1) % n;
                return;
            }
            egui::Key::ArrowDown => {
                self.papa_sel = (self.papa_sel + 1) % n;
                return;
            }
            egui::Key::ArrowLeft => -1.0,
            egui::Key::ArrowRight | egui::Key::Enter => 1.0,
            _ => return,
        };
        match papa::PapaItem::ALL[self.papa_sel] {
            papa::PapaItem::VolumenMaximo => {
                let (lo, hi) = papa::MAX_DB_RANGE;
                self.state.max_db = (self.state.max_db + d).clamp(lo, hi);
                let c = Command::SetParam(mapach_engine::Param::MaxLevelDb(self.state.max_db));
                self.send_all(vec![c]);
            }
            papa::PapaItem::Nombres => self.state.show_names = !self.state.show_names,
            papa::PapaItem::Arranque => {
                let on = !papa::autostart_enabled();
                if let Err(e) = papa::set_autostart(on) {
                    eprintln!("Arranque automático: {e}");
                }
            }
            papa::PapaItem::Aprender => {
                if self.mando_status == mando::Status::Conectado {
                    self.set_learning(true, None);
                } else {
                    self.state.show_toast("Conecta el mando primero");
                }
            }
            papa::PapaItem::MandoDeSerie => {
                let _ = aprender::path().map(std::fs::remove_file);
                self.set_learning(false, Some(mando::Mapping::default()));
                self.state
                    .show_toast("🎮 Mando como de fábrica de Mapachsynthesizer");
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
            "Versión {} · ⭐ {} estrellas",
            env!("CARGO_PKG_VERSION"),
            self.state.stars
        ));
        v.push(String::new());
        let si_no = |b: bool| if b { "sí" } else { "no" };
        for (i, item) in papa::PapaItem::ALL.iter().enumerate() {
            let value = match item {
                papa::PapaItem::VolumenMaximo => format!("{:.0} dBFS", self.state.max_db),
                papa::PapaItem::Nombres => si_no(self.state.show_names).into(),
                papa::PapaItem::Arranque => si_no(papa::autostart_enabled()).into(),
                papa::PapaItem::Aprender | papa::PapaItem::MandoDeSerie => "Intro".into(),
            };
            let mark = if i == self.papa_sel { "▶" } else { "  " };
            v.push(format!("{mark} {}: ◀ {value} ▶", item.label()));
        }
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

        while let Ok(code) = self.mando_raw.try_recv() {
            if let Some(l) = &mut self.learn
                && let Some(m) = l.code(code)
            {
                if let Err(e) = aprender::save(&m) {
                    eprintln!("No se pudo guardar el mando: {e}");
                }
                self.set_learning(false, Some(m));
                self.state.show_toast("🎮 ¡Mando aprendido!");
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

        let looper = self
            .audio
            .as_ref()
            .map(|a| a.looper())
            .unwrap_or(self.state.looper);
        let cmds = self.state.sync(looper, now);
        self.send_all(cmds);
        self.songs_tick(now);

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
                if self.learn.is_some() {
                    if pressed && key == egui::Key::Escape {
                        self.set_learning(false, None);
                    }
                    continue;
                }
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
                    if pressed {
                        self.papa_key(key);
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

        let drum_step = self.audio.as_ref().and_then(|a| a.drum_step());
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
        let songs_view = self.songs.as_ref().map(|s| ui::SongsView {
            current: &s.current.nombre,
            current_color: s.current.color,
            list: &s.list,
            busy: s.busy.as_deref(),
            saved_ago: s.last_saved.map(|t| now - t),
            last_export: s.last_export.as_deref(),
        });
        let t = &self.taller;
        let taller_view = Some(ui::TallerView {
            list: &t.list,
            overview: &t.overview,
            root: t.root,
            recording: t.is_recording(),
            rec_level: t.rec_level,
            rec_seconds: t.rec_seconds,
            seconds: t.current.as_ref().map_or(0.0, |c| c.1.seconds()),
        });
        ui::draw(
            &canvas,
            &self.state,
            &ui::Extra {
                time: now,
                peak,
                status: &status,
                drum_step,
                songs: songs_view,
                taller: taller_view,
            },
        );
        if self.papa {
            ui::draw_papa(&canvas, &self.papa_lines());
        }
        if let Some(l) = &self.learn {
            ui::draw_learn(
                &canvas,
                l.step(),
                l.total(),
                &l.current().map(aprender::prompt).unwrap_or_default(),
                l.message.as_deref(),
            );
        }

        self.autosave(now, false);

        // Animación del osciloscopio mientras suena algo.
        if peak > 1e-4
            || self.taller.is_recording()
            || drum_step.is_some()
            || self.state.looper_visible()
            || self.state.looper.playing
            || self.state.current.is_some()
            || !self.state.melody.is_empty()
            || self.state.in_game()
        {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}
