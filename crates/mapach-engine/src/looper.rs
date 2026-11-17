//! Looper de 6 pistas.
//!
//! - La primera grabación marca la duración de la vuelta. Si hay tempo (ritmo sonando), se
//!   redondea a compases enteros para que "cuadre".
//! - Las demás pistas graban una vuelta entera desde donde esté la reproducción y paran solas.
//! - Toda la memoria se reserva al crearlo: grabar no reserva nada en el hilo de audio.

pub const TRACKS: usize = 6;

/// Duración máxima de la vuelta, en segundos.
pub const MAX_SECONDS: f32 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rec {
    track: usize,
    /// Frames grabados.
    done: usize,
}

/// Estado del looper para la pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LooperStatus {
    /// Pistas con algo grabado (bit por pista).
    pub used: u8,
    /// Pista que se está grabando.
    pub recording: Option<usize>,
    pub playing: bool,
    /// Posición en la vuelta, 0–1 (en la primera grabación, segundos grabados / máximo).
    pub pos: f32,
    /// Duración de la vuelta en segundos (0 = aún no hay).
    pub length: f32,
    /// Pistas silenciadas (bit por pista).
    pub muted: u8,
}

impl LooperStatus {
    pub fn has(&self, track: usize) -> bool {
        self.used & (1 << track) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.used == 0
    }

    /// Empaqueta en un `u64` para pasarlo por un atómico.
    pub fn pack(&self) -> u64 {
        let rec = self.recording.map_or(0, |t| t as u64 + 1);
        (self.used as u64)
            | (self.muted as u64) << 8
            | rec << 16
            | (self.playing as u64) << 20
            | ((self.pos.clamp(0.0, 1.0) * 65_535.0) as u64) << 24
            | ((self.length * 100.0).clamp(0.0, 65_535.0) as u64) << 40
    }

    pub fn unpack(v: u64) -> Self {
        let rec = ((v >> 16) & 0xF) as usize;
        Self {
            used: (v & 0xFF) as u8,
            muted: ((v >> 8) & 0xFF) as u8,
            recording: rec.checked_sub(1),
            playing: (v >> 20) & 1 != 0,
            pos: ((v >> 24) & 0xFFFF) as f32 / 65_535.0,
            length: ((v >> 40) & 0xFFFF) as f32 / 100.0,
        }
    }
}

pub struct Looper {
    sample_rate: f32,
    /// Audio estéreo intercalado de cada pista.
    tracks: Vec<Vec<f32>>,
    capacity: usize,
    used: [bool; TRACKS],
    volume: [f32; TRACKS],
    muted: [bool; TRACKS],
    /// Frames de la vuelta (0 = aún no hay).
    length: usize,
    pos: usize,
    playing: bool,
    rec: Option<Rec>,
}

impl Looper {
    pub fn new(sample_rate: f32) -> Self {
        let capacity = (MAX_SECONDS * sample_rate) as usize;
        Self {
            sample_rate,
            // `vec![0.0; n]` pide memoria a cero al sistema: no se toca hasta que se usa.
            tracks: (0..TRACKS).map(|_| vec![0.0; capacity * 2]).collect(),
            capacity,
            used: [false; TRACKS],
            volume: [1.0; TRACKS],
            muted: [false; TRACKS],
            length: 0,
            pos: 0,
            playing: false,
            rec: None,
        }
    }

    pub fn status(&self) -> LooperStatus {
        let mut used = 0u8;
        let mut muted = 0u8;
        for t in 0..TRACKS {
            used |= (self.used[t] as u8) << t;
            muted |= (self.muted[t] as u8) << t;
        }
        let pos = if self.length > 0 {
            self.pos as f32 / self.length as f32
        } else if let Some(r) = self.rec {
            r.done as f32 / self.capacity as f32
        } else {
            0.0
        };
        LooperStatus {
            used,
            recording: self.rec.map(|r| r.track),
            playing: self.playing,
            pos,
            length: self.length as f32 / self.sample_rate,
            muted,
        }
    }

    /// Empieza a grabar la pista, o para si ya se estaba grabando.
    /// `bpm`: si hay ritmo sonando, la primera vuelta se redondea a compases.
    pub fn toggle_record(&mut self, track: usize, bpm: Option<f32>) {
        let track = track % TRACKS;
        match self.rec {
            Some(_) => self.stop_record(bpm),
            None => {
                self.rec = Some(Rec { track, done: 0 });
                self.used[track] = true;
                if self.length == 0 {
                    self.pos = 0;
                }
                self.playing = true;
            }
        }
    }

    fn stop_record(&mut self, bpm: Option<f32>) {
        let Some(rec) = self.rec.take() else { return };
        if self.length == 0 {
            // Primera grabación: marca la vuelta.
            if rec.done < (self.sample_rate * 0.25) as usize {
                // Demasiado corta: se descarta.
                self.used[rec.track] = false;
                self.clear_track(rec.track);
                self.playing = false;
                return;
            }
            let mut len = rec.done;
            if let Some(bpm) = bpm {
                let bar = (4.0 * 60.0 / bpm.max(1.0) * self.sample_rate) as usize;
                let bars = ((rec.done as f32 / bar as f32).round() as usize).max(1);
                len = (bars * bar).min(self.capacity);
            }
            self.length = len;
            self.pos = rec.done % len;
        }
    }

    fn clear_track(&mut self, t: usize) {
        self.tracks[t].iter_mut().for_each(|s| *s = 0.0);
    }

    /// Borra una pista. Si no queda ninguna, la vuelta vuelve a estar libre.
    pub fn clear(&mut self, track: usize) {
        let t = track % TRACKS;
        if self.rec.map(|r| r.track) == Some(t) {
            self.rec = None;
        }
        self.used[t] = false;
        self.clear_track(t);
        if !self.used.iter().any(|&u| u) {
            self.length = 0;
            self.pos = 0;
            self.playing = false;
        }
    }

    pub fn set_playing(&mut self, on: bool) {
        if !on {
            self.rec = None;
        }
        self.playing = on && self.length > 0;
    }

    pub fn set_volume(&mut self, track: usize, v: f32) {
        self.volume[track % TRACKS] = v.clamp(0.0, 1.0);
    }

    pub fn set_muted(&mut self, track: usize, m: bool) {
        self.muted[track % TRACKS] = m;
    }

    /// Graba la entrada `dry` (estéreo) y le suma las pistas que suenan.
    pub fn process(&mut self, io: &mut [f32], bpm: Option<f32>) {
        if !self.playing && self.rec.is_none() {
            return;
        }
        for frame in io.chunks_exact_mut(2) {
            // 1) Grabar.
            if let Some(rec) = &mut self.rec {
                let idx = if self.length == 0 { rec.done } else { self.pos };
                let buf = &mut self.tracks[rec.track];
                buf[idx * 2] = frame[0];
                buf[idx * 2 + 1] = frame[1];
                rec.done += 1;
            }
            // 2) Reproducir las demás pistas.
            if self.length > 0 && self.playing {
                let recording = self.rec.map(|r| r.track);
                for t in 0..TRACKS {
                    if !self.used[t] || self.muted[t] || recording == Some(t) {
                        continue;
                    }
                    let g = self.volume[t];
                    frame[0] += self.tracks[t][self.pos * 2] * g;
                    frame[1] += self.tracks[t][self.pos * 2 + 1] * g;
                }
                self.pos = (self.pos + 1) % self.length;
            }
            // 3) ¿Fin de la grabación?
            if let Some(rec) = self.rec {
                let limit = if self.length == 0 {
                    self.capacity
                } else {
                    self.length
                };
                if rec.done >= limit {
                    self.stop_record(bpm);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 1000.0; // frecuencia baja para que los tests sean rápidos

    fn feed(l: &mut Looper, frames: usize, value: f32, bpm: Option<f32>) -> Vec<f32> {
        let mut buf = vec![value; frames * 2];
        l.process(&mut buf, bpm);
        buf
    }

    #[test]
    fn primera_grabacion_marca_la_vuelta() {
        let mut l = Looper::new(SR);
        l.toggle_record(0, None);
        feed(&mut l, 1500, 0.5, None);
        l.toggle_record(0, None);
        let st = l.status();
        assert!((st.length - 1.5).abs() < 1e-3);
        assert!(st.has(0) && st.playing && st.recording.is_none());
        // Ahora suena lo grabado encima de la entrada.
        let out = feed(&mut l, 10, 0.0, None);
        assert!(out.iter().all(|&s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn con_ritmo_se_redondea_a_compases() {
        let mut l = Looper::new(SR);
        // 120 BPM: un compás = 2 s = 2000 frames. 3,7 s → 2 compases = 4 s.
        l.toggle_record(0, Some(120.0));
        feed(&mut l, 3700, 0.1, Some(120.0));
        l.toggle_record(0, Some(120.0));
        assert!((l.status().length - 4.0).abs() < 1e-3);
        assert_eq!(l.pos, 3700);
    }

    #[test]
    fn segunda_pista_graba_una_vuelta_y_para_sola() {
        let mut l = Looper::new(SR);
        l.toggle_record(0, None);
        feed(&mut l, 1000, 0.25, None);
        l.toggle_record(0, None);
        l.toggle_record(1, None);
        feed(&mut l, 999, 0.5, None);
        assert_eq!(l.status().recording, Some(1));
        feed(&mut l, 1, 0.5, None);
        assert_eq!(l.status().recording, None);
        // Suenan las dos.
        let out = feed(&mut l, 10, 0.0, None);
        assert!(out.iter().all(|&s| (s - 0.75).abs() < 1e-6));
    }

    #[test]
    fn grabacion_cortisima_se_descarta() {
        let mut l = Looper::new(SR);
        l.toggle_record(0, None);
        feed(&mut l, 10, 0.5, None);
        l.toggle_record(0, None);
        assert!(l.status().is_empty());
        assert_eq!(l.status().length, 0.0);
    }

    #[test]
    fn borrar_y_silenciar() {
        let mut l = Looper::new(SR);
        l.toggle_record(0, None);
        feed(&mut l, 1000, 0.25, None);
        l.toggle_record(0, None);
        l.set_muted(0, true);
        assert!(feed(&mut l, 10, 0.0, None).iter().all(|&s| s == 0.0));
        l.clear(0);
        let st = l.status();
        assert!(st.is_empty() && !st.playing && st.length == 0.0);
    }

    #[test]
    fn llegar_al_maximo_para_la_grabacion() {
        let mut l = Looper::new(SR);
        l.toggle_record(0, None);
        feed(&mut l, (MAX_SECONDS * SR) as usize + 100, 0.1, None);
        let st = l.status();
        assert_eq!(st.recording, None);
        assert!((st.length - MAX_SECONDS).abs() < 1e-3);
    }

    #[test]
    fn empaquetar_estado() {
        let st = LooperStatus {
            used: 0b101,
            recording: Some(2),
            playing: true,
            pos: 0.5,
            length: 8.25,
            muted: 0b100,
        };
        let back = LooperStatus::unpack(st.pack());
        assert_eq!(back.used, st.used);
        assert_eq!(back.recording, st.recording);
        assert_eq!(back.muted, st.muted);
        assert!(back.playing);
        assert!((back.pos - 0.5).abs() < 1e-3);
        assert!((back.length - 8.25).abs() < 1e-3);
    }
}
