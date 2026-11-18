//! Una voz del sintetizador: genera según el motor del sonido ([`Kind`]), y pasa por filtro y ADSR.

use std::f32::consts::TAU;

use mapach_dsp::drums::Noise;
use mapach_dsp::{Adsr, AdsrParams, Oscillator, Svf, Waveform, pan_gains};

use crate::midi_to_freq;
use crate::patch::{Kind, Patch};

/// Formantes (Hz) de las vocales a, e, i, o, u.
const VOWELS: [(f32, f32); 5] = [
    (800.0, 1150.0),
    (400.0, 1700.0),
    (300.0, 2300.0),
    (450.0, 800.0),
    (325.0, 700.0),
];

const HARMONICS: [f32; 6] = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Voice {
    pub note: u8,
    pub started: u64,
    velocity: f32,
    pan_l: f32,
    pan_r: f32,
    osc1: Oscillator,
    osc2: Oscillator,
    filter: Svf,
    /// Segundo filtro (formantes, ruido afinado).
    filter2: Svf,
    env: Adsr,
    /// La tecla sigue pulsada.
    held: bool,
    /// Muestras desde que empezó la nota (para la envolvente del índice FM).
    age: f32,
    lfo: f32,
    noise: Noise,
}

impl Voice {
    pub fn start(&mut self, note: u8, velocity: f32, pan: f32, clock: u64, patch: &Patch) {
        let restart = !self.env.is_active() || self.note != note;
        self.note = note;
        self.started = clock;
        self.velocity = velocity;
        let (l, r) = pan_gains(pan * patch.stereo);
        self.pan_l = l;
        self.pan_r = r;
        if restart {
            // Fases distintas por nota para que los acordes no empiecen todos con el mismo golpe,
            // pero deterministas.
            let p = (note as f32 * 0.137).fract();
            self.osc1.reset(p);
            self.osc2.reset((p + 0.5).fract());
            self.filter.reset();
            self.filter2.reset();
            self.noise = Noise::new(0x51AF_0000 ^ (note as u32).wrapping_mul(2_654_435_761));
            self.lfo = p;
        }
        self.age = 0.0;
        self.held = true;
        self.env.gate_on();
    }

    pub fn release(&mut self) {
        self.held = false;
        self.env.gate_off();
    }

    pub fn is_held(&self, note: u8) -> bool {
        self.held && self.note == note && self.env.is_active()
    }

    pub fn is_active(&self) -> bool {
        self.env.is_active()
    }

    pub fn is_released(&self) -> bool {
        self.env.is_released()
    }

    pub fn level(&self) -> f32 {
        self.env.level()
    }

    /// Suma la voz a un bloque estéreo intercalado.
    pub fn render(
        &mut self,
        out: &mut [f32],
        patch: &Patch,
        env: &AdsrParams,
        cutoff: f32,
        sample_rate: f32,
    ) {
        let base = self.note as f32;
        let f1 = midi_to_freq(base + patch.osc1_semitones);
        let f2 = midi_to_freq(base + patch.osc2_semitones + patch.detune_cents / 100.0);
        let dt1 = f1 / sample_rate;
        let dt2 = f2 / sample_rate;
        // El corte sigue un poco a la nota, para que las agudas no se apaguen.
        let key_track = 2f32.powf((base - 60.0) / 24.0);
        let gain = 0.22 * (0.4 + 0.6 * self.velocity);
        let m2 = patch.osc2_mix;
        let m1 = 1.0 - m2 * 0.5;
        let lfo_dt = 5.5 / sample_rate;
        // El vibrato multiplica la frecuencia: 2^(semitonos/12) ≈ 1 + 0,0578·semitonos.
        let vib_depth = patch.vibrato * 0.0578;
        let inv_sr = 1.0 / sample_rate;
        // Ajustes que no cambian dentro del bloque.
        match patch.kind {
            Kind::Formant { vowel } => {
                let (fa, fb) = VOWELS[vowel as usize % VOWELS.len()];
                self.filter.set(fa, 0.75, sample_rate);
                self.filter2.set(fb, 0.75, sample_rate);
            }
            Kind::Noise { focus, .. } => {
                self.filter2.set(f1, focus, sample_rate);
            }
            _ => {}
        }
        for frame in out.chunks_exact_mut(2) {
            let e = self.env.next(env, sample_rate);
            self.lfo += lfo_dt;
            if self.lfo >= 1.0 {
                self.lfo -= 1.0;
            }
            let vib = 1.0 + vib_depth * (self.lfo * TAU).sin();
            let secs = self.age * inv_sr;
            self.age += 1.0;
            let raw = match patch.kind {
                Kind::Analog => {
                    self.osc1.next(patch.osc1, dt1 * vib) * m1
                        + self.osc2.next(patch.osc2, dt2 * vib) * m2
                }
                Kind::Fm {
                    ratio,
                    index,
                    index_decay,
                    index_sustain,
                } => {
                    let ienv = index_sustain
                        + (1.0 - index_sustain) * (-secs / index_decay.max(0.001)).exp();
                    let i = index * ienv * (0.5 + 0.5 * self.velocity);
                    let m = self.osc2.next(Waveform::Sine, dt1 * ratio * vib);
                    // Portadora: fase propia más la modulación.
                    self.osc1.phase += dt1 * vib;
                    if self.osc1.phase >= 1.0 {
                        self.osc1.phase -= 1.0;
                    }
                    (self.osc1.phase * TAU + i * m).sin()
                }
                Kind::Additive { drawbars } => {
                    self.osc1.phase += dt1 * vib;
                    if self.osc1.phase >= 1.0 {
                        self.osc1.phase -= 1.0;
                    }
                    let ph = self.osc1.phase * TAU;
                    let mut sum = 0.0;
                    for (h, &g) in HARMONICS.iter().zip(&drawbars) {
                        // Sin armónicos por encima de Nyquist.
                        if g > 0.0 && f1 * h < sample_rate * 0.45 {
                            sum += (ph * h).sin() * g;
                        }
                    }
                    sum * 0.45
                }
                Kind::Noise { lfo, focus } => {
                    let focus_gain = 1.0 + (1.0 - focus) * 6.0;
                    // Ondulación lenta (olas) o rápida (nave).
                    let wobble = 0.65 + 0.35 * (secs * lfo * TAU).sin();
                    let n = self.noise.sample();
                    // Cuanto más ancho el filtro, más energía: lo compensamos.
                    self.filter2.process(n).band * 2.5 * wobble * (0.4 + 0.6 * focus_gain)
                }
                Kind::Formant { .. } => {
                    let saw = self.osc1.next(Waveform::Saw, dt1 * vib)
                        + self.osc2.next(Waveform::Saw, dt1 * vib * 1.004);
                    let a = self.filter.process(saw).band;
                    let b = self.filter2.process(saw).band;
                    (a + b * 0.6) * 0.35
                }
            };
            let s = match patch.kind {
                // Los formantes ya están filtrados; los demás pasan por el filtro de brillo.
                Kind::Formant { .. } => raw * e * gain,
                _ => {
                    let fc = cutoff * key_track * (1.0 + patch.env_to_filter * e);
                    self.filter.set(fc, patch.resonance, sample_rate);
                    self.filter.lowpass(raw) * e * gain
                }
            };
            frame[0] += s * self.pan_l;
            frame[1] += s * self.pan_r;
            if !self.env.is_active() {
                break;
            }
        }
    }
}
