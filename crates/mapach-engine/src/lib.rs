//! Motor de audio de Mapachsynthesizer.
//!
//! El [`Engine`] es puro y determinista: recibe [`Command`]s y rellena buffers estéreo
//! intercalados. No sabe nada de `cpal`, así que se puede probar y renderizar sin tarjeta de
//! sonido. El módulo [`audio`] lo conecta a la salida real.

pub mod audio;
pub mod drums;
pub mod looper;
mod patch;
pub mod performer;
mod voice;

pub use patch::{Kind, Patch, SOUNDS, Sound};
pub use performer::{ARP_PATTERNS, MAX_CHORD, RATES, STRUM_MS, Style};

use drums::{DrumMachine, Pattern};
use looper::{Looper, LooperStatus};
use performer::{Out, Performer};

use mapach_dsp::{AdsrParams, Limiter, Smoothed, db_to_gain};
use voice::Voice;

/// Parámetros que se controlan con las perillas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Param {
    /// Volumen general, 0–1.
    Volume(f32),
    /// Ataque en segundos.
    Attack(f32),
    /// Release en segundos.
    Release(f32),
    /// Brillo (corte del filtro), 0–1.
    Brightness(f32),
    /// Techo del limitador en dBFS (volumen máximo, modo papá).
    MaxLevelDb(f32),
}

/// Órdenes que la interfaz manda al motor por el ring buffer.
///
/// Los búferes van en `Box`: los reserva la interfaz y el motor los devuelve en una
/// [`Reply`], así nunca se reserva ni se libera memoria en el hilo de audio.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Empieza a sonar una nota MIDI. `velocity` de 0 a 1.
    NoteOn {
        note: u8,
        velocity: f32,
    },
    /// Suelta una nota MIDI.
    NoteOff {
        note: u8,
    },
    /// Suelta todas las notas.
    AllNotesOff,
    /// Cambia el sonido (las perillas de ataque, release y brillo vuelven a los del sonido).
    SetPatch(Patch),
    SetParam(Param),
    /// Toca un acorde con el modo de toque actual. `notes[..len]` sin el bajo.
    /// `retrigger`: tecla nueva; si es `false` solo cambia lo necesario (flechas).
    ChordOn {
        notes: [u8; MAX_CHORD],
        len: u8,
        bass: Option<u8>,
        velocity: f32,
        retrigger: bool,
    },
    /// Se suelta la tecla del acorde.
    ChordOff,
    /// Para todo lo que toca el intérprete (también el zumbido).
    StopChord,
    SetStyle(Style),
    SetBpm(f32),
    /// Golpe de batería en directo (instrumento 0–6).
    DrumHit {
        drum: u8,
        velocity: f32,
    },
    SetKit(u8),
    SetPattern(Pattern),
    /// Arranca o para la caja de ritmos.
    DrumsPlay(bool),
    /// Empieza o termina de grabar una pista del looper (0–5).
    LoopRecord(u8),
    /// Reproduce o para el looper.
    LoopPlay(bool),
    LoopClear(u8),
    LoopMute(u8, bool),
    LoopVolume(u8, f32),
    /// Copia una pista del looper en el búfer (de `looper_capacity() * 2` muestras).
    LoopCopy {
        track: u8,
        buf: Box<[f32]>,
    },
    /// Mete en el looper una pista cargada (búfer de `looper_capacity() * 2` muestras).
    LoopLoad {
        track: u8,
        buf: Box<[f32]>,
        length: usize,
        used: bool,
    },
}

/// Respuestas del motor a la interfaz.
#[derive(Debug)]
pub enum Reply {
    /// Una pista copiada.
    Track(looper::TrackCopy),
    /// Un búfer que ya no se usa: hay que liberarlo fuera del hilo de audio.
    Free(Box<[f32]>),
}

impl Command {
    /// Construye un `ChordOn` a partir de una lista de notas.
    pub fn chord(notes: &[i32], bass: Option<i32>, velocity: f32, retrigger: bool) -> Command {
        let mut arr = [0u8; MAX_CHORD];
        let len = notes.len().min(MAX_CHORD);
        for (a, &n) in arr.iter_mut().zip(notes) {
            *a = n.clamp(0, 127) as u8;
        }
        Command::ChordOn {
            notes: arr,
            len: len as u8,
            bass: bass.map(|b| b.clamp(0, 127) as u8),
            velocity,
            retrigger,
        }
    }
}

/// Búfer fijo para las órdenes del intérprete (sin reservar memoria).
struct OutBuf {
    buf: [Out; 160],
    len: usize,
}

impl OutBuf {
    fn new() -> Self {
        Self {
            buf: [Out::Off(0); 160],
            len: 0,
        }
    }

    fn push(&mut self, o: Out) {
        if self.len < self.buf.len() {
            self.buf[self.len] = o;
            self.len += 1;
        }
    }
}

/// Número máximo de voces simultáneas.
pub const MAX_VOICES: usize = 24;

/// Techo por defecto del limitador: −12 dBFS.
pub const DEFAULT_MAX_DB: f32 = -12.0;

/// Convierte una nota MIDI en frecuencia (La4 = 69 = 440 Hz).
pub fn midi_to_freq(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// Brillo (0–1) a frecuencia de corte: de 80 Hz a 16 kHz, exponencial.
pub fn brightness_to_hz(b: f32) -> f32 {
    80.0 * 200f32.powf(b.clamp(0.0, 1.0))
}

/// El motor de síntesis.
pub struct Engine {
    sample_rate: f32,
    voices: [Voice; MAX_VOICES],
    patch: Patch,
    env: AdsrParams,
    brightness: Smoothed,
    volume: Smoothed,
    limiter: Limiter,
    performer: Performer,
    drums: DrumMachine,
    looper: Looper,
    bpm: f32,
    /// Respuestas pendientes (como mucho unas pocas: no reserva en el hilo de audio).
    replies: [Option<Reply>; 4],
    /// Contador para saber qué voz es la más vieja.
    clock: u64,
    peak: f32,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let patch = Patch::default();
        Self {
            sample_rate,
            voices: [Voice::default(); MAX_VOICES],
            patch,
            env: patch.env,
            brightness: Smoothed::new(patch.brightness, 0.02, sample_rate),
            volume: Smoothed::new(0.7, 0.02, sample_rate),
            limiter: Limiter::new(db_to_gain(DEFAULT_MAX_DB), 0.15, sample_rate),
            performer: Performer::new(sample_rate),
            drums: DrumMachine::new(sample_rate),
            looper: Looper::new(sample_rate),
            bpm: 120.0,
            replies: [None, None, None, None],
            clock: 0,
            peak: 0.0,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn patch(&self) -> &Patch {
        &self.patch
    }

    /// Pico de salida del último bloque (lineal).
    pub fn peak(&self) -> f32 {
        self.peak
    }

    /// Aplica una orden. No reserva memoria.
    pub fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::NoteOn { note, velocity } => self.note_on(note, velocity),
            Command::NoteOff { note } => {
                for v in self.voices.iter_mut().filter(|v| v.is_held(note)) {
                    v.release();
                }
            }
            Command::AllNotesOff => {
                let mut ob = OutBuf::new();
                self.performer.stop(&mut |o| ob.push(o));
                self.voices.iter_mut().for_each(Voice::release);
            }
            Command::ChordOn {
                notes,
                len,
                bass,
                velocity,
                retrigger,
            } => {
                let mut ob = OutBuf::new();
                let len = (len as usize).min(MAX_CHORD);
                self.performer
                    .chord_on(&notes[..len], bass, velocity, retrigger, &mut |o| {
                        ob.push(o)
                    });
                self.apply(&ob);
            }
            Command::ChordOff => {
                let mut ob = OutBuf::new();
                self.performer.chord_off(&mut |o| ob.push(o));
                self.apply(&ob);
            }
            Command::StopChord => {
                let mut ob = OutBuf::new();
                self.performer.stop(&mut |o| ob.push(o));
                self.apply(&ob);
            }
            Command::SetStyle(st) => {
                let mut ob = OutBuf::new();
                self.performer.set_style(st, &mut |o| ob.push(o));
                self.apply(&ob);
            }
            Command::SetBpm(b) => {
                self.bpm = b.clamp(20.0, 400.0);
                self.performer.set_bpm(self.bpm);
            }
            Command::DrumHit { drum, velocity } => self.drums.hit(drum as usize, velocity),
            Command::SetKit(k) => self.drums.set_kit(k as usize),
            Command::SetPattern(p) => self.drums.set_pattern(p),
            Command::DrumsPlay(on) => self.drums.set_playing(on),
            Command::LoopRecord(t) => {
                let bpm = self.drums.is_playing().then_some(self.bpm);
                self.looper.toggle_record(t as usize, bpm);
            }
            Command::LoopPlay(on) => self.looper.set_playing(on),
            Command::LoopClear(t) => self.looper.clear(t as usize),
            Command::LoopMute(t, m) => self.looper.set_muted(t as usize, m),
            Command::LoopVolume(t, v) => self.looper.set_volume(t as usize, v),
            Command::LoopCopy { track, buf } => {
                if let Some(back) = self.looper.request_copy(track as usize, buf) {
                    self.reply(Reply::Free(back));
                }
            }
            Command::LoopLoad {
                track,
                buf,
                length,
                used,
            } => {
                let old = self.looper.load_track(track as usize, buf, length, used);
                self.looper.reset_if_empty();
                self.reply(Reply::Free(old));
            }
            Command::SetPatch(p) => {
                self.patch = p;
                self.env = p.env;
                self.brightness.set(p.brightness);
            }
            Command::SetParam(p) => match p {
                Param::Volume(v) => self.volume.set(v.clamp(0.0, 1.0)),
                Param::Attack(a) => self.env.attack = a.clamp(0.0, 5.0),
                Param::Release(r) => self.env.release = r.clamp(0.0, 8.0),
                Param::Brightness(b) => self.brightness.set(b.clamp(0.0, 1.0)),
                Param::MaxLevelDb(db) => self.limiter.set_ceiling(db_to_gain(db.clamp(-40.0, 0.0))),
            },
        }
    }

    fn apply(&mut self, ob: &OutBuf) {
        for &o in &ob.buf[..ob.len] {
            match o {
                Out::On(n, v) => self.note_on(n, v),
                Out::Off(n) => {
                    for v in self.voices.iter_mut().filter(|v| v.is_held(n)) {
                        v.release();
                    }
                }
            }
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        self.clock += 1;
        // 1) Misma nota que ya suena: se reutiliza. 2) Voz libre. 3) La más silenciosa de las
        // soltadas. 4) La más vieja.
        let idx = self
            .voices
            .iter()
            .position(|v| v.is_active() && v.note == note)
            .or_else(|| self.voices.iter().position(|v| !v.is_active()))
            .or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.is_released())
                    .min_by(|a, b| a.1.level().total_cmp(&b.1.level()))
                    .map(|(i, _)| i)
            })
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.started)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });
        let pan = pan_for_note(note);
        self.voices[idx].start(note, velocity.clamp(0.0, 1.0), pan, self.clock, &self.patch);
    }

    /// Paso de la caja de ritmos que suena, si está en marcha.
    pub fn drum_step(&self) -> Option<usize> {
        self.drums.step()
    }

    fn reply(&mut self, r: Reply) {
        if let Some(slot) = self.replies.iter_mut().find(|s| s.is_none()) {
            *slot = Some(r);
        }
        // Si no cabe (no debería pasar), se pierde y se libera aquí: es un caso de error.
    }

    /// Saca una respuesta pendiente.
    pub fn take_reply(&mut self) -> Option<Reply> {
        if let Some(c) = self.looper.poll_copy() {
            self.reply(Reply::Track(c));
        }
        self.replies.iter_mut().find_map(|s| s.take())
    }

    /// Frames que caben en una pista del looper.
    pub fn looper_capacity(&self) -> usize {
        self.looper.capacity()
    }

    pub fn looper_status(&self) -> LooperStatus {
        self.looper.status()
    }

    /// Número de voces que están sonando.
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// Rellena un buffer estéreo intercalado (`[L, R, L, R, …]`).
    pub fn process(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let frames = out.len() / 2;
        // El brillo y el volumen se suavizan por bloques de 32 muestras.
        const SUB: usize = 32;
        let mut start = 0;
        while start < frames {
            let n = SUB.min(frames - start);
            let mut ob = OutBuf::new();
            self.performer.advance(n, &mut |o| ob.push(o));
            self.apply(&ob);
            let mut bright = 0.0;
            for _ in 0..n {
                bright = self.brightness.tick();
            }
            let cutoff = brightness_to_hz(bright);
            let block = &mut out[start * 2..(start + n) * 2];
            for v in self.voices.iter_mut().filter(|v| v.is_active()) {
                v.render(block, &self.patch, &self.env, cutoff, self.sample_rate);
            }
            self.drums.render(block, self.bpm);
            let bpm = self.drums.is_playing().then_some(self.bpm);
            self.looper.process(block, bpm);
            start += n;
        }

        let ceiling = self.limiter.ceiling();
        let mut peak = 0.0f32;
        for frame in out.chunks_exact_mut(2) {
            let vol = self.volume.tick();
            // Curva de volumen cuadrática, y la mezcla escalada al techo del limitador.
            let g = vol * vol * ceiling * 3.0;
            let (l, r) = self.limiter.process(frame[0] * g, frame[1] * g);
            frame[0] = l;
            frame[1] = r;
            peak = peak.max(l.abs()).max(r.abs());
        }
        self.peak = peak;
    }
}

/// Las notas graves al centro y las agudas un poco abiertas, alternando lado.
fn pan_for_note(note: u8) -> f32 {
    let spread = ((note as f32 - 48.0) / 36.0).clamp(0.0, 1.0) * 0.4;
    if note.is_multiple_of(2) {
        -spread
    } else {
        spread
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(note: u8) -> Command {
        Command::NoteOn {
            note,
            velocity: 1.0,
        }
    }

    fn render(e: &mut Engine, secs: f32) -> Vec<f32> {
        let mut buf = vec![0.0; (secs * e.sample_rate()) as usize * 2];
        e.process(&mut buf);
        buf
    }

    #[test]
    fn la_es_440() {
        assert!((midi_to_freq(69.0) - 440.0).abs() < 1e-3);
        assert!((midi_to_freq(81.0) - 880.0).abs() < 1e-2);
    }

    #[test]
    fn silencio_sin_notas() {
        let mut e = Engine::new(48_000.0);
        let buf = render(&mut e, 0.1);
        assert!(buf.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn nota_suena_y_se_apaga() {
        let mut e = Engine::new(48_000.0);
        e.handle(on(69));
        let buf = render(&mut e, 0.3);
        assert!(e.peak() > 0.05, "pico {}", e.peak());
        assert!(buf.iter().all(|s| s.is_finite()));

        e.handle(Command::NoteOff { note: 69 });
        render(&mut e, 3.0);
        assert_eq!(e.active_voices(), 0);
        let buf = render(&mut e, 0.1);
        assert!(buf.iter().all(|&s| s.abs() < 1e-6));
    }

    #[test]
    fn nunca_pasa_del_techo_con_todas_las_voces() {
        for sound in SOUNDS {
            let mut e = Engine::new(48_000.0);
            e.handle(Command::SetPatch(sound.patch));
            e.handle(Command::SetParam(Param::Volume(1.0)));
            for n in 0..40u8 {
                e.handle(on(30 + n * 2));
            }
            let buf = render(&mut e, 1.0);
            let c = db_to_gain(DEFAULT_MAX_DB);
            assert!(buf.iter().all(|s| s.abs() <= c + 1e-6), "{}", sound.name);
            assert!(e.active_voices() <= MAX_VOICES);
        }
    }

    #[test]
    fn determinista() {
        let run = || {
            let mut e = Engine::new(48_000.0);
            e.handle(on(60));
            e.handle(on(64));
            render(&mut e, 0.2)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn robar_voces_no_rompe_nada() {
        let mut e = Engine::new(48_000.0);
        for n in 0..200u8 {
            e.handle(on(n % 100 + 20));
        }
        assert_eq!(e.active_voices(), MAX_VOICES);
        let buf = render(&mut e, 0.1);
        assert!(buf.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn arpegio_en_el_motor() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::SetStyle(Style::Arp {
            pattern: 0,
            rate: 3,
        }));
        e.handle(Command::chord(&[60, 64, 67], Some(48), 0.9, true));
        render(&mut e, 0.05);
        // Bajo + una nota del arpegio.
        assert_eq!(e.active_voices(), 2);
        e.handle(Command::ChordOff);
        render(&mut e, 3.0);
        assert_eq!(e.active_voices(), 0);
    }

    #[test]
    fn ritmo_suena_y_no_pasa_del_techo() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::SetParam(Param::Volume(1.0)));
        e.handle(Command::SetPattern(drums::rhythm(2, 6)));
        e.handle(Command::DrumsPlay(true));
        for n in 0..12u8 {
            e.handle(on(40 + n * 3));
        }
        let buf = render(&mut e, 2.0);
        assert!(e.drum_step().is_some());
        let c = db_to_gain(DEFAULT_MAX_DB);
        assert!(buf.iter().all(|s| s.abs() <= c + 1e-6));
        e.handle(Command::DrumsPlay(false));
        assert!(e.drum_step().is_none());
    }

    #[test]
    fn el_looper_repite_lo_grabado() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::LoopRecord(0));
        e.handle(on(60));
        render(&mut e, 0.5);
        e.handle(Command::NoteOff { note: 60 });
        render(&mut e, 0.5);
        e.handle(Command::LoopRecord(0));
        assert!((e.looper_status().length - 1.0).abs() < 0.01);
        // Ya no suena ninguna voz, pero el looper repite la nota.
        render(&mut e, 2.0);
        assert_eq!(e.active_voices(), 0);
        render(&mut e, 0.95);
        let buf = render(&mut e, 0.3);
        assert!(e.peak() > 0.02, "pico {}", e.peak());
        assert!(buf.iter().any(|s| s.abs() > 0.02));
    }

    #[test]
    fn sacar_una_pista_del_motor() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::LoopRecord(0));
        e.handle(on(60));
        render(&mut e, 0.5);
        e.handle(Command::LoopRecord(0));
        let cap = e.looper_capacity();
        e.handle(Command::LoopCopy {
            track: 0,
            buf: vec![0.0; cap * 2].into_boxed_slice(),
        });
        let mut got = None;
        for _ in 0..100 {
            render(&mut e, 0.01);
            if let Some(Reply::Track(t)) = e.take_reply() {
                got = Some(t);
                break;
            }
        }
        let t = got.expect("la copia no llegó");
        assert_eq!(t.frames, 24_000);
        assert!(t.buf[..48_000].iter().any(|s| s.abs() > 0.01));
    }

    /// RMS de un acorde de Do mayor durante 1 s con cada sonido.
    fn loudness() -> Vec<(&'static str, f32)> {
        SOUNDS
            .iter()
            .map(|snd| {
                let mut e = Engine::new(48_000.0);
                e.handle(Command::SetPatch(snd.patch));
                // Volumen bajo para que el limitador no iguale los sonidos.
                e.handle(Command::SetParam(Param::Volume(0.3)));
                for n in [48, 52, 55, 60] {
                    e.handle(on(n));
                }
                let buf = render(&mut e, 1.0);
                let rms = (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt();
                (snd.name, rms)
            })
            .collect()
    }

    #[test]
    fn todos_los_sonidos_suenan_parecido_de_fuerte() {
        let l = loudness();
        let mut v: Vec<f32> = l.iter().map(|x| x.1).collect();
        v.sort_by(f32::total_cmp);
        let median = v[v.len() / 2];
        for (name, rms) in &l {
            let ratio = rms / median;
            assert!(
                (0.3..3.0).contains(&ratio),
                "{name}: {rms:.4} ({ratio:.2}× la mediana {median:.4})"
            );
        }
    }

    #[test]
    fn todos_los_sonidos_se_apagan() {
        for snd in SOUNDS {
            let mut e = Engine::new(48_000.0);
            e.handle(Command::SetPatch(snd.patch));
            e.handle(on(60));
            render(&mut e, 0.5);
            e.handle(Command::NoteOff { note: 60 });
            render(&mut e, 10.0);
            assert_eq!(e.active_voices(), 0, "{}", snd.name);
        }
    }

    #[test]
    fn volumen_cero_es_silencio() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::SetParam(Param::Volume(0.0)));
        e.handle(on(60));
        render(&mut e, 0.5);
        let buf = render(&mut e, 0.1);
        assert!(buf.iter().all(|s| s.abs() < 1e-4));
    }
}
