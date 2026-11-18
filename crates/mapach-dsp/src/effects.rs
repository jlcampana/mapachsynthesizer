//! Efectos: sala (reverb), eco, coro, flanger, fase, distorsión, robot (bitcrush) y trémolo.
//!
//! Toda la memoria de los retardos se reserva al crearlos.

use std::f32::consts::TAU;

/// Efecto del hueco de efecto (la sala va aparte, siempre disponible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Effect {
    #[default]
    Ninguno,
    Eco,
    Coro,
    Flanger,
    Fase,
    Distorsion,
    Robot,
    Tremolo,
}

impl Effect {
    pub const ALL: [Effect; 8] = [
        Effect::Ninguno,
        Effect::Eco,
        Effect::Coro,
        Effect::Flanger,
        Effect::Fase,
        Effect::Distorsion,
        Effect::Robot,
        Effect::Tremolo,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Effect::Ninguno => "Ninguno",
            Effect::Eco => "Eco",
            Effect::Coro => "Coro",
            Effect::Flanger => "Avión",
            Effect::Fase => "Remolino",
            Effect::Distorsion => "Distorsión",
            Effect::Robot => "Robot",
            Effect::Tremolo => "Temblor",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Effect::Ninguno => "·",
            Effect::Eco => "🏔",
            Effect::Coro => "👥",
            Effect::Flanger => "✈",
            Effect::Fase => "🌀",
            Effect::Distorsion => "🔥",
            Effect::Robot => "🤖",
            Effect::Tremolo => "〰",
        }
    }
}

/// Línea de retardo con lectura interpolada.
#[derive(Debug, Clone)]
struct DelayLine {
    buf: Vec<f32>,
    pos: usize,
}

impl DelayLine {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(2)],
            pos: 0,
        }
    }

    fn write(&mut self, x: f32) {
        self.buf[self.pos] = x;
        self.pos = (self.pos + 1) % self.buf.len();
    }

    /// Lee `delay` muestras atrás (con decimales).
    fn read(&self, delay: f32) -> f32 {
        let len = self.buf.len();
        let d = delay.clamp(1.0, (len - 2) as f32);
        let i = d.floor() as usize;
        let frac = d - i as f32;
        let a = self.buf[(self.pos + len - i) % len];
        let b = self.buf[(self.pos + len - i - 1) % len];
        a + (b - a) * frac
    }

    fn clear(&mut self) {
        self.buf.iter_mut().for_each(|s| *s = 0.0);
    }
}

/// Filtro peine con amortiguación (Freeverb).
#[derive(Debug, Clone)]
struct Comb {
    buf: Vec<f32>,
    pos: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
            store: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let y = self.buf[self.pos];
        self.store = y * (1.0 - damp) + self.store * damp;
        self.buf[self.pos] = x + self.store * feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

#[derive(Debug, Clone)]
struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let b = self.buf[self.pos];
        self.buf[self.pos] = x + b * 0.5;
        self.pos = (self.pos + 1) % self.buf.len();
        b - x
    }
}

/// Reverb Freeverb (Jezar), estéreo.
#[derive(Debug, Clone)]
pub struct Reverb {
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
}

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let k = sample_rate / 44_100.0;
        let len = |n: usize, ch: usize| ((n + ch * STEREO_SPREAD) as f32 * k) as usize;
        Self {
            combs: [0, 1].map(|ch| COMB_TUNING.iter().map(|&n| Comb::new(len(n, ch))).collect()),
            allpasses: [0, 1].map(|ch| {
                ALLPASS_TUNING
                    .iter()
                    .map(|&n| Allpass::new(len(n, ch)))
                    .collect()
            }),
        }
    }

    /// Devuelve solo la parte húmeda. `size` 0–1.
    #[inline]
    pub fn process(&mut self, l: f32, r: f32, size: f32) -> (f32, f32) {
        let input = (l + r) * 0.015;
        let feedback = 0.7 + size.clamp(0.0, 1.0) * 0.28;
        let damp = 0.3;
        let mut out = [0.0f32; 2];
        for ch in 0..2 {
            let mut acc = 0.0;
            for c in self.combs[ch].iter_mut() {
                acc += c.process(input, feedback, damp);
            }
            for a in self.allpasses[ch].iter_mut() {
                acc = a.process(acc);
            }
            out[ch] = acc;
        }
        (out[0], out[1])
    }
}

/// La cadena de efectos: el efecto elegido y luego la sala.
#[derive(Debug, Clone)]
pub struct FxChain {
    sample_rate: f32,
    pub effect: Effect,
    /// Cantidad del efecto, 0–1.
    pub amount: f32,
    /// Cantidad de sala, 0–1.
    pub reverb: f32,
    delay: [DelayLine; 2],
    reverb_fx: Reverb,
    lfo: f32,
    /// Filtros pasa-todo del "remolino" (fase), por canal.
    phaser: [[f32; 4]; 2],
    phaser_fb: [f32; 2],
    crush_hold: [f32; 2],
    crush_count: u32,
}

impl FxChain {
    pub fn new(sample_rate: f32) -> Self {
        let max = (2.0 * sample_rate) as usize;
        Self {
            sample_rate,
            effect: Effect::Ninguno,
            amount: 0.5,
            reverb: 0.15,
            delay: [DelayLine::new(max), DelayLine::new(max)],
            reverb_fx: Reverb::new(sample_rate),
            lfo: 0.0,
            phaser: [[0.0; 4]; 2],
            phaser_fb: [0.0; 2],
            crush_hold: [0.0; 2],
            crush_count: 0,
        }
    }

    pub fn set_effect(&mut self, e: Effect) {
        if e != self.effect {
            self.effect = e;
            // Los retardos de un efecto no deben sonar en el siguiente.
            self.delay.iter_mut().for_each(DelayLine::clear);
            self.phaser = [[0.0; 4]; 2];
            self.phaser_fb = [0.0; 2];
        }
    }

    /// Procesa un bloque estéreo intercalado. `bpm` para el eco y el temblor.
    pub fn process(&mut self, io: &mut [f32], bpm: f32) {
        let sr = self.sample_rate;
        let amt = self.amount.clamp(0.0, 1.0);
        let beat = 60.0 / bpm.max(20.0) * sr;
        for frame in io.chunks_exact_mut(2) {
            let (mut l, mut r) = (frame[0], frame[1]);
            match self.effect {
                Effect::Ninguno => {}
                Effect::Eco => {
                    // Ping-pong a corchea con puntillo.
                    let d = beat * 0.75;
                    let fb = 0.25 + amt * 0.5;
                    let el = self.delay[0].read(d);
                    let er = self.delay[1].read(d);
                    self.delay[0].write(l + er * fb);
                    self.delay[1].write(r + el * fb);
                    l += el * (0.3 + amt * 0.5);
                    r += er * (0.3 + amt * 0.5);
                }
                Effect::Coro | Effect::Flanger => {
                    let (base, depth, rate, fb) = if self.effect == Effect::Coro {
                        (0.018, 0.006, 0.8, 0.0)
                    } else {
                        (0.003, 0.0025, 0.25, 0.55)
                    };
                    self.lfo = (self.lfo + rate / sr) % 1.0;
                    let ml = (self.lfo * TAU).sin();
                    let mr = (self.lfo * TAU + std::f32::consts::FRAC_PI_2).sin();
                    let dl = (base + depth * ml) * sr;
                    let dr = (base + depth * mr) * sr;
                    let wl = self.delay[0].read(dl);
                    let wr = self.delay[1].read(dr);
                    self.delay[0].write(l + wl * fb);
                    self.delay[1].write(r + wr * fb);
                    let w = 0.3 + amt * 0.7;
                    l = l * (1.0 - w * 0.4) + wl * w;
                    r = r * (1.0 - w * 0.4) + wr * w;
                }
                Effect::Fase => {
                    self.lfo = (self.lfo + 0.4 / sr) % 1.0;
                    let m = 0.5 + 0.5 * (self.lfo * TAU).sin();
                    // Frecuencia del pasa-todo entre 300 Hz y 3 kHz.
                    let f = 300.0 * 10f32.powf(m);
                    let t = (std::f32::consts::PI * f / sr).tan();
                    let a = (t - 1.0) / (t + 1.0);
                    let mix = 0.3 + amt * 0.5;
                    for (ch, x) in [&mut l, &mut r].into_iter().enumerate() {
                        let mut y = *x + self.phaser_fb[ch] * 0.5 * amt;
                        for st in self.phaser[ch].iter_mut() {
                            let out = a * y + *st;
                            *st = y - a * out;
                            y = out;
                        }
                        self.phaser_fb[ch] = y;
                        *x = *x * (1.0 - mix) + y * mix;
                    }
                }
                Effect::Distorsion => {
                    let drive = 1.0 + amt * 20.0;
                    // Compensación para que no suba mucho el volumen.
                    let comp = 1.0 / (1.0 + amt * 2.0).sqrt();
                    l = (l * drive).tanh() * comp;
                    r = (r * drive).tanh() * comp;
                }
                Effect::Robot => {
                    // Menos bits y menos frecuencia de muestreo.
                    let bits = 12.0 - amt * 9.0;
                    let q = 2f32.powf(bits);
                    let hold = 1 + (amt * 12.0) as u32;
                    if self.crush_count == 0 {
                        self.crush_hold = [(l * q).round() / q, (r * q).round() / q];
                    }
                    self.crush_count = (self.crush_count + 1) % hold;
                    l = self.crush_hold[0];
                    r = self.crush_hold[1];
                }
                Effect::Tremolo => {
                    // A semicorcheas.
                    self.lfo = (self.lfo + 4.0 / beat) % 1.0;
                    let g = 1.0 - amt * (0.5 + 0.5 * (self.lfo * TAU).sin());
                    l *= g;
                    r *= g;
                }
            }
            if self.reverb > 0.001 {
                let (wl, wr) = self.reverb_fx.process(l, r, 0.5 + self.reverb * 0.4);
                let w = self.reverb * 1.2;
                l += wl * w;
                r += wr * w;
            }
            frame[0] = l;
            frame[1] = r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn impulse_response(e: Effect, amount: f32, reverb: f32, secs: f32) -> Vec<f32> {
        let mut fx = FxChain::new(SR);
        fx.set_effect(e);
        fx.amount = amount;
        fx.reverb = reverb;
        let mut buf = vec![0.0; (secs * SR) as usize * 2];
        buf[0] = 0.5;
        buf[1] = 0.5;
        fx.process(&mut buf, 120.0);
        buf
    }

    #[test]
    fn todos_estables_y_finitos() {
        for e in Effect::ALL {
            for amt in [0.0, 0.5, 1.0] {
                let mut fx = FxChain::new(SR);
                fx.set_effect(e);
                fx.amount = amt;
                fx.reverb = 1.0;
                // Ruido fuerte durante 2 s.
                let mut n = crate::drums::Noise::new(7);
                let mut buf: Vec<f32> = (0..SR as usize * 4).map(|_| n.sample() * 0.8).collect();
                fx.process(&mut buf, 140.0);
                let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(buf.iter().all(|x| x.is_finite()), "{e:?}");
                assert!(peak < 8.0, "{e:?} {amt}: {peak}");
            }
        }
    }

    #[test]
    fn colas_que_se_apagan() {
        for e in [Effect::Eco, Effect::Flanger, Effect::Coro] {
            let buf = impulse_response(e, 1.0, 1.0, 20.0);
            let tail = &buf[buf.len() - 4800..];
            let peak = tail.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak < 1e-3, "{e:?}: {peak}");
        }
    }

    #[test]
    fn eco_repite_a_tempo() {
        let buf = impulse_response(Effect::Eco, 0.5, 0.0, 1.0);
        // 120 BPM: corchea con puntillo = 0,375 s = 18 000 muestras.
        let at = |frame: usize| buf[frame * 2].abs().max(buf[frame * 2 + 1].abs());
        assert!(at(18_001) > 0.05 || at(18_000) > 0.05);
        assert!(at(9_000) < 1e-6);
    }

    #[test]
    fn sin_efecto_ni_sala_no_cambia_nada() {
        let mut fx = FxChain::new(SR);
        fx.reverb = 0.0;
        let input: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut buf = input.clone();
        fx.process(&mut buf, 120.0);
        assert_eq!(buf, input);
    }

    #[test]
    fn la_sala_deja_cola() {
        let buf = impulse_response(Effect::Ninguno, 0.0, 0.8, 1.0);
        let mid = &buf[SR as usize / 2..SR as usize];
        assert!(mid.iter().any(|x| x.abs() > 1e-4));
    }
}
