//! Mapachsynthesizer: sintetizador para niños.

mod ajustes;
mod aprender;
mod canciones;
mod captura;
mod keyboard;
mod sistema;
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
    /// Menú F1 abierto.
    menu: bool,
    /// Escribiendo un nombre de canción.
    naming: Option<Naming>,
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

/// Para qué se pide un nombre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamePurpose {
    /// Poner nombre o renombrar.
    Renombrar,
    /// Guardar una copia con otro nombre.
    Copiar,
    /// Guardar por primera vez.
    Guardar,
}

/// Nombre que se está escribiendo.
struct Naming {
    purpose: NamePurpose,
    text: String,
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
            menu: false,
            naming: None,
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
            app.menu = cap.demo == 14;
            if cap.demo == 20 {
                app.naming = Some(Naming {
                    purpose: NamePurpose::Guardar,
                    text: "Dragón Naranja".into(),
                });
            }
        }
        // Los samples de los atajos.
        let atajos = app.state.shortcuts.clone();
        for (i, path) in atajos.iter().enumerate() {
            if let Some(p) = path {
                match taller::load_instrument(std::path::Path::new(p)) {
                    Some((data, _)) => app.send_all(vec![Command::SetShortcut {
                        slot: i as u8,
                        data: Some(data),
                    }]),
                    None => app.state.shortcuts[i] = None,
                }
            }
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
                        | Action::AssignShortcut(_)
                        | Action::OpenPicker(_)
                        | Action::PreviewPick(_)
                        | Action::ToggleFavorite(_)
                        | Action::AssignPick { .. }
                        | Action::ClosePicker
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
            Action::AssignShortcut(i) => {
                let Some((data, _, name, path)) = t.for_instrument() else {
                    self.state.show_toast("Primero elige o graba un sonido");
                    return;
                };
                let path = path.to_string_lossy().into_owned();
                let slot = &mut self.state.shortcuts[i as usize];
                if slot.as_deref() == Some(&path) {
                    *slot = None;
                    link.send(Command::SetShortcut {
                        slot: i,
                        data: None,
                    });
                    self.state.show_toast(format!("Atajo {} vacío", i + 1));
                } else {
                    *slot = Some(path);
                    link.send(Command::SetShortcut {
                        slot: i,
                        data: Some(data),
                    });
                    self.state
                        .show_toast(format!("⚡ El atajo {} suena a «{name}»", i + 1));
                }
            }
            Action::OpenPicker(_) => {
                t.read_picks(&self.state.favorites);
                self.state.picker_count = t.picks.len();
                if t.picks.is_empty() {
                    self.state.picker = None;
                    self.state
                        .show_toast("No hay samples: deja MP3, OGG o WAV en Música › Mapachsynthesizer › Samples");
                } else if let Some((data, _, _)) = t.load_pick(0) {
                    link.send(Command::Preview(Some(data)));
                }
            }
            Action::PreviewPick(i) => {
                if let Some((data, _, _)) = t.load_pick(i) {
                    link.send(Command::Preview(Some(data)));
                }
            }
            Action::ToggleFavorite(i) => {
                if let Some(file) = t.pick_file(i) {
                    let favs = &mut self.state.favorites;
                    let fav = if let Some(pos) = favs.iter().position(|f| *f == file) {
                        favs.remove(pos);
                        false
                    } else {
                        favs.push(file.clone());
                        true
                    };
                    // Reordenar sin perder el sample elegido.
                    t.read_picks(&self.state.favorites);
                    self.state.picker_count = t.picks.len();
                    let new = (0..t.picks.len())
                        .find(|&j| t.pick_file(j).as_deref() == Some(&file))
                        .unwrap_or(0);
                    if let Some(p) = &mut self.state.picker {
                        p.sel = new;
                    }
                    self.state.show_toast(if fav {
                        "★ Favorito"
                    } else {
                        "☆ Ya no es favorito"
                    });
                }
            }
            Action::AssignPick { target, pick } => match (target, t.load_pick(pick)) {
                (state::PickTarget::Atajo(i), Some((data, name, path))) => {
                    link.send(Command::SetShortcut {
                        slot: i,
                        data: Some(data),
                    });
                    self.state.shortcuts[i as usize] = Some(path.to_string_lossy().into_owned());
                    self.state
                        .show_toast(format!("⚡ El atajo {} suena a «{name}»", i + 1));
                }
                (state::PickTarget::Instrumento, Some((data, name, path))) => {
                    // Se afina según su nota (si no tiene, como un Do).
                    let root = mapach_samples::pitch::root_note(&data);
                    link.send(Command::SetSample {
                        slot: 0,
                        data,
                        root,
                    });
                    for c in self
                        .state
                        .use_instrument(path.to_string_lossy().into_owned())
                    {
                        link.send(c);
                    }
                    self.state
                        .show_toast(format!("🎤 Tus teclas suenan a «{name}»"));
                }
                (_, None) => self.state.show_toast("Ese fichero no se puede leer 😕"),
            },
            Action::ClosePicker => t.stop_preview(link),
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
            self.menu = false;
        }
    }

    /// Teclas F2–F10 con el menú F1 abierto.
    fn menu_key(&mut self, key: egui::Key) {
        use egui::Key as K;
        let name = self
            .songs
            .as_ref()
            .map(|s| s.current.nombre.clone())
            .unwrap_or_default();
        match key {
            K::F2 => {
                // Canción nueva: se pierde lo que no esté guardado.
                if let (Some(songs), Some(audio)) = (&mut self.songs, &mut self.audio) {
                    songs.new_song(&mut self.state, audio);
                    self.state.show_toast("✨ Canción nueva");
                }
                self.menu = false;
            }
            K::F3 => {
                self.naming = Some(Naming {
                    purpose: NamePurpose::Renombrar,
                    text: name,
                })
            }
            K::F4 => {
                self.naming = Some(Naming {
                    purpose: NamePurpose::Copiar,
                    text: format!("{name} copia"),
                })
            }
            K::F5 => {
                if self.songs.as_ref().is_some_and(|s| s.is_saved()) {
                    self.save_song();
                    self.menu = false;
                } else {
                    // Aún no tiene nombre: se pide.
                    self.naming = Some(Naming {
                        purpose: NamePurpose::Guardar,
                        text: name,
                    });
                }
            }
            K::F6 | K::F7 => {
                let d = if key == K::F6 { -1.0 } else { 1.0 };
                let (lo, hi) = sistema::MAX_DB_RANGE;
                self.state.max_db = (self.state.max_db + d).clamp(lo, hi);
                let c = Command::SetParam(mapach_engine::Param::MaxLevelDb(self.state.max_db));
                self.send_all(vec![c]);
            }
            K::F8 => self.state.note_colors = !self.state.note_colors,
            K::F9 => {
                let on = !sistema::autostart_enabled();
                if let Err(e) = sistema::set_autostart(on) {
                    eprintln!("Arranque automático: {e}");
                }
            }
            K::F10 => {
                if self.mando_status == mando::Status::Conectado {
                    self.set_learning(true, None);
                } else {
                    self.state.show_toast("Conecta el mando primero");
                }
            }
            _ => {}
        }
    }

    /// Guarda la canción ya (en segundo plano).
    fn save_song(&mut self) {
        if let (Some(songs), Some(audio)) = (&mut self.songs, &mut self.audio) {
            songs.begin_save(&self.state, audio, None);
            self.state
                .show_toast(format!("💾 Guardando «{}»", songs.current.nombre));
        } else {
            self.state
                .show_toast("Sin audio no se pueden guardar canciones");
        }
    }

    /// Se ha escrito un nombre y se ha pulsado Intro.
    fn naming_done(&mut self, n: Naming) {
        let Some(songs) = &mut self.songs else { return };
        let res = match n.purpose {
            NamePurpose::Renombrar => songs.rename(&n.text),
            NamePurpose::Copiar => songs.start_copy(&n.text),
            NamePurpose::Guardar => songs.rename(&n.text),
        };
        match res {
            Ok(()) => {
                if n.purpose != NamePurpose::Renombrar || songs.is_saved() {
                    self.save_song();
                } else {
                    self.state
                        .show_toast(format!("✏ Se llama «{}»", n.text.trim()));
                }
                self.menu = false;
            }
            Err(e) => {
                self.state.show_toast(e);
                // Se puede volver a intentar.
                self.naming = Some(n);
            }
        }
    }

    fn menu_lines(&self) -> (Vec<String>, Vec<(&'static str, String)>) {
        let mut info = Vec::new();
        match &self.audio {
            Some(a) => info.push(format!(
                "Audio: {} · {} Hz · latencia {:.1} ms",
                a.device_name,
                a.sample_rate,
                a.latency_ms()
            )),
            None => info.push("Sin audio".into()),
        }
        info.push(format!(
            "Mando: {} · versión {} · ⭐ {}",
            status_text(self.mando_status),
            env!("CARGO_PKG_VERSION"),
            self.state.stars
        ));
        let song = self
            .songs
            .as_ref()
            .map(|s| {
                if s.is_saved() {
                    format!("«{}» (guardada)", s.current.nombre)
                } else {
                    "sin guardar".to_string()
                }
            })
            .unwrap_or_default();
        info.push(format!("Canción: {song}"));
        let si_no = |b: bool| if b { "sí" } else { "no" };
        let opts = vec![
            ("F2", "Canción nueva (se pierde lo no guardado)".to_string()),
            ("F3", "Poner nombre o renombrar".to_string()),
            ("F4", "Copiar la canción".to_string()),
            ("F5", "Guardar".to_string()),
            (
                "F6/F7",
                format!("Volumen máximo: {:.0} dBFS", self.state.max_db),
            ),
            (
                "F8",
                format!("Colores por nota: {}", si_no(self.state.note_colors)),
            ),
            (
                "F9",
                format!(
                    "Arrancar al encender: {}",
                    si_no(sistema::autostart_enabled())
                ),
            ),
            ("F10", "Aprender mando".to_string()),
        ];
        (info, opts)
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
            if !self.menu && self.naming.is_none() {
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
            if let (egui::Event::Text(t), Some(n)) = (&ev, &mut self.naming) {
                for ch in t.chars().filter(|c| !c.is_control()) {
                    if n.text.chars().count() < 40 {
                        n.text.push(ch);
                    }
                }
                continue;
            }
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
                // Escribiendo un nombre: Intro acepta, Esc cancela, Retroceso borra.
                if let Some(n) = &mut self.naming {
                    if pressed {
                        match key {
                            egui::Key::Enter => {
                                if let Some(n) = self.naming.take() {
                                    self.naming_done(n);
                                }
                            }
                            egui::Key::Escape => self.naming = None,
                            egui::Key::Backspace => {
                                n.text.pop();
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                if pressed && key == egui::Key::Escape {
                    if self.menu {
                        // Esc cierra el menú.
                        self.menu = false;
                    } else {
                        // Esc cierra el programa (guardando antes los ajustes).
                        self.release_all();
                        self.autosave(now, true);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    continue;
                }
                if pressed && key == egui::Key::F1 {
                    self.release_all();
                    self.menu = !self.menu;
                    continue;
                }
                if self.menu {
                    if pressed {
                        self.menu_key(key);
                    }
                    continue;
                }
                match keyboard::translate(key, modifiers, pressed) {
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
            if !self.menu && self.naming.is_none() {
                self.event(Event::Turn(Knob::Cursor, d as i32), now);
            }
        }

        let screen = ui.max_rect();
        let painter = ui.painter_at(screen);
        let canvas = ui::Canvas::new(&painter, screen);

        // Ratón o dedo sobre las teclas de acorde.
        let (down, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));
        let target = if down && !self.menu && self.naming.is_none() {
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
            saved: s.is_saved(),
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
            picks: &t.picks,
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
        if self.menu {
            let (info, opts) = self.menu_lines();
            ui::draw_menu(&canvas, &info, &opts);
        }
        if let Some(n) = &self.naming {
            let title = match n.purpose {
                NamePurpose::Renombrar => "Nombre de la canción",
                NamePurpose::Copiar => "Nombre de la copia",
                NamePurpose::Guardar => "Guardar como…",
            };
            ui::draw_naming(&canvas, title, &n.text, now);
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
