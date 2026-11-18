//! Estado de la app y su lógica: convierte eventos de entrada en órdenes para el motor.
//!
//! No depende de `egui` ni de `cpal`, así que se prueba con tests normales.

use mapach_dsp::drums::KITS;
use mapach_engine::drums::{self, Pattern, STEPS, STYLES, VARIATIONS};
use mapach_engine::looper::{LooperStatus, TRACKS};
use mapach_engine::{ARP_PATTERNS, Command, Param, RATES, SOUNDS, STRUM_MS, Style};
use mapach_input::mando;
use mapach_input::{Arrow, Control, Event, Knob, Menu};
use mapach_project::Format;
use mapach_theory::{
    BUTTONS, Chord, Direction, Key, ModifierMode, Scale, VoicingOptions, voice_lead,
};

/// Modos de toque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayMode {
    #[default]
    Tocar,
    Rasgueo,
    Arpegio,
    Repetir,
    Zumbido,
    Melodia,
    /// Las 7 teclas tocan la batería.
    Bateria,
    /// Un ritmo suena solo y encima se tocan acordes.
    Ritmos,
    /// Secuenciador de 16 pasos.
    Secuencia,
}

impl PlayMode {
    pub const ALL: [PlayMode; 9] = [
        PlayMode::Tocar,
        PlayMode::Rasgueo,
        PlayMode::Arpegio,
        PlayMode::Repetir,
        PlayMode::Zumbido,
        PlayMode::Melodia,
        PlayMode::Bateria,
        PlayMode::Ritmos,
        PlayMode::Secuencia,
    ];

    /// ¿Es un modo de batería?
    pub fn is_drums(self) -> bool {
        matches!(
            self,
            PlayMode::Bateria | PlayMode::Ritmos | PlayMode::Secuencia
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            PlayMode::Tocar => "Tocar",
            PlayMode::Rasgueo => "Rasgueo",
            PlayMode::Arpegio => "Arpegio",
            PlayMode::Repetir => "Repetir",
            PlayMode::Zumbido => "Zumbido",
            PlayMode::Melodia => "Melodía",
            PlayMode::Bateria => "Batería",
            PlayMode::Ritmos => "Ritmos",
            PlayMode::Secuencia => "Secuencia",
        }
    }

    /// Texto e icono para la barra de arriba.
    pub fn badge(self) -> &'static str {
        match self {
            PlayMode::Tocar => "▶ TOCAR",
            PlayMode::Rasgueo => "🎸 RASGUEO",
            PlayMode::Arpegio => "〰 ARPEGIO",
            PlayMode::Repetir => "🔁 REPETIR",
            PlayMode::Zumbido => "🐝 ZUMBIDO",
            PlayMode::Melodia => "♪ MELODÍA",
            PlayMode::Bateria => "🥁 BATERÍA",
            PlayMode::Ritmos => "🎶 RITMOS",
            PlayMode::Secuencia => "▦ SECUENCIA",
        }
    }
}

/// Pantallas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Tocar,
    Canciones,
}

/// Cosas que la app tiene que hacer fuera de la lógica (leer y escribir ficheros).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Abrir la pantalla de canciones (hay que leer la lista).
    OpenSongs,
    /// Cargar la canción número `n` de la lista.
    Load(usize),
    /// Canción nueva (vacía).
    New,
    Export(Format),
}

/// Fila elegida en la pantalla de canciones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongFocus {
    /// Tarjeta `n` (la última es "Nueva").
    Card(usize),
    /// Botón de exportar `n` (MP3, OGG, WAV).
    Export(usize),
}

/// Lo que suena ahora en modo Tocar.
#[derive(Debug, Clone, PartialEq)]
pub struct Sounding {
    pub button: u8,
    pub high: bool,
    pub chord: Chord,
    pub dir: Option<Direction>,
    pub notes: Vec<i32>,
}

/// Opciones de cada página de menú.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Tonalidad,
    Escala,
    Octava,
    Acordes,
    Bajo,
    Nombres,
    Sonido,
    Ataque,
    Release,
    Brillo,
    Modo,
    Tempo,
    Patron,
    Figura,
    Velocidad,
    Kit,
    Estilo,
    Variacion,
    /// Abre la pantalla de canciones (para llegar desde el mando).
    Canciones,
}

impl Item {
    /// Todas las opciones posibles de un menú (algunas solo salen en ciertos modos).
    pub fn of(menu: Menu) -> &'static [Item] {
        match menu {
            Menu::Tono => &[
                Item::Tonalidad,
                Item::Escala,
                Item::Octava,
                Item::Acordes,
                Item::Bajo,
                Item::Nombres,
                Item::Canciones,
            ],
            Menu::Sonido => &[Item::Sonido, Item::Ataque, Item::Release, Item::Brillo],
            Menu::Modo => &[
                Item::Modo,
                Item::Tempo,
                Item::Patron,
                Item::Figura,
                Item::Velocidad,
                Item::Kit,
                Item::Estilo,
                Item::Variacion,
            ],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Item::Tonalidad => "Tonalidad",
            Item::Escala => "Escala",
            Item::Octava => "Octava",
            Item::Acordes => "Flechas",
            Item::Bajo => "Bajo",
            Item::Nombres => "Nombres",
            Item::Sonido => "Sonido",
            Item::Ataque => "Ataque",
            Item::Release => "Release",
            Item::Brillo => "Brillo",
            Item::Modo => "Modo",
            Item::Tempo => "Tempo",
            Item::Patron => "Patrón",
            Item::Figura => "Figura",
            Item::Velocidad => "Velocidad",
            Item::Kit => "Batería",
            Item::Estilo => "Ritmo",
            Item::Variacion => "Variación",
            Item::Canciones => "Mis canciones",
        }
    }
}

/// Una página de menú abierta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenMenu {
    pub menu: Menu,
    pub item: usize,
}

pub const BPM_MIN: f32 = 40.0;
pub const BPM_MAX: f32 = 300.0;

#[derive(Debug, Clone)]
pub struct State {
    pub key: Key,
    pub mod_mode: ModifierMode,
    pub sound: usize,
    pub volume: f32,
    pub muted: bool,
    pub attack: f32,
    pub release: f32,
    pub brightness: f32,
    pub bpm: f32,
    pub bass: bool,
    /// Enseñar nombres de acordes y notas (por defecto no: solo colores).
    pub show_names: bool,
    pub play_mode: PlayMode,
    pub arp_pattern: usize,
    pub arp_rate: usize,
    pub strum_speed: usize,
    pub repeat_rate: usize,
    pub kit: usize,
    /// Estilo de ritmo; `STYLES.len()` es "Mi ritmo" (el del secuenciador).
    pub drum_style: usize,
    pub drum_variation: usize,
    pub drums_playing: bool,
    pub seq_pattern: Pattern,
    /// Instrumento que se edita en el secuenciador.
    pub seq_row: usize,
    /// Último golpe de batería (para la pantalla).
    pub last_hit: Option<u8>,
    pub screen: Screen,
    pub song_focus: SongFocus,
    /// Canciones en la lista (lo dice la app al leer la carpeta).
    pub song_count: usize,
    actions: Vec<Action>,
    /// Mensaje corto en pantalla y hasta cuándo.
    pub toast: Option<(String, f64)>,
    /// Pista elegida del looper.
    pub track: usize,
    /// Estado del looper que llega del motor.
    pub looper: LooperStatus,
    pub track_volume: [f32; TRACKS],
    /// Hasta cuándo se ve la pantalla del looper.
    pub looper_shown_until: f64,
    /// Pista que se mantiene pulsada y desde cuándo (mantener 1,5 s = borrar).
    pub track_hold: Option<(usize, f64)>,
    /// Grabar está pulsado; si mientras tanto se elige pista, al soltar no se graba.
    record_held: Option<bool>,
    /// Última pista grabada, para deshacer.
    last_recorded: Option<usize>,
    now: f64,
    pub open_menu: Option<OpenMenu>,
    pub current: Option<Sounding>,
    /// Notas que suenan en modo Melodía (tecla → nota).
    pub melody: Vec<(Control, i32)>,
    /// Último "color" elegido con la perilla mágica.
    pub knob_color: usize,
    /// Teclas de acorde pulsadas, en orden.
    held: Vec<(u8, bool)>,
    arrows: [bool; 4],
    menus_held: [bool; 3],
    /// Se ha usado una combinación mientras se mantenía el menú (entonces no se abre al soltar).
    menu_used: bool,
    last_upper: Vec<i32>,
    taps: Vec<f64>,
    /// Pequeño generador pseudoaleatorio para el 🎲 (determinista).
    rng: u32,
}

impl Default for State {
    fn default() -> Self {
        let p = SOUNDS[0].patch;
        Self {
            key: Key::new(0, Scale::Major),
            mod_mode: ModifierMode::Basic,
            sound: 0,
            volume: 0.7,
            muted: false,
            attack: p.env.attack,
            release: p.env.release,
            brightness: p.brightness,
            bpm: 120.0,
            bass: true,
            show_names: false,
            play_mode: PlayMode::Tocar,
            arp_pattern: 0,
            arp_rate: 3,
            strum_speed: 1,
            repeat_rate: 1,
            kit: 0,
            drum_style: 0,
            drum_variation: 0,
            drums_playing: false,
            seq_pattern: drums::EMPTY,
            seq_row: 0,
            last_hit: None,
            screen: Screen::Tocar,
            song_focus: SongFocus::Card(0),
            song_count: 0,
            actions: Vec::new(),
            toast: None,
            track: 0,
            looper: LooperStatus::default(),
            track_volume: [1.0; TRACKS],
            looper_shown_until: 0.0,
            track_hold: None,
            record_held: None,
            last_recorded: None,
            now: 0.0,
            open_menu: None,
            current: None,
            melody: Vec::new(),
            knob_color: 0,
            held: Vec::new(),
            arrows: [false; 4],
            menus_held: [false; 3],
            menu_used: false,
            last_upper: Vec::new(),
            taps: Vec::new(),
            rng: 0x1234_5678,
        }
    }
}

fn arrow_idx(a: Arrow) -> usize {
    match a {
        Arrow::Up => 0,
        Arrow::Down => 1,
        Arrow::Left => 2,
        Arrow::Right => 3,
    }
}

fn menu_idx(m: Menu) -> usize {
    match m {
        Menu::Tono => 0,
        Menu::Sonido => 1,
        Menu::Modo => 2,
    }
}

fn wrap(i: usize, delta: i32, len: usize) -> usize {
    (i as i32 + delta).rem_euclid(len as i32) as usize
}

impl State {
    /// Órdenes para dejar el motor igual que este estado (al arrancar).
    pub fn initial_commands(&self) -> Vec<Command> {
        vec![
            Command::SetPatch(SOUNDS[self.sound].patch),
            Command::SetParam(Param::Attack(self.attack)),
            Command::SetParam(Param::Release(self.release)),
            Command::SetParam(Param::Brightness(self.brightness)),
            Command::SetParam(Param::Volume(self.effective_volume())),
            Command::SetStyle(self.style()),
            Command::SetBpm(self.bpm),
            Command::SetKit(self.kit as u8),
            Command::SetPattern(self.drum_pattern()),
        ]
    }

    /// Patrón que debe sonar en la caja de ritmos.
    pub fn drum_pattern(&self) -> Pattern {
        if self.play_mode == PlayMode::Secuencia || self.drum_style >= STYLES.len() {
            self.seq_pattern
        } else {
            drums::rhythm(self.drum_style, self.drum_variation)
        }
    }

    /// Nombre del ritmo elegido.
    pub fn rhythm_name(&self) -> String {
        if self.drum_style >= STYLES.len() {
            "Mi ritmo".into()
        } else {
            STYLES[self.drum_style].to_string()
        }
    }

    fn set_drums_playing(&mut self, on: bool, out: &mut Vec<Command>) {
        if on {
            out.push(Command::SetPattern(self.drum_pattern()));
        }
        self.drums_playing = on;
        out.push(Command::DrumsPlay(on));
    }

    /// Saca las acciones pendientes para la app.
    pub fn take_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.actions)
    }

    /// Enseña un mensaje unos segundos.
    pub fn show_toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), self.now + 3.0));
    }

    pub fn toast_text(&self) -> Option<&str> {
        self.toast
            .as_ref()
            .filter(|(_, until)| self.now < *until)
            .map(|(t, _)| t.as_str())
    }

    fn open_songs(&mut self) {
        self.screen = Screen::Canciones;
        self.open_menu = None;
        self.song_focus = SongFocus::Card(0);
        self.actions.push(Action::OpenSongs);
    }

    /// Controles en la pantalla de canciones. Devuelve `true` si el control se ha usado.
    fn songs_press(&mut self, c: Control) -> bool {
        let cards = self.song_count + 1;
        match (c, self.song_focus) {
            (Control::Arrow(Arrow::Left), SongFocus::Card(i)) => {
                self.song_focus = SongFocus::Card(wrap(i, -1, cards));
            }
            (Control::Arrow(Arrow::Right), SongFocus::Card(i)) => {
                self.song_focus = SongFocus::Card(wrap(i, 1, cards));
            }
            (Control::Arrow(Arrow::Down), SongFocus::Card(_)) => {
                self.song_focus = SongFocus::Export(0);
            }
            (Control::Arrow(Arrow::Left), SongFocus::Export(i)) => {
                self.song_focus = SongFocus::Export(wrap(i, -1, Format::ALL.len()));
            }
            (Control::Arrow(Arrow::Right), SongFocus::Export(i)) => {
                self.song_focus = SongFocus::Export(wrap(i, 1, Format::ALL.len()));
            }
            (Control::Arrow(Arrow::Up), SongFocus::Export(_)) => {
                self.song_focus = SongFocus::Card(0);
            }
            (Control::Arrow(_), _) => {}
            (Control::Record | Control::KnobPress(Knob::Tempo), SongFocus::Card(i)) => {
                self.actions.push(if i >= self.song_count {
                    Action::New
                } else {
                    Action::Load(i)
                });
                self.screen = Screen::Tocar;
            }
            (Control::Record | Control::KnobPress(Knob::Tempo), SongFocus::Export(i)) => {
                self.actions
                    .push(Action::Export(Format::ALL[i % Format::ALL.len()]));
            }
            (Control::Canciones | Control::KnobPress(Knob::Magica) | Control::Menu(_), _) => {
                self.screen = Screen::Tocar
            }
            _ => return false,
        }
        true
    }

    /// Segundos que hay que mantener una pista para borrarla.
    pub const HOLD_TO_CLEAR: f64 = 1.5;

    /// Llamar en cada fotograma con el estado del looper que da el motor.
    pub fn sync(&mut self, looper: LooperStatus, now: f64) -> Vec<Command> {
        let mut out = Vec::new();
        self.now = now;
        let was_recording = self.looper.recording;
        self.looper = looper;
        if let Some(t) = was_recording
            && looper.recording.is_none()
        {
            // Terminó una grabación: siguiente pista vacía.
            self.last_recorded = looper.has(t).then_some(t);
            if let Some(next) = (1..=TRACKS)
                .map(|i| (t + i) % TRACKS)
                .find(|&i| !looper.has(i))
            {
                self.track = next;
            }
            self.looper_shown_until = now + 2.0;
        }
        if let Some((t, since)) = self.track_hold
            && now - since >= Self::HOLD_TO_CLEAR
        {
            self.track_hold = None;
            self.clear_track(t, &mut out);
        }
        out
    }

    fn clear_track(&mut self, t: usize, out: &mut Vec<Command>) {
        out.push(Command::LoopClear(t as u8));
        self.looper.used &= !(1 << t);
        if self.last_recorded == Some(t) {
            self.last_recorded = None;
        }
        self.track = t;
        self.looper_shown_until = self.now + 2.0;
    }

    /// ¿Se ve la pantalla del looper?
    pub fn looper_visible(&self) -> bool {
        self.looper.recording.is_some()
            || self.now < self.looper_shown_until
            || self.track_hold.is_some()
            || self.record_held.is_some()
    }

    fn record(&mut self, out: &mut Vec<Command>) {
        out.push(Command::LoopRecord(self.track as u8));
        // Lo marcamos ya, sin esperar al motor, para que la pantalla responda enseguida.
        if self.looper.recording.is_none() {
            self.looper.recording = Some(self.track);
            self.looper.used |= 1 << self.track;
        }
        self.looper_shown_until = self.now + 2.0;
    }

    /// Traduce las teclas crudas del mando y, en el secuenciador, las teclas de pasos.
    fn remap(&self, c: Control) -> Option<Control> {
        let seq = self.play_mode == PlayMode::Secuencia;
        match c {
            Control::Pad(n) if seq => Some(Control::Step(n - 1)),
            Control::Pad(n) => mando::key_control(n),
            Control::Track(t) if seq => Some(Control::Step(t)),
            Control::ChordHigh(b) if seq => Some(Control::Step(8 + b)),
            other => Some(other),
        }
    }

    /// Modo de toque para el motor.
    pub fn style(&self) -> Style {
        match self.play_mode {
            PlayMode::Tocar
            | PlayMode::Melodia
            | PlayMode::Bateria
            | PlayMode::Ritmos
            | PlayMode::Secuencia => Style::Normal,
            PlayMode::Rasgueo => Style::Strum {
                speed: self.strum_speed as u8,
            },
            PlayMode::Arpegio => Style::Arp {
                pattern: self.arp_pattern as u8,
                rate: self.arp_rate as u8,
            },
            PlayMode::Repetir => Style::Repeat {
                rate: self.repeat_rate as u8,
            },
            PlayMode::Zumbido => Style::Drone,
        }
    }

    /// Opciones que se ven en la página de un menú (dependen del modo).
    pub fn items(&self, menu: Menu) -> Vec<Item> {
        Item::of(menu)
            .iter()
            .copied()
            .filter(|i| match i {
                Item::Patron => self.play_mode == PlayMode::Arpegio,
                Item::Figura => matches!(self.play_mode, PlayMode::Arpegio | PlayMode::Repetir),
                Item::Velocidad => self.play_mode == PlayMode::Rasgueo,
                Item::Kit => self.play_mode.is_drums(),
                Item::Estilo | Item::Variacion => self.play_mode == PlayMode::Ritmos,
                _ => true,
            })
            .collect()
    }

    /// Opción elegida en la página abierta.
    pub fn selected_item(&self) -> Option<Item> {
        let o = self.open_menu?;
        let items = self.items(o.menu);
        items
            .get(o.item.min(items.len().saturating_sub(1)))
            .copied()
    }

    fn effective_volume(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }

    /// Suelta todo lo pulsado (al perder el foco o abrir el modo papá).
    pub fn release_all(&mut self) {
        self.track_hold = None;
        self.record_held = None;
        self.held.clear();
        self.arrows = [false; 4];
        self.menus_held = [false; 3];
        self.menu_used = false;
        self.current = None;
        self.melody.clear();
        self.knob_color = 0;
    }

    /// El menú que se mantiene pulsado (si hay varios, el primero).
    pub fn menu_held(&self) -> Option<Menu> {
        Menu::ALL
            .into_iter()
            .find(|&m| self.menus_held[menu_idx(m)])
    }

    pub fn arrow_held(&self, a: Arrow) -> bool {
        self.arrows[arrow_idx(a)]
    }

    pub fn chord_held(&self, button: u8) -> bool {
        self.held.iter().any(|&(b, _)| b == button)
            || self.melody.iter().any(
                |(c, _)| matches!(c, Control::Chord(b) | Control::ChordHigh(b) if *b == button),
            )
    }

    /// Dirección de las flechas pulsadas.
    pub fn arrow_dir(&self) -> Option<Direction> {
        let [u, d, l, r] = self.arrows;
        Direction::from_arrows(u, d, l, r)
    }

    /// Dirección efectiva: las flechas mandan; si no hay, la de la perilla mágica.
    pub fn dir(&self) -> Option<Direction> {
        self.arrow_dir().or(Direction::KNOB_CYCLE[self.knob_color])
    }

    /// Acorde que tocaría la tecla `button` con la dirección actual.
    pub fn chord_for(&self, button: u8, high: bool) -> Chord {
        let mut c = self
            .mod_mode
            .apply(self.key.chord(button as usize), self.dir());
        if high {
            c.root += 12;
        }
        c
    }

    /// Procesa un evento. `now` en segundos (para el tap tempo).
    pub fn handle(&mut self, ev: Event, now: f64) -> Vec<Command> {
        let mut out = Vec::new();
        match ev {
            Event::Press(c) => {
                if let Some(c) = self.remap(c) {
                    self.press(c, now, &mut out);
                }
            }
            Event::Release(c) => {
                if let Some(c) = self.remap(c) {
                    self.release_control(c, &mut out);
                }
            }
            Event::Turn(k, d) => self.turn(k, d, &mut out),
        }
        out
    }

    fn press(&mut self, c: Control, now: f64, out: &mut Vec<Command>) {
        self.now = self.now.max(now);
        if self.screen == Screen::Canciones && self.songs_press(c) {
            return;
        }
        match c {
            Control::Canciones => self.open_songs(),
            // Grabar + tecla de acorde 1–6 (en el mando): elegir pista.
            Control::Chord(b) if self.record_held.is_some() && (b as usize) < TRACKS => {
                self.record_held = Some(true);
                self.track = b as usize;
                self.looper_shown_until = self.now + 2.0;
            }
            Control::Chord(b) | Control::ChordHigh(b) if (b as usize) < BUTTONS => {
                let high = matches!(c, Control::ChordHigh(_));
                match self.play_mode {
                    PlayMode::Bateria | PlayMode::Secuencia => {
                        self.held.retain(|&(x, h)| (x, h) != (b, high));
                        self.held.push((b, high));
                        self.last_hit = Some(b);
                        if self.play_mode == PlayMode::Secuencia {
                            self.seq_row = b as usize;
                        }
                        out.push(Command::DrumHit {
                            drum: b,
                            velocity: if high { 0.6 } else { 1.0 },
                        });
                    }
                    PlayMode::Melodia => {
                        let note = self.key.scale_note(b as i32) + if high { 12 } else { 0 };
                        self.melody.retain(|(x, _)| *x != c);
                        self.melody.push((c, note));
                        out.push(Command::NoteOn {
                            note: note.clamp(0, 127) as u8,
                            velocity: 0.9,
                        });
                    }
                    _ => {
                        self.held.retain(|&(x, h)| (x, h) != (b, high));
                        self.held.push((b, high));
                        self.play_chord(b, high, true, out);
                    }
                }
            }
            Control::Arrow(a) => {
                self.arrows[arrow_idx(a)] = true;
                if let Some(m) = self.menu_held() {
                    self.menu_used = true;
                    self.menu_shortcut(m, a, out);
                } else if let Some(open) = self.open_menu {
                    self.navigate(open, a, out);
                } else if self.play_mode == PlayMode::Secuencia {
                    // En el secuenciador, ↑↓ eligen el instrumento.
                    let d = match a {
                        Arrow::Up => -1,
                        Arrow::Down => 1,
                        _ => 0,
                    };
                    self.seq_row = wrap(self.seq_row, d, 7);
                } else {
                    self.revoice(out);
                }
            }
            Control::Step(n) if (n as usize) < STEPS => {
                let bit = 1u8 << self.seq_row;
                self.seq_pattern[n as usize] ^= bit;
                if self.seq_pattern[n as usize] & bit != 0 {
                    out.push(Command::DrumHit {
                        drum: self.seq_row as u8,
                        velocity: 0.8,
                    });
                }
                out.push(Command::SetPattern(self.drum_pattern()));
            }
            Control::Menu(m) => {
                self.menus_held[menu_idx(m)] = true;
                self.menu_used = false;
            }
            Control::Record => {
                // Espacio con un menú pulsado: 🎲 aleatorio. Con un menú abierto: cerrar.
                if let Some(m) = self.menu_held() {
                    self.menu_used = true;
                    self.randomize(m, out);
                } else if self.open_menu.is_some() {
                    self.open_menu = None;
                } else if self.looper.recording.is_some() {
                    // Parar enseguida.
                    self.record(out);
                } else {
                    // Empezar al soltar (por si mientras tanto se elige pista).
                    self.record_held = Some(false);
                }
            }
            Control::Track(t) if (t as usize) < TRACKS => {
                self.track = t as usize;
                self.track_hold = Some((self.track, self.now));
                self.looper_shown_until = self.now + 2.0;
            }
            Control::Undo => {
                if let Some(t) = self.last_recorded.take() {
                    self.clear_track(t, out);
                }
            }
            Control::KnobPress(Knob::Volumen) => {
                self.muted = !self.muted;
                out.push(Command::SetParam(Param::Volume(self.effective_volume())));
            }
            Control::KnobPress(Knob::Magica) => {
                if let Some(m) = self.menu_held() {
                    self.menu_used = true;
                    self.randomize(m, out);
                } else if self.open_menu.is_some() {
                    self.open_menu = None;
                } else if self.play_mode == PlayMode::Secuencia {
                    // Salir del secuenciador: el ritmo sigue como "Mi ritmo".
                    self.drum_style = STYLES.len();
                    self.set_play_mode(PlayMode::Ritmos, out);
                }
            }
            Control::KnobPress(Knob::Tempo) | Control::Tap => {
                self.tap(now);
                out.push(Command::SetBpm(self.bpm));
            }
            // De momento, Play para el zumbido (el looper llegará en la fase 7).
            Control::Play if self.play_mode == PlayMode::Zumbido && self.held.is_empty() => {
                self.current = None;
                out.push(Command::StopChord);
            }
            Control::Play if !self.looper.is_empty() => {
                let on = !self.looper.playing;
                out.push(Command::LoopPlay(on));
                self.looper.playing = on;
                if self.play_mode.is_drums() {
                    self.set_drums_playing(on, out);
                }
            }
            Control::Play if self.play_mode.is_drums() => {
                self.set_drums_playing(!self.drums_playing, out);
            }
            _ => {}
        }
    }

    fn release_control(&mut self, c: Control, out: &mut Vec<Command>) {
        match c {
            Control::Chord(b) | Control::ChordHigh(b) => {
                let high = matches!(c, Control::ChordHigh(_));
                if let Some(pos) = self.melody.iter().position(|(x, _)| *x == c) {
                    let (_, note) = self.melody.remove(pos);
                    if !self.melody.iter().any(|&(_, n)| n == note) {
                        out.push(Command::NoteOff {
                            note: note.clamp(0, 127) as u8,
                        });
                    }
                }
                let was_current = self
                    .current
                    .as_ref()
                    .is_some_and(|s| s.button == b && s.high == high);
                self.held.retain(|&(x, h)| (x, h) != (b, high));
                if was_current {
                    if let Some(&(nb, nh)) = self.held.last() {
                        self.play_chord(nb, nh, true, out);
                    } else {
                        self.stop_chord(out);
                    }
                }
            }
            Control::Arrow(a) => {
                self.arrows[arrow_idx(a)] = false;
                if self.menu_held().is_none() && self.open_menu.is_none() {
                    self.revoice(out);
                }
            }
            Control::Record => {
                if self.record_held.take() == Some(false) {
                    self.record(out);
                }
            }
            Control::Track(t) => {
                if self.track_hold.is_some_and(|(h, _)| h == t as usize) {
                    self.track_hold = None;
                }
            }
            Control::Menu(m) => {
                self.menus_held[menu_idx(m)] = false;
                if !self.menu_used {
                    // Pulsar y soltar sin combinar: abre (o cierra) la página del menú.
                    self.open_menu = match self.open_menu {
                        Some(o) if o.menu == m => None,
                        _ => Some(OpenMenu { menu: m, item: 0 }),
                    };
                }
                self.menu_used = false;
            }
            _ => {}
        }
    }

    fn turn(&mut self, k: Knob, d: i32, out: &mut Vec<Command>) {
        let held = self.menu_held();
        if held.is_some() {
            self.menu_used = true;
        }
        match (k, held) {
            (Knob::Volumen, Some(Menu::Tono)) => self.change(Item::Ataque, d, out),
            (Knob::Volumen, Some(Menu::Sonido)) => self.change(Item::Release, d, out),
            (Knob::Volumen, Some(Menu::Modo)) => self.change(Item::Brillo, d, out),
            (Knob::Volumen, None) if self.track_hold.is_some() => {
                // Pista pulsada + perilla 1: volumen de esa pista.
                let t = self.track;
                self.track_hold = Some((t, f64::INFINITY));
                self.track_volume[t] = (self.track_volume[t] + d as f32 * 0.05).clamp(0.0, 1.0);
                out.push(Command::LoopVolume(t as u8, self.track_volume[t]));
            }
            (Knob::Volumen, None) => {
                self.volume = (self.volume + d as f32 * 0.04).clamp(0.0, 1.0);
                self.muted = false;
                out.push(Command::SetParam(Param::Volume(self.volume)));
            }
            (Knob::Magica, Some(Menu::Tono)) => self.change(Item::Tonalidad, d, out),
            (Knob::Magica, Some(Menu::Sonido)) => self.change(Item::Sonido, d, out),
            (Knob::Magica, Some(Menu::Modo)) => self.change(Item::Modo, d, out),
            (Knob::Magica, None) => {
                if let Some(item) = self.selected_item() {
                    self.change(item, d, out);
                } else if self.play_mode == PlayMode::Secuencia {
                    self.seq_row = wrap(self.seq_row, d, 7);
                } else if self.current.is_some() {
                    self.knob_color = wrap(self.knob_color, d, Direction::KNOB_CYCLE.len());
                    self.revoice(out);
                }
            }
            (Knob::Tempo, Some(Menu::Tono)) => self.change(Item::Octava, d, out),
            (Knob::Tempo, Some(Menu::Modo)) => {
                let item = match self.play_mode {
                    PlayMode::Arpegio => Item::Patron,
                    PlayMode::Rasgueo => Item::Velocidad,
                    PlayMode::Ritmos => Item::Variacion,
                    PlayMode::Bateria | PlayMode::Secuencia => Item::Kit,
                    _ => Item::Figura,
                };
                self.change(item, d, out);
            }
            (Knob::Tempo, _) => self.change(Item::Tempo, d, out),
        }
    }

    /// Combinaciones de menú mantenido + flecha (como en el HiChord).
    fn menu_shortcut(&mut self, m: Menu, a: Arrow, out: &mut Vec<Command>) {
        let d = match a {
            Arrow::Right | Arrow::Up => 1,
            Arrow::Left | Arrow::Down => -1,
        };
        let item = match (m, a) {
            (Menu::Tono, Arrow::Left | Arrow::Right) => Item::Tonalidad,
            (Menu::Tono, _) => Item::Octava,
            (Menu::Sonido, Arrow::Left | Arrow::Right) => Item::Sonido,
            (Menu::Sonido, _) => Item::Brillo,
            (Menu::Modo, Arrow::Left | Arrow::Right) => Item::Modo,
            (Menu::Modo, _) => {
                self.change(Item::Tempo, d * 5, out);
                return;
            }
        };
        self.change(item, d, out);
    }

    fn navigate(&mut self, mut open: OpenMenu, a: Arrow, out: &mut Vec<Command>) {
        let len = self.items(open.menu).len();
        open.item = open.item.min(len - 1);
        self.open_menu = Some(open);
        match a {
            Arrow::Up => open.item = wrap(open.item, -1, len),
            Arrow::Down => open.item = wrap(open.item, 1, len),
            Arrow::Left | Arrow::Right => {
                let d = if a == Arrow::Left { -1 } else { 1 };
                if let Some(item) = self.selected_item() {
                    self.change(item, d, out);
                }
                return;
            }
        }
        self.open_menu = Some(open);
    }

    /// Cambia el valor de una opción `d` pasos.
    pub fn change(&mut self, item: Item, d: i32, out: &mut Vec<Command>) {
        match item {
            Item::Tonalidad => {
                self.key.transpose(d);
                self.revoice(out);
            }
            Item::Escala => {
                for _ in 0..d.unsigned_abs() {
                    self.key.scale = if d > 0 {
                        self.key.scale.next()
                    } else {
                        self.key.scale.prev()
                    };
                }
                self.revoice(out);
            }
            Item::Octava => {
                self.key.shift_octave(d);
                self.last_upper.clear();
                self.revoice(out);
            }
            Item::Acordes => {
                self.mod_mode = ModifierMode::ALL[wrap(
                    ModifierMode::ALL
                        .iter()
                        .position(|&m| m == self.mod_mode)
                        .unwrap_or(0),
                    d,
                    ModifierMode::ALL.len(),
                )];
                self.revoice(out);
            }
            Item::Bajo => {
                self.bass = !self.bass;
                self.revoice(out);
            }
            Item::Nombres => self.show_names = !self.show_names,
            Item::Sonido => {
                self.sound = wrap(self.sound, d, SOUNDS.len());
                self.apply_sound(out);
            }
            Item::Ataque => {
                self.attack = step_time(self.attack, d, 0.002, 4.0);
                out.push(Command::SetParam(Param::Attack(self.attack)));
            }
            Item::Release => {
                self.release = step_time(self.release, d, 0.02, 6.0);
                out.push(Command::SetParam(Param::Release(self.release)));
            }
            Item::Brillo => {
                self.brightness = (self.brightness + d as f32 * 0.03).clamp(0.0, 1.0);
                out.push(Command::SetParam(Param::Brightness(self.brightness)));
            }
            Item::Modo => {
                let i = PlayMode::ALL
                    .iter()
                    .position(|&m| m == self.play_mode)
                    .unwrap_or(0);
                self.set_play_mode(PlayMode::ALL[wrap(i, d, PlayMode::ALL.len())], out);
            }
            Item::Tempo => {
                self.bpm = (self.bpm + d as f32).clamp(BPM_MIN, BPM_MAX);
                out.push(Command::SetBpm(self.bpm));
            }
            Item::Patron => {
                self.arp_pattern = wrap(self.arp_pattern, d, ARP_PATTERNS.len());
                out.push(Command::SetStyle(self.style()));
            }
            Item::Figura => {
                if self.play_mode == PlayMode::Repetir {
                    self.repeat_rate = wrap(self.repeat_rate, d, RATES.len());
                } else {
                    self.arp_rate = wrap(self.arp_rate, d, RATES.len());
                }
                out.push(Command::SetStyle(self.style()));
            }
            Item::Velocidad => {
                self.strum_speed = wrap(self.strum_speed, d, STRUM_MS.len());
                out.push(Command::SetStyle(self.style()));
            }
            Item::Kit => {
                self.kit = wrap(self.kit, d, KITS.len());
                out.push(Command::SetKit(self.kit as u8));
            }
            Item::Estilo => {
                // Uno más que los estilos: "Mi ritmo".
                self.drum_style = wrap(self.drum_style, d, STYLES.len() + 1);
                out.push(Command::SetPattern(self.drum_pattern()));
            }
            Item::Variacion => {
                self.drum_variation = wrap(self.drum_variation, d, VARIATIONS.len());
                out.push(Command::SetPattern(self.drum_pattern()));
            }
            Item::Canciones => self.open_songs(),
        }
    }

    fn set_play_mode(&mut self, m: PlayMode, out: &mut Vec<Command>) {
        if m != self.play_mode {
            self.play_mode = m;
            self.current = None;
            self.knob_color = 0;
            self.held.clear();
            out.push(Command::StopChord);
            for (_, n) in self.melody.drain(..) {
                out.push(Command::NoteOff {
                    note: n.clamp(0, 127) as u8,
                });
            }
            out.push(Command::SetStyle(self.style()));
            self.last_hit = None;
            match m {
                PlayMode::Ritmos | PlayMode::Secuencia => self.set_drums_playing(true, out),
                _ if self.drums_playing => self.set_drums_playing(false, out),
                _ => {}
            }
        }
    }

    fn apply_sound(&mut self, out: &mut Vec<Command>) {
        let p = SOUNDS[self.sound].patch;
        self.attack = p.env.attack;
        self.release = p.env.release;
        self.brightness = p.brightness;
        out.push(Command::SetPatch(p));
    }

    /// 🎲: valores al azar para el menú.
    fn randomize(&mut self, m: Menu, out: &mut Vec<Command>) {
        let mut r = |n: usize| {
            // xorshift32
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 17;
            self.rng ^= self.rng << 5;
            self.rng as usize % n
        };
        match m {
            Menu::Tono => {
                let tonic = r(12) as i32;
                let scale = Scale::ALL[r(Scale::ALL.len())];
                let mode = ModifierMode::ALL[r(ModifierMode::ALL.len())];
                self.key.tonic = tonic;
                self.key.scale = scale;
                self.mod_mode = mode;
                self.revoice(out);
            }
            Menu::Sonido => {
                let s = r(SOUNDS.len());
                let b = r(100) as f32 / 100.0;
                self.sound = s;
                self.apply_sound(out);
                self.brightness = 0.25 + b * 0.6;
                out.push(Command::SetParam(Param::Brightness(self.brightness)));
            }
            Menu::Modo => {
                let bpm = 70.0 + r(100) as f32;
                let pat = r(ARP_PATTERNS.len());
                let style = r(STYLES.len());
                let var = r(VARIATIONS.len());
                self.bpm = bpm;
                self.arp_pattern = pat;
                if self.play_mode == PlayMode::Secuencia {
                    // 🎲 en el secuenciador: empieza desde un ritmo al azar.
                    self.seq_pattern = drums::rhythm(style, var);
                } else {
                    self.drum_style = style;
                    self.drum_variation = var;
                }
                out.push(Command::SetBpm(self.bpm));
                out.push(Command::SetStyle(self.style()));
                out.push(Command::SetPattern(self.drum_pattern()));
            }
        }
    }

    fn tap(&mut self, now: f64) {
        self.taps.retain(|&t| now - t < 2.0);
        self.taps.push(now);
        if self.taps.len() >= 3 {
            let n = self.taps.len();
            let avg = (self.taps[n - 1] - self.taps[0]) / (n - 1) as f64;
            if avg > 0.0 {
                self.bpm = ((60.0 / avg) as f32).round().clamp(BPM_MIN, BPM_MAX);
            }
        }
        if self.taps.len() > 8 {
            self.taps.remove(0);
        }
    }

    /// Hace sonar el acorde de la tecla. `retrigger`: vuelve a atacar todas las notas.
    fn play_chord(&mut self, button: u8, high: bool, retrigger: bool, out: &mut Vec<Command>) {
        let chord = self.chord_for(button, high);
        let opts = VoicingOptions {
            bass: self.bass,
            inversion: 0,
        };
        let v = voice_lead(&self.last_upper, chord, opts);
        let notes: Vec<i32> = v.notes().into_iter().map(|n| n.clamp(0, 127)).collect();
        self.current.take();
        out.push(Command::chord(&v.upper, v.bass, 0.85, retrigger));
        self.last_upper = v.upper;
        self.current = Some(Sounding {
            button,
            high,
            chord,
            dir: self.dir(),
            notes,
        });
    }

    /// Vuelve a calcular el acorde que suena (cambió una flecha, la tonalidad…).
    fn revoice(&mut self, out: &mut Vec<Command>) {
        if let Some(s) = &self.current {
            let (b, h) = (s.button, s.high);
            let before = (s.chord, s.dir);
            if (self.chord_for(b, h), self.dir()) != before {
                self.play_chord(b, h, false, out);
            }
        }
    }

    /// Se suelta el acorde. En Zumbido sigue sonando (y se sigue viendo).
    fn stop_chord(&mut self, out: &mut Vec<Command>) {
        if self.current.is_some() {
            out.push(Command::ChordOff);
        }
        if self.play_mode != PlayMode::Zumbido {
            self.current = None;
        }
        self.knob_color = 0;
    }
}

/// Sube o baja un tiempo de forma exponencial (cada paso, ×1,25).
fn step_time(t: f32, d: i32, min: f32, max: f32) -> f32 {
    (t.max(min) * 1.25f32.powi(d)).clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapach_theory::ChordType;

    fn press(s: &mut State, c: Control) -> Vec<Command> {
        s.handle(Event::Press(c), 0.0)
    }
    fn release(s: &mut State, c: Control) -> Vec<Command> {
        s.handle(Event::Release(c), 0.0)
    }
    fn ons(cmds: &[Command]) -> Vec<u8> {
        cmds.iter()
            .filter_map(|c| match c {
                Command::NoteOn { note, .. } => Some(*note),
                _ => None,
            })
            .collect()
    }
    /// (notas, bajo, retrigger) del ChordOn.
    fn chord(cmds: &[Command]) -> Option<(Vec<u8>, Option<u8>, bool)> {
        cmds.iter().find_map(|c| match *c {
            Command::ChordOn {
                notes,
                len,
                bass,
                retrigger,
                ..
            } => Some((notes[..len as usize].to_vec(), bass, retrigger)),
            _ => None,
        })
    }
    fn offs(cmds: &[Command]) -> Vec<u8> {
        cmds.iter()
            .filter_map(|c| match c {
                Command::NoteOff { note } => Some(*note),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tocar_y_soltar_un_acorde() {
        let mut s = State::default();
        let c = press(&mut s, Control::Chord(0));
        assert_eq!(chord(&c), Some((vec![48, 52, 55], Some(36), true)));
        let c = release(&mut s, Control::Chord(0));
        assert_eq!(c, [Command::ChordOff]);
        assert!(s.current.is_none());
    }

    #[test]
    fn flecha_cambia_el_color_sin_repetir_notas_comunes() {
        let mut s = State::default();
        press(&mut s, Control::Chord(0));
        let c = press(&mut s, Control::Arrow(Arrow::Up)); // Do → Dom
        assert_eq!(s.current.as_ref().unwrap().chord.kind, ChordType::Minor);
        assert_eq!(chord(&c), Some((vec![48, 51, 55], Some(36), false)));
        let c = release(&mut s, Control::Arrow(Arrow::Up));
        assert_eq!(s.current.as_ref().unwrap().chord.kind, ChordType::Major);
        assert_eq!(chord(&c), Some((vec![48, 52, 55], Some(36), false)));
    }

    #[test]
    fn flecha_antes_del_acorde() {
        let mut s = State::default();
        press(&mut s, Control::Arrow(Arrow::Right));
        press(&mut s, Control::Chord(4)); // Sol → Solmaj7
        assert_eq!(s.current.as_ref().unwrap().chord.kind, ChordType::Maj7);
    }

    #[test]
    fn al_soltar_vuelve_al_acorde_anterior_pulsado() {
        let mut s = State::default();
        press(&mut s, Control::Chord(0));
        press(&mut s, Control::Chord(3));
        assert_eq!(s.current.as_ref().unwrap().button, 3);
        release(&mut s, Control::Chord(3));
        assert_eq!(s.current.as_ref().unwrap().button, 0);
        // Soltar uno que no suena no cambia nada.
        press(&mut s, Control::Chord(4));
        let c = release(&mut s, Control::Chord(0));
        assert!(c.is_empty());
        assert_eq!(s.current.as_ref().unwrap().button, 4);
    }

    #[test]
    fn menu_mantenido_con_flechas_cambia_tonalidad_sin_abrir_pagina() {
        let mut s = State::default();
        press(&mut s, Control::Menu(Menu::Tono));
        press(&mut s, Control::Arrow(Arrow::Right));
        release(&mut s, Control::Arrow(Arrow::Right));
        press(&mut s, Control::Arrow(Arrow::Up));
        release(&mut s, Control::Menu(Menu::Tono));
        assert_eq!(s.key.tonic, 1);
        assert_eq!(s.key.octave, 1);
        assert!(s.open_menu.is_none());
    }

    #[test]
    fn menu_pulsado_y_soltado_abre_la_pagina() {
        let mut s = State::default();
        press(&mut s, Control::Menu(Menu::Sonido));
        release(&mut s, Control::Menu(Menu::Sonido));
        assert_eq!(s.open_menu.map(|o| o.menu), Some(Menu::Sonido));
        // Flecha derecha: siguiente sonido.
        let c = press(&mut s, Control::Arrow(Arrow::Right));
        assert_eq!(s.sound, 1);
        assert!(matches!(c[0], Command::SetPatch(_)));
        // Abajo: siguiente opción.
        press(&mut s, Control::Arrow(Arrow::Down));
        assert_eq!(s.selected_item(), Some(Item::Ataque));
        // Volver a pulsar el menú lo cierra.
        let _ = s.selected_item();
        press(&mut s, Control::Menu(Menu::Sonido));
        release(&mut s, Control::Menu(Menu::Sonido));
        assert!(s.open_menu.is_none());
    }

    #[test]
    fn perilla_magica_recorre_colores() {
        let mut s = State::default();
        press(&mut s, Control::Chord(0));
        s.handle(Event::Turn(Knob::Magica, 1), 0.0);
        assert_eq!(s.current.as_ref().unwrap().chord.kind, ChordType::Minor);
        s.handle(Event::Turn(Knob::Magica, 1), 0.0);
        assert_eq!(s.current.as_ref().unwrap().chord.kind, ChordType::Maj7);
        s.handle(Event::Turn(Knob::Magica, -3), 0.0);
        assert_eq!(s.current.as_ref().unwrap().chord.kind, ChordType::Aug);
        // Al soltar el acorde vuelve a "normal".
        release(&mut s, Control::Chord(0));
        assert_eq!(s.knob_color, 0);
    }

    #[test]
    fn volumen_y_silencio() {
        let mut s = State::default();
        s.handle(Event::Turn(Knob::Volumen, 2), 0.0);
        assert!((s.volume - 0.78).abs() < 1e-5);
        let c = press(&mut s, Control::KnobPress(Knob::Volumen));
        assert_eq!(c, [Command::SetParam(Param::Volume(0.0))]);
        s.handle(Event::Turn(Knob::Volumen, -100), 0.0);
        assert_eq!(s.volume, 0.0);
    }

    #[test]
    fn perilla_con_menu_mantenido() {
        let mut s = State::default();
        press(&mut s, Control::Menu(Menu::Modo));
        let before = s.brightness;
        s.handle(Event::Turn(Knob::Volumen, 1), 0.0);
        assert!(s.brightness > before);
        release(&mut s, Control::Menu(Menu::Modo));
        assert!(s.open_menu.is_none());
    }

    #[test]
    fn tap_tempo() {
        let mut s = State::default();
        for i in 0..4 {
            s.handle(Event::Press(Control::Tap), i as f64 * 0.5);
        }
        assert_eq!(s.bpm, 120.0);
        for i in 0..4 {
            s.handle(Event::Press(Control::Tap), 10.0 + i as f64 * 0.6);
        }
        assert_eq!(s.bpm, 100.0);
    }

    #[test]
    fn modo_melodia_toca_notas_de_la_escala() {
        let mut s = State::default();
        set_mode(&mut s, PlayMode::Melodia);
        let c = press(&mut s, Control::Chord(2));
        assert_eq!(ons(&c), [64]);
        let c = press(&mut s, Control::ChordHigh(0));
        assert_eq!(ons(&c), [72]);
        let c = release(&mut s, Control::Chord(2));
        assert_eq!(offs(&c), [64]);
    }

    #[test]
    fn aleatorio_no_rompe_nada() {
        let mut s = State::default();
        for m in Menu::ALL {
            for _ in 0..50 {
                press(&mut s, Control::Menu(m));
                press(&mut s, Control::Record);
                release(&mut s, Control::Record);
                release(&mut s, Control::Menu(m));
            }
        }
        assert!(s.open_menu.is_none());
        assert!((BPM_MIN..=BPM_MAX).contains(&s.bpm));
    }

    #[test]
    fn cambiar_tonalidad_mientras_suena() {
        let mut s = State::default();
        press(&mut s, Control::Chord(0));
        let mut out = Vec::new();
        s.change(Item::Tonalidad, 2, &mut out);
        let root = s.current.as_ref().unwrap().chord.root;
        assert_eq!(root.rem_euclid(12), 2);
        assert!(chord(&out).is_some());
    }

    #[test]
    fn modos_de_toque_mandan_su_estilo() {
        let mut s = State::default();
        let mut out = Vec::new();
        s.change(Item::Modo, 2, &mut out);
        assert_eq!(s.play_mode, PlayMode::Arpegio);
        assert!(out.contains(&Command::SetStyle(Style::Arp {
            pattern: 0,
            rate: 3
        })));
        // El menú rojo enseña Patrón y Figura solo en Arpegio.
        assert!(s.items(Menu::Modo).contains(&Item::Patron));
        s.change(Item::Modo, 1, &mut out);
        assert!(!s.items(Menu::Modo).contains(&Item::Patron));
        assert!(s.items(Menu::Modo).contains(&Item::Figura));
    }

    fn set_mode(s: &mut State, m: PlayMode) -> Vec<Command> {
        let pos = |m: PlayMode| PlayMode::ALL.iter().position(|&x| x == m).unwrap() as i32;
        let mut out = Vec::new();
        s.change(Item::Modo, pos(m) - pos(s.play_mode), &mut out);
        assert_eq!(s.play_mode, m);
        out
    }

    #[test]
    fn bateria_toca_golpes() {
        let mut s = State::default();
        set_mode(&mut s, PlayMode::Bateria);
        let c = press(&mut s, Control::Chord(1));
        assert_eq!(
            c,
            [Command::DrumHit {
                drum: 1,
                velocity: 1.0
            }]
        );
        assert!(s.chord_held(1));
        release(&mut s, Control::Chord(1));
        assert!(!s.chord_held(1));
        // Play arranca el ritmo.
        let c = press(&mut s, Control::Play);
        assert!(c.contains(&Command::DrumsPlay(true)));
    }

    #[test]
    fn ritmos_arranca_solo_y_para_al_salir() {
        let mut s = State::default();
        let c = set_mode(&mut s, PlayMode::Ritmos);
        assert!(c.contains(&Command::DrumsPlay(true)));
        assert!(s.drums_playing);
        // Encima se tocan acordes normales.
        let c = press(&mut s, Control::Chord(0));
        assert!(chord(&c).is_some());
        let mut out = Vec::new();
        s.change(Item::Variacion, 1, &mut out);
        assert!(matches!(out[0], Command::SetPattern(_)));
        let c = set_mode(&mut s, PlayMode::Tocar);
        assert!(c.contains(&Command::DrumsPlay(false)));
    }

    #[test]
    fn secuenciador_enciende_pasos() {
        let mut s = State::default();
        set_mode(&mut s, PlayMode::Secuencia);
        // Elegir la caja tocándola y encender los pasos 5 y 13 desde el mando.
        press(&mut s, Control::Chord(1));
        release(&mut s, Control::Chord(1));
        press(&mut s, Control::Pad(5));
        press(&mut s, Control::Pad(13));
        assert_eq!(s.seq_pattern[4], 0b10);
        assert_eq!(s.seq_pattern[12], 0b10);
        // Desde el teclado: 1 y Q son los pasos 1 y 9; ↓ cambia de instrumento.
        press(&mut s, Control::Arrow(Arrow::Down));
        press(&mut s, Control::Track(0));
        press(&mut s, Control::ChordHigh(0));
        assert_eq!(s.seq_pattern[0], 0b100);
        assert_eq!(s.seq_pattern[8], 0b100);
        // Volver a pulsar apaga.
        press(&mut s, Control::Track(0));
        assert_eq!(s.seq_pattern[0], 0);
        // Pulsar la perilla mágica sale a Ritmos con "Mi ritmo".
        press(&mut s, Control::KnobPress(Knob::Magica));
        assert_eq!(s.play_mode, PlayMode::Ritmos);
        assert_eq!(s.drum_pattern(), s.seq_pattern);
        assert!(s.drums_playing);
    }

    #[test]
    fn el_mando_sigue_tocando_acordes_fuera_del_secuenciador() {
        let mut s = State::default();
        let c = press(&mut s, Control::Pad(1));
        assert!(chord(&c).is_some());
        press(&mut s, Control::Pad(13));
        release(&mut s, Control::Pad(13));
        assert_eq!(s.open_menu.map(|o| o.menu), Some(Menu::Tono));
    }

    #[test]
    fn looper_grabar_parar_y_siguiente_pista() {
        let mut s = State::default();
        s.sync(LooperStatus::default(), 1.0);
        press(&mut s, Control::Record);
        assert!(s.looper_visible());
        let c = release(&mut s, Control::Record);
        assert_eq!(c, [Command::LoopRecord(0)]);
        assert_eq!(s.looper.recording, Some(0));
        // El motor dice que graba la 0.
        let rec = LooperStatus {
            used: 1,
            recording: Some(0),
            playing: true,
            ..Default::default()
        };
        s.sync(rec, 2.0);
        // Pulsar otra vez para enseguida.
        let c = press(&mut s, Control::Record);
        assert_eq!(c, [Command::LoopRecord(0)]);
        release(&mut s, Control::Record);
        s.sync(
            LooperStatus {
                recording: None,
                ..rec
            },
            3.0,
        );
        assert_eq!(s.track, 1);
        // Deshacer borra la última grabación.
        let c = press(&mut s, Control::Undo);
        assert_eq!(c, [Command::LoopClear(0)]);
    }

    #[test]
    fn mantener_pista_la_borra() {
        let mut s = State::default();
        s.sync(LooperStatus::default(), 10.0);
        press(&mut s, Control::Track(3));
        assert_eq!(s.track, 3);
        assert!(s.sync(LooperStatus::default(), 11.0).is_empty());
        let c = s.sync(LooperStatus::default(), 11.6);
        assert_eq!(c, [Command::LoopClear(3)]);
        // Soltar antes no borra.
        press(&mut s, Control::Track(2));
        release(&mut s, Control::Track(2));
        assert!(s.sync(LooperStatus::default(), 20.0).is_empty());
    }

    #[test]
    fn grabar_mas_acorde_elige_pista_en_el_mando() {
        let mut s = State::default();
        press(&mut s, Control::Pad(8));
        let c = press(&mut s, Control::Pad(3));
        assert!(c.is_empty());
        assert_eq!(s.track, 2);
        release(&mut s, Control::Pad(3));
        // Al soltar Grabar no empieza a grabar, porque se ha elegido pista.
        let c = release(&mut s, Control::Pad(8));
        assert!(c.is_empty());
    }

    #[test]
    fn play_controla_el_looper_si_hay_algo() {
        let mut s = State::default();
        s.sync(
            LooperStatus {
                used: 1,
                playing: true,
                length: 2.0,
                ..Default::default()
            },
            0.0,
        );
        let c = press(&mut s, Control::Play);
        assert_eq!(c, [Command::LoopPlay(false)]);
    }

    #[test]
    fn pantalla_de_canciones() {
        let mut s = State::default();
        press(&mut s, Control::Canciones);
        assert_eq!(s.screen, Screen::Canciones);
        assert_eq!(s.take_actions(), [Action::OpenSongs]);
        s.song_count = 2;
        // ← desde la primera da la vuelta a "Nueva".
        press(&mut s, Control::Arrow(Arrow::Left));
        assert_eq!(s.song_focus, SongFocus::Card(2));
        press(&mut s, Control::Record);
        assert_eq!(s.take_actions(), [Action::New]);
        assert_eq!(s.screen, Screen::Tocar);

        // Exportar a OGG: abajo, derecha, Espacio.
        press(&mut s, Control::Canciones);
        s.take_actions();
        press(&mut s, Control::Arrow(Arrow::Down));
        press(&mut s, Control::Arrow(Arrow::Right));
        press(&mut s, Control::Record);
        assert_eq!(s.take_actions(), [Action::Export(Format::Ogg)]);
        // Sigue en la pantalla; F5 sale.
        press(&mut s, Control::Canciones);
        assert_eq!(s.screen, Screen::Tocar);
    }

    #[test]
    fn canciones_desde_el_menu_gris() {
        let mut s = State::default();
        s.change(Item::Canciones, 1, &mut Vec::new());
        assert_eq!(s.screen, Screen::Canciones);
    }

    #[test]
    fn zumbido_sigue_al_soltar_y_play_lo_para() {
        let mut s = State::default();
        s.change(Item::Modo, 4, &mut Vec::new());
        assert_eq!(s.play_mode, PlayMode::Zumbido);
        press(&mut s, Control::Chord(0));
        release(&mut s, Control::Chord(0));
        assert!(s.current.is_some());
        let c = press(&mut s, Control::Play);
        assert_eq!(c, [Command::StopChord]);
        assert!(s.current.is_none());
    }
}
