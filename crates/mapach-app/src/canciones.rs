//! Guardar, cargar y exportar canciones sin bloquear la pantalla.
//!
//! Las pistas se sacan del motor de una en una (el motor las copia a trozos), y escribir los
//! ficheros se hace en otro hilo.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use mapach_engine::audio::AudioOutput;
use mapach_engine::looper::{LooperStatus, TRACKS};
use mapach_engine::{Command, Reply};
use mapach_project::paths::{exports_dir, songs_dir};
use mapach_project::song::{self, SongInfo};
use mapach_project::{Format, Song, TrackAudio, export, funny_name};

use crate::ajustes::Ajustes;
use crate::state::State;

/// Techo del limitador al exportar (el mismo que en directo).
const EXPORT_CEILING_DB: f32 = mapach_engine::DEFAULT_MAX_DB;

/// Lo que necesitamos del motor: mandar órdenes y recibir respuestas.
pub trait Link {
    fn send(&mut self, c: Command);
    fn reply(&mut self) -> Option<Reply>;
    /// Frames que caben en una pista del looper.
    fn capacity(&self) -> usize;
}

impl Link for AudioOutput {
    fn send(&mut self, c: Command) {
        AudioOutput::send(self, c);
    }
    fn reply(&mut self) -> Option<Reply> {
        AudioOutput::reply(self)
    }
    fn capacity(&self) -> usize {
        self.looper_capacity
    }
}

/// La canción en la que se está trabajando.
#[derive(Debug, Clone)]
pub struct Current {
    pub nombre: String,
    pub color: [u8; 3],
    /// Carpeta donde está guardada (`None` si aún no se ha guardado nunca).
    pub carpeta: Option<PathBuf>,
}

impl Current {
    pub fn new_random() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let (nombre, color) = funny_name(seed);
        Self {
            nombre,
            color,
            carpeta: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum After {
    Save,
    Export(Format),
}

/// Recogida de pistas del motor en marcha.
struct Collect {
    remaining: Vec<usize>,
    waiting: bool,
    got: [Option<TrackAudio>; TRACKS],
    loop_frames: usize,
    after: After,
    ajustes: Ajustes,
    volumes: [f32; TRACKS],
    muted: [bool; TRACKS],
}

/// Resultado de un trabajo en segundo plano.
pub enum Done {
    Saved(PathBuf),
    Exported(PathBuf),
    Loaded(Box<Song<Ajustes>>, PathBuf),
    Failed(String),
}

/// Grabación de la canción entera (todo lo que suena).
pub struct Live {
    /// Audio estéreo intercalado.
    buf: Vec<f32>,
    /// Cuándo empezó (tiempo de la app).
    pub started: f64,
}

/// Duración máxima de la grabación de la canción entera.
pub const LIVE_MAX_SECONDS: f64 = 600.0;

pub struct Songs {
    pub current: Current,
    /// Grabando la canción entera.
    pub live: Option<Live>,
    pub list: Vec<SongInfo>,
    /// Qué se está haciendo ("Guardando…", "Creando MP3…").
    pub busy: Option<String>,
    /// Cuándo se guardó por última vez (tiempo de la app).
    pub last_saved: Option<f64>,
    /// Última exportación, para enseñar dónde está.
    pub last_export: Option<PathBuf>,
    collect: Option<Collect>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
    /// Hay cambios en el looper sin guardar, desde cuándo.
    dirty_since: Option<f64>,
    last_used: u8,
    sample_rate: u32,
    /// Carpetas (configurables para los tests).
    songs_dir: PathBuf,
    exports_dir: PathBuf,
}

impl Songs {
    pub fn new(current: Current, sample_rate: u32) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            current,
            live: None,
            list: Vec::new(),
            busy: None,
            last_saved: None,
            last_export: None,
            collect: None,
            tx,
            rx,
            dirty_since: None,
            last_used: 0,
            sample_rate,
            songs_dir: songs_dir(),
            exports_dir: exports_dir(),
        }
    }

    #[cfg(test)]
    fn with_dirs(mut self, songs: PathBuf, exports: PathBuf) -> Self {
        self.songs_dir = songs;
        self.exports_dir = exports;
        self
    }

    pub fn refresh_list(&mut self) {
        self.list = song::list_in(&self.songs_dir);
    }

    pub fn is_busy(&self) -> bool {
        self.busy.is_some()
    }

    /// Empieza a guardar (o a guardar y exportar) la canción actual.
    pub fn begin_save(&mut self, st: &State, audio: &mut impl Link, export: Option<Format>) {
        if self.collect.is_some() {
            return;
        }
        let after = export.map_or(After::Save, After::Export);
        let remaining: Vec<usize> = (0..TRACKS).filter(|&t| st.looper.has(t)).collect();
        self.busy = Some(match after {
            After::Save => "Guardando…".into(),
            After::Export(f) => format!("Creando {}…", f.name()),
        });
        self.dirty_since = None;
        let mut muted = [false; TRACKS];
        for (t, m) in muted.iter_mut().enumerate() {
            *m = st.looper.muted & (1 << t) != 0;
        }
        self.collect = Some(Collect {
            remaining,
            waiting: false,
            got: Default::default(),
            loop_frames: 0,
            after,
            ajustes: Ajustes::from_state(st),
            volumes: st.track_volume,
            muted,
        });
        self.pump(audio);
    }

    /// Pide la siguiente pista al motor o, si ya están todas, lanza el hilo que escribe.
    fn pump(&mut self, audio: &mut impl Link) {
        let Some(c) = &mut self.collect else { return };
        if c.waiting {
            return;
        }
        if let Some(&t) = c.remaining.first() {
            let buf = vec![0.0f32; audio.capacity() * 2].into_boxed_slice();
            audio.send(Command::LoopCopy {
                track: t as u8,
                buf,
            });
            c.waiting = true;
            return;
        }
        let Some(c) = self.collect.take() else { return };
        let mut song = Song::new(self.current.nombre.clone(), self.current.color, c.ajustes);
        song.frecuencia = self.sample_rate;
        song.vuelta = c.loop_frames;
        song.volumenes = c.volumes;
        song.silenciadas = c.muted;
        song.pistas = c.got;
        let base = self.songs_dir.clone();
        let exports = self.exports_dir.clone();
        // Primera vez: que no pise otra canción con el mismo nombre.
        if self.current.carpeta.is_none() {
            let mut n = 2;
            let original = song.nombre.clone();
            while song.folder_in(&base).exists() {
                song.nombre = format!("{original} {n}");
                n += 1;
            }
            self.current.nombre = song.nombre.clone();
        }
        let tx = self.tx.clone();
        let after = c.after;
        std::thread::spawn(move || {
            let res = (|| -> anyhow::Result<Done> {
                let dir = song.save_in(&base)?;
                if let After::Export(f) = after {
                    let mix = export::mixdown(
                        &song.pistas,
                        song.vuelta,
                        &song.volumenes,
                        &song.silenciadas,
                        song.frecuencia,
                        EXPORT_CEILING_DB,
                    );
                    let path = export::free_path(&exports, &song.nombre, f);
                    export::write(&path, &mix, song.frecuencia, f, &song.nombre)?;
                    let _ = tx.send(Done::Saved(dir));
                    return Ok(Done::Exported(path));
                }
                Ok(Done::Saved(dir))
            })();
            let _ = tx.send(res.unwrap_or_else(|e| Done::Failed(format!("{e:#}"))));
        });
    }

    /// Atiende todas las respuestas pendientes del motor.
    pub fn pump_replies(&mut self, audio: &mut impl Link) {
        while let Some(r) = audio.reply() {
            self.on_reply(r, audio);
        }
    }

    /// Respuesta del motor.
    pub fn on_reply(&mut self, r: Reply, audio: &mut impl Link) {
        match r {
            Reply::Track(copy) => {
                if let Some(c) = &mut self.collect {
                    c.waiting = false;
                    c.remaining.retain(|&t| t != copy.track);
                    let mut v = copy.buf.into_vec();
                    v.truncate(copy.frames * 2);
                    v.shrink_to_fit();
                    c.loop_frames = c.loop_frames.max(copy.frames);
                    if copy.frames > 0 {
                        c.got[copy.track] = Some(TrackAudio { samples: v });
                    }
                }
                self.pump(audio);
            }
            // Búferes viejos: se liberan aquí, fuera del hilo de audio.
            Reply::Free(buf) => drop(buf),
            Reply::FreeSample(s) => drop(s),
        }
    }

    /// Carga en segundo plano la canción `idx` de la lista.
    pub fn begin_load(&mut self, idx: usize) {
        let Some(info) = self.list.get(idx) else {
            return;
        };
        self.load_folder(info.carpeta.clone());
    }

    pub fn load_folder(&mut self, dir: PathBuf) {
        self.busy = Some("Abriendo…".into());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let msg = match Song::<Ajustes>::load(&dir) {
                Ok(s) => Done::Loaded(Box::new(s), dir),
                Err(e) => Done::Failed(format!("{e:#}")),
            };
            let _ = tx.send(msg);
        });
    }

    /// Mete en el motor una canción cargada.
    pub fn apply_loaded(
        &mut self,
        song: Song<Ajustes>,
        dir: PathBuf,
        st: &mut State,
        audio: &mut impl Link,
    ) {
        song.ajustes.apply(st);
        for c in st.initial_commands() {
            audio.send(c);
        }
        let cap = audio.capacity();
        let length = song.vuelta.min(cap);
        for t in 0..TRACKS {
            let mut buf = vec![0.0f32; cap * 2].into_boxed_slice();
            let used = if let Some(tr) = &song.pistas[t] {
                let n = tr.samples.len().min(cap * 2);
                buf[..n].copy_from_slice(&tr.samples[..n]);
                true
            } else {
                false
            };
            audio.send(Command::LoopLoad {
                track: t as u8,
                buf,
                length,
                used,
            });
            audio.send(Command::LoopVolume(t as u8, song.volumenes[t]));
            audio.send(Command::LoopMute(t as u8, song.silenciadas[t]));
            st.track_volume[t] = song.volumenes[t];
        }
        audio.send(Command::LoopPlay(false));
        self.current = Current {
            nombre: song.nombre,
            color: song.color,
            carpeta: Some(dir),
        };
        self.dirty_since = None;
    }

    /// Canción nueva: borra el looper y le pone otro nombre.
    pub fn new_song(&mut self, st: &mut State, audio: &mut impl Link) {
        for t in 0..TRACKS {
            audio.send(Command::LoopClear(t as u8));
            audio.send(Command::LoopVolume(t as u8, 1.0));
            audio.send(Command::LoopMute(t as u8, false));
            st.track_volume[t] = 1.0;
        }
        st.track = 0;
        self.current = Current::new_random();
        self.dirty_since = None;
    }

    /// Empieza o termina la grabación de la canción entera. Al terminar, la guarda como MP3
    /// en Exportadas (en otro hilo; el resultado llega por [`Self::poll`]).
    pub fn toggle_live(&mut self, now: f64, audio: &mut AudioOutput) -> bool {
        if self.live.is_some() {
            self.finish_live(audio);
            false
        } else {
            // Lo que hubiera en la cola de antes no cuenta.
            let mut old = Vec::new();
            audio.drain_song(&mut old);
            audio.send(Command::SongRecord(true));
            self.live = Some(Live {
                buf: Vec::with_capacity(self.sample_rate as usize * 2 * 60),
                started: now,
            });
            true
        }
    }

    /// Llamar en cada fotograma: recoge lo grabado y para sola al llegar al máximo.
    pub fn live_tick(&mut self, now: f64, audio: &mut AudioOutput) -> bool {
        let Some(live) = &mut self.live else {
            return false;
        };
        audio.drain_song(&mut live.buf);
        if now - live.started >= LIVE_MAX_SECONDS {
            self.finish_live(audio);
            return true;
        }
        false
    }

    fn finish_live(&mut self, audio: &mut AudioOutput) {
        audio.send(Command::SongRecord(false));
        let Some(mut live) = self.live.take() else {
            return;
        };
        audio.drain_song(&mut live.buf);
        if live.buf.len() < self.sample_rate as usize {
            // Menos de medio segundo: no vale la pena.
            return;
        }
        self.busy = Some("Guardando la grabación…".into());
        let tx = self.tx.clone();
        let rate = self.sample_rate;
        let name = format!("{} (grabación)", self.current.nombre);
        let exports = self.exports_dir.clone();
        std::thread::spawn(move || {
            let mut samples = live.buf;
            // Volumen igualado: no depende de cómo estuviera la perilla.
            let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            if peak > 1e-4 {
                let g = 0.89 / peak;
                samples.iter_mut().for_each(|s| *s *= g);
            }
            let path = export::free_path(&exports, &name, Format::Mp3);
            let msg = match export::write(&path, &samples, rate, Format::Mp3, &name) {
                Ok(()) => Done::Exported(path),
                Err(e) => Done::Failed(format!("{e:#}")),
            };
            let _ = tx.send(msg);
        });
    }

    /// ¿La canción actual está guardada (tiene nombre y carpeta)?
    pub fn is_saved(&self) -> bool {
        self.current.carpeta.is_some()
    }

    /// Cambia el nombre de la canción. Si ya estaba guardada, también su carpeta.
    pub fn rename(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El nombre no puede estar vacío".into());
        }
        if let Some(old) = self.current.carpeta.clone() {
            let new_dir = self
                .songs_dir
                .join(mapach_project::paths::safe_file_name(name));
            if new_dir != old {
                if new_dir.exists() {
                    return Err("Ya hay una canción con ese nombre".into());
                }
                std::fs::rename(&old, &new_dir).map_err(|e| e.to_string())?;
            }
            self.current.carpeta = Some(new_dir);
            // Que el cancion.ron lleve el nombre nuevo en el próximo guardado.
            self.dirty_since = Some(f64::MIN);
        }
        self.current.nombre = name.to_string();
        Ok(())
    }

    /// Prepara una copia: la canción pasa a llamarse `name` y se guardará en una carpeta nueva
    /// (la original se queda como estaba en disco).
    pub fn start_copy(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El nombre no puede estar vacío".into());
        }
        self.current.nombre = name.to_string();
        self.current.carpeta = None;
        Ok(())
    }

    /// Mira si el looper ha cambiado y, un par de segundos después, guarda solo.
    pub fn autosave(&mut self, now: f64, looper: LooperStatus, st: &State, audio: &mut impl Link) {
        if looper.used != self.last_used {
            self.last_used = looper.used;
            self.dirty_since.get_or_insert(now);
        }
        if let Some(since) = self.dirty_since
            && looper.recording.is_none()
            && self.collect.is_none()
            && now - since > 2.0
            // Solo las canciones ya guardadas (con nombre) se guardan solas.
            && self.current.carpeta.is_some()
        {
            self.begin_save(st, audio, None);
        }
    }

    /// Resultados de los hilos.
    pub fn poll(&mut self) -> Option<Done> {
        let d = self.rx.try_recv().ok()?;
        match &d {
            Done::Saved(dir) => {
                self.current.carpeta = Some(dir.clone());
                if !matches!(self.busy.as_deref(), Some(b) if b.starts_with("Creando")) {
                    self.busy = None;
                }
            }
            Done::Exported(p) => {
                self.busy = None;
                self.last_export = Some(p.clone());
            }
            Done::Loaded(..) | Done::Failed(_) => self.busy = None,
        }
        Some(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapach_engine::Engine;

    /// Motor de verdad, pero sin tarjeta de sonido: cada respuesta procesa un bloque.
    struct Fake {
        engine: Engine,
        buf: Vec<f32>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                engine: Engine::new(48_000.0),
                buf: vec![0.0; 512],
            }
        }
        fn run(&mut self, secs: f32) {
            for _ in 0..(secs * 48_000.0 / 256.0) as usize {
                self.engine.process(&mut self.buf);
            }
        }
    }

    impl Link for Fake {
        fn send(&mut self, c: Command) {
            self.engine.handle(c);
        }
        fn reply(&mut self) -> Option<Reply> {
            self.engine.process(&mut self.buf);
            self.engine.take_reply()
        }
        fn capacity(&self) -> usize {
            self.engine.looper_capacity()
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mapach-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Bombea respuestas y resultados hasta que llegue uno.
    fn wait(songs: &mut Songs, link: &mut Fake) -> Done {
        for _ in 0..2000 {
            songs.pump_replies(link);
            if let Some(d) = songs.poll() {
                return d;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("no terminó");
    }

    #[test]
    fn grabar_guardar_exportar_y_volver_a_abrir() {
        let base = tmp("songs");
        let exports = tmp("exports");
        let mut link = Fake::new();
        // Grabar 1 s en la pista 0 con una nota.
        link.send(Command::LoopRecord(0));
        link.send(Command::NoteOn {
            note: 60,
            velocity: 1.0,
        });
        link.run(1.0);
        link.send(Command::LoopRecord(0));
        link.send(Command::NoteOff { note: 60 });

        let mut st = State::default();
        st.looper = link.engine.looper_status();
        assert!(st.looper.has(0));
        let mut songs = Songs::new(
            Current {
                nombre: "Rana Veloz".into(),
                color: [1, 2, 3],
                carpeta: None,
            },
            48_000,
        )
        .with_dirs(base.clone(), exports.clone());

        // Guardar y exportar a MP3.
        songs.begin_save(&st, &mut link, Some(Format::Mp3));
        assert!(matches!(wait(&mut songs, &mut link), Done::Saved(_)));
        let Done::Exported(mp3) = wait(&mut songs, &mut link) else {
            panic!("no exportó");
        };
        assert!(mp3.exists());
        assert!(base.join("Rana Veloz/pistas/1.wav").exists());
        assert_eq!(songs.current.carpeta, Some(base.join("Rana Veloz")));

        // Canción nueva: el looper queda vacío.
        songs.new_song(&mut st, &mut link);
        link.run(0.01);
        assert!(link.engine.looper_status().is_empty());

        // Volver a abrir la guardada.
        songs.refresh_list();
        assert_eq!(songs.list.len(), 1);
        songs.begin_load(0);
        let Done::Loaded(song, dir) = wait(&mut songs, &mut link) else {
            panic!("no cargó");
        };
        assert_eq!(song.nombre, "Rana Veloz");
        songs.apply_loaded(*song, dir, &mut st, &mut link);
        link.run(0.01);
        let lp = link.engine.looper_status();
        assert!(lp.has(0));
        assert!(
            !lp.playing,
            "al abrir una canción, el looper no arranca solo"
        );
        assert!((lp.length - 1.0).abs() < 0.01);

        // El audio de vuelta suena.
        link.send(Command::LoopPlay(true));
        link.run(0.3);
        assert!(link.engine.peak() > 0.01);

        let _ = std::fs::remove_dir_all(base);
        let _ = std::fs::remove_dir_all(exports);
    }

    #[test]
    fn renombrar_y_copiar() {
        let base = tmp("rename");
        let mut link = Fake::new();
        let st = State::default();
        let mut songs = Songs::new(
            Current {
                nombre: "Rana Veloz".into(),
                color: [0; 3],
                carpeta: None,
            },
            48_000,
        )
        .with_dirs(base.clone(), tmp("rename-exp"));
        // Sin guardar, renombrar solo cambia el nombre y no se guarda sola.
        songs.rename("Mi canción").unwrap();
        assert!(!songs.is_saved());
        songs.begin_save(&st, &mut link, None);
        assert!(matches!(wait(&mut songs, &mut link), Done::Saved(_)));
        assert!(base.join("Mi canción").exists());
        // Guardada: renombrar mueve la carpeta.
        songs.rename("Otra").unwrap();
        assert!(base.join("Otra").exists());
        assert!(!base.join("Mi canción").exists());
        // Copiar: se guarda aparte y la original sigue.
        songs.start_copy("Otra copia").unwrap();
        songs.begin_save(&st, &mut link, None);
        assert!(matches!(wait(&mut songs, &mut link), Done::Saved(_)));
        assert!(base.join("Otra").exists() && base.join("Otra copia").exists());
        // No se puede pisar otra canción al renombrar.
        assert!(songs.rename("Otra").is_err());
        assert!(songs.rename("   ").is_err());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn no_pisa_otra_cancion_con_el_mismo_nombre() {
        let base = tmp("dup");
        let mut link = Fake::new();
        let st = State::default();
        for _ in 0..2 {
            let mut songs = Songs::new(
                Current {
                    nombre: "Luna Mágica".into(),
                    color: [0; 3],
                    carpeta: None,
                },
                48_000,
            )
            .with_dirs(base.clone(), tmp("dup-exp"));
            songs.begin_save(&st, &mut link, None);
            assert!(matches!(wait(&mut songs, &mut link), Done::Saved(_)));
        }
        assert!(base.join("Luna Mágica").exists());
        assert!(base.join("Luna Mágica 2").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn nombre_al_azar() {
        let c = Current::new_random();
        assert!(c.nombre.contains(' '));
        assert!(c.carpeta.is_none());
    }
}
