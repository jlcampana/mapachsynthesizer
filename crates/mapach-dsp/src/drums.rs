//! Batería sintetizada (sin samples): bombo, caja, charles, tom, palmas y platillo.
//!
//! Cada instrumento es una voz monofónica que se vuelve a disparar. El ruido sale de un
//! generador pseudoaleatorio con semilla fija, así que el resultado es determinista.

use std::f32::consts::TAU;

use crate::filter::Svf;

/// Los 7 instrumentos, uno por tecla de acorde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Drum {
    Bombo,
    Caja,
    CharlesCerrado,
    CharlesAbierto,
    Tom,
    Palmas,
    Platillo,
}

impl Drum {
    pub const ALL: [Drum; 7] = [
        Drum::Bombo,
        Drum::Caja,
        Drum::CharlesCerrado,
        Drum::CharlesAbierto,
        Drum::Tom,
        Drum::Palmas,
        Drum::Platillo,
    ];

    pub fn index(self) -> usize {
        Drum::ALL.iter().position(|&d| d == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        match self {
            Drum::Bombo => "Bombo",
            Drum::Caja => "Caja",
            Drum::CharlesCerrado => "Charles",
            Drum::CharlesAbierto => "Charles abierto",
            Drum::Tom => "Tom",
            Drum::Palmas => "Palmas",
            Drum::Platillo => "Platillo",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Drum::Bombo => "🥁",
            Drum::Caja => "🪘",
            Drum::CharlesCerrado => "✴",
            Drum::CharlesAbierto => "✳",
            Drum::Tom => "🛢",
            Drum::Palmas => "👏",
            Drum::Platillo => "🔔",
        }
    }
}

/// Carácter de un kit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kit {
    pub name: &'static str,
    pub icon: &'static str,
    /// Multiplicador de afinación.
    pub pitch: f32,
    /// Multiplicador de duración.
    pub decay: f32,
    /// Bits de resolución (0 = sin reducir). Para el kit 8-bit.
    pub bits: u8,
    /// Cuánto tono y cuánto ruido: 0 = normal, 1 = todo tono (juguetes).
    pub tonal: f32,
}

pub const KITS: [Kit; 4] = [
    Kit {
        name: "Electrónica",
        icon: "🎛",
        pitch: 1.0,
        decay: 1.0,
        bits: 0,
        tonal: 0.0,
    },
    Kit {
        name: "Acústica",
        icon: "🥁",
        pitch: 1.15,
        decay: 0.8,
        bits: 0,
        tonal: 0.1,
    },
    Kit {
        name: "8-bit",
        icon: "👾",
        pitch: 1.3,
        decay: 0.7,
        bits: 4,
        tonal: 0.3,
    },
    Kit {
        name: "Juguetes",
        icon: "🧸",
        pitch: 2.2,
        decay: 0.6,
        bits: 0,
        tonal: 0.8,
    },
];

/// Generador de ruido blanco determinista (xorshift).
#[derive(Debug, Clone, Copy)]
pub struct Noise(u32);

impl Default for Noise {
    fn default() -> Self {
        Noise::new(1)
    }
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Una voz de batería.
#[derive(Debug, Clone, Copy)]
pub struct DrumVoice {
    drum: Drum,
    /// Muestras desde el golpe.
    t: f32,
    active: bool,
    velocity: f32,
    phase: f32,
    phases: [f32; 6],
    noise: Noise,
    filter: Svf,
    /// Para reducir la frecuencia de muestreo en el kit 8-bit.
    hold: f32,
    hold_count: u32,
}

impl DrumVoice {
    pub fn new(drum: Drum) -> Self {
        Self {
            drum,
            t: 0.0,
            active: false,
            velocity: 1.0,
            phase: 0.0,
            phases: [0.0; 6],
            noise: Noise::new(0x1234_5678 ^ (drum.index() as u32 * 7919)),
            filter: Svf::default(),
            hold: 0.0,
            hold_count: 0,
        }
    }

    pub fn trigger(&mut self, velocity: f32) {
        self.t = 0.0;
        self.active = true;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.phase = 0.0;
        self.phases = [0.0; 6];
        self.filter.reset();
    }

    /// Corta en seco (el charles cerrado corta al abierto).
    pub fn choke(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Siguiente muestra (mono).
    #[inline]
    pub fn sample(&mut self, kit: &Kit, sr: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let secs = self.t / sr;
        self.t += 1.0;
        let dk = kit.decay;
        let p = kit.pitch;
        let tonal = kit.tonal;
        let env = |tau: f32| (-secs / (tau * dk)).exp();
        let tone = |freq: f32, phase: &mut f32| {
            *phase += freq / sr;
            if *phase >= 1.0 {
                *phase -= 1.0;
            }
            (*phase * TAU).sin()
        };
        let (out, length) = match self.drum {
            Drum::Bombo => {
                let f = 48.0 * p + 140.0 * p * (-secs / 0.035).exp();
                let body = tone(f, &mut self.phase) * env(0.28);
                let click = self.noise.sample() * (-secs / 0.004).exp() * 0.3 * (1.0 - tonal);
                (body * 1.1 + click, 1.2)
            }
            Drum::Caja => {
                let body = tone(185.0 * p, &mut self.phase) * env(0.08) * (0.5 + tonal * 0.5);
                self.filter.set(1_800.0 * p, 0.1, sr);
                let n = self.noise.sample();
                let hp = self.filter.process(n).high;
                (body + hp * env(0.13) * 0.8 * (1.0 - tonal * 0.7), 0.6)
            }
            Drum::CharlesCerrado | Drum::CharlesAbierto => {
                let tau = if self.drum == Drum::CharlesCerrado {
                    0.035
                } else {
                    0.28
                };
                self.filter.set(7_500.0, 0.2, sr);
                let n = if tonal > 0.5 {
                    // Juguetes: un "tic" metálico agudo.
                    tone(5_200.0 * p.min(1.5), &mut self.phase).signum()
                } else {
                    self.noise.sample()
                };
                let hp = self.filter.process(n).high;
                (hp * env(tau) * 0.5, tau * 8.0)
            }
            Drum::Tom => {
                let f = 110.0 * p + 60.0 * p * (-secs / 0.08).exp();
                (tone(f, &mut self.phase) * env(0.22), 1.0)
            }
            Drum::Palmas => {
                // Tres golpes muy seguidos y una cola.
                let burst = [0.0, 0.011, 0.022]
                    .iter()
                    .map(|&o| {
                        if secs >= o {
                            (-(secs - o) / 0.006).exp()
                        } else {
                            0.0
                        }
                    })
                    .sum::<f32>()
                    + 0.6 * env(0.12) * (secs > 0.03) as u8 as f32;
                self.filter.set(1_300.0 * p, 0.45, sr);
                let n = self.noise.sample();
                (self.filter.process(n).band * burst * 1.4, 0.6)
            }
            Drum::Platillo => {
                // Seis cuadradas en proporciones inarmónicas (como una 808), filtradas.
                const RATIOS: [f32; 6] = [1.0, 1.342, 1.2312, 1.6532, 1.9523, 2.1523];
                let base = 400.0 * p;
                let mut m = 0.0;
                for (ph, r) in self.phases.iter_mut().zip(RATIOS) {
                    *ph += base * r / sr;
                    if *ph >= 1.0 {
                        *ph -= 1.0;
                    }
                    m += if *ph < 0.5 { 1.0 } else { -1.0 };
                }
                self.filter.set(6_000.0, 0.3, sr);
                (self.filter.process(m / 6.0).high * env(0.6) * 0.6, 4.0)
            }
        };
        if secs > length * dk {
            self.active = false;
        }
        let mut y = out * self.velocity;
        if kit.bits > 0 {
            // 8-bit: menos resolución y menos frecuencia de muestreo.
            if self.hold_count == 0 {
                let q = (1u32 << kit.bits) as f32;
                self.hold = (y * q).round() / q;
            }
            self.hold_count = (self.hold_count + 1) % 4;
            y = self.hold;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn render(drum: Drum, kit: &Kit, secs: f32) -> Vec<f32> {
        let mut v = DrumVoice::new(drum);
        v.trigger(1.0);
        (0..(secs * SR) as usize)
            .map(|_| v.sample(kit, SR))
            .collect()
    }

    #[test]
    fn todos_suenan_y_se_apagan() {
        for kit in &KITS {
            for d in Drum::ALL {
                let s = render(d, kit, 6.0);
                let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(peak > 0.05, "{} {:?}: {peak}", kit.name, d);
                assert!(peak < 2.0, "{} {:?}: {peak}", kit.name, d);
                assert!(s.iter().all(|x| x.is_finite()));
                assert!(
                    s[s.len() - 100..].iter().all(|&x| x == 0.0),
                    "{} {:?}",
                    kit.name,
                    d
                );
            }
        }
    }

    #[test]
    fn deterministas() {
        let a = render(Drum::Caja, &KITS[0], 0.2);
        let b = render(Drum::Caja, &KITS[0], 0.2);
        assert_eq!(a, b);
    }

    #[test]
    fn ruido_centrado() {
        let mut n = Noise::new(1);
        let mean: f32 = (0..100_000).map(|_| n.sample()).sum::<f32>() / 100_000.0;
        assert!(mean.abs() < 0.02);
    }
}
