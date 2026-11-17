//! Motor de audio de Mapachsynthesizer.
//!
//! El [`Engine`] es puro y determinista: recibe [`Command`]s y rellena buffers estéreo
//! intercalados. No sabe nada de `cpal`, así que se puede probar y renderizar sin tarjeta de
//! sonido. El módulo [`audio`] lo conecta a la salida real.

pub mod audio;

use std::f32::consts::TAU;

/// Órdenes que la interfaz manda al motor por el ring buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Empieza a sonar una nota MIDI.
    NoteOn { note: u8 },
    /// Suelta una nota MIDI.
    NoteOff { note: u8 },
}

/// Número máximo de voces simultáneas.
pub const MAX_VOICES: usize = 16;

#[derive(Debug, Clone, Copy)]
struct Voice {
    note: u8,
    phase: f32,
    freq: f32,
    gain: f32,
    target: f32,
    active: bool,
}

impl Voice {
    const IDLE: Voice = Voice {
        note: 0,
        phase: 0.0,
        freq: 0.0,
        gain: 0.0,
        target: 0.0,
        active: false,
    };
}

/// Convierte una nota MIDI en frecuencia (La4 = 69 = 440 Hz).
pub fn midi_to_freq(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// El motor de síntesis.
pub struct Engine {
    sample_rate: f32,
    voices: [Voice; MAX_VOICES],
    /// Cuánto se acerca la ganancia a su objetivo en cada muestra (evita clics).
    ramp: f32,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            voices: [Voice::IDLE; MAX_VOICES],
            // Unos 5 ms para subir o bajar.
            ramp: 1.0 / (0.005 * sample_rate),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Aplica una orden. No reserva memoria.
    pub fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::NoteOn { note } => {
                let slot = self
                    .voices
                    .iter()
                    .position(|v| v.active && v.note == note)
                    .or_else(|| self.voices.iter().position(|v| !v.active))
                    .unwrap_or(0);
                let v = &mut self.voices[slot];
                if !(v.active && v.note == note) {
                    v.phase = 0.0;
                    v.gain = 0.0;
                }
                v.note = note;
                v.freq = midi_to_freq(note as f32);
                v.target = 1.0;
                v.active = true;
            }
            Command::NoteOff { note } => {
                for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
                    v.target = 0.0;
                }
            }
        }
    }

    /// Número de voces que están sonando.
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Rellena un buffer estéreo intercalado (`[L, R, L, R, …]`).
    pub fn process(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let dt = 1.0 / self.sample_rate;
        for v in self.voices.iter_mut().filter(|v| v.active) {
            for frame in out.chunks_exact_mut(2) {
                if v.gain < v.target {
                    v.gain = (v.gain + self.ramp).min(v.target);
                } else if v.gain > v.target {
                    v.gain = (v.gain - self.ramp).max(v.target);
                }
                let s = (v.phase * TAU).sin() * v.gain * 0.2;
                v.phase += v.freq * dt;
                if v.phase >= 1.0 {
                    v.phase -= 1.0;
                }
                frame[0] += s;
                frame[1] += s;
            }
            if v.target == 0.0 && v.gain == 0.0 {
                v.active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_es_440() {
        assert!((midi_to_freq(69.0) - 440.0).abs() < 1e-3);
        assert!((midi_to_freq(81.0) - 880.0).abs() < 1e-2);
    }

    #[test]
    fn silencio_sin_notas() {
        let mut e = Engine::new(48_000.0);
        let mut buf = vec![1.0; 512];
        e.process(&mut buf);
        assert!(buf.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn nota_suena_y_se_apaga() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::NoteOn { note: 69 });
        let mut buf = vec![0.0; 2 * 4800];
        e.process(&mut buf);
        let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.1, "pico {peak}");
        assert!(buf.iter().all(|s| s.is_finite()));

        e.handle(Command::NoteOff { note: 69 });
        e.process(&mut buf);
        e.process(&mut buf);
        assert_eq!(e.active_voices(), 0);
        assert!(buf.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn sin_clic_al_empezar() {
        let mut e = Engine::new(48_000.0);
        e.handle(Command::NoteOn { note: 69 });
        let mut buf = vec![0.0; 2 * 64];
        e.process(&mut buf);
        // La primera muestra tiene que ser casi cero (rampa de entrada).
        assert!(buf[0].abs() < 0.01);
    }
}
