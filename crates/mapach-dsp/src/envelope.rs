//! Envolvente ADSR.

/// Tiempos en segundos y sustain de 0 a 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrParams {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl AdsrParams {
    pub const fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
        }
    }

    /// Las 7 envolventes de fm1-chord (LONG, SHORT, SWELL, PLUCK, TOUCH, SUSTAIN, KEYS).
    pub const LONG: AdsrParams = AdsrParams::new(0.8, 1.0, 0.7, 2.0);
    pub const SHORT: AdsrParams = AdsrParams::new(0.1, 0.1, 1.0, 1.0);
    pub const SWELL: AdsrParams = AdsrParams::new(0.8, 0.3, 0.8, 2.0);
    pub const PLUCK: AdsrParams = AdsrParams::new(0.005, 0.08, 0.0, 0.18);
    pub const TOUCH: AdsrParams = AdsrParams::new(0.05, 0.26, 0.66, 0.45);
    pub const SUSTAIN: AdsrParams = AdsrParams::new(0.2, 0.3, 0.85, 3.0);
    pub const KEYS: AdsrParams = AdsrParams::new(0.005, 1.2, 0.55, 0.7);
}

impl Default for AdsrParams {
    fn default() -> Self {
        AdsrParams::TOUCH
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stage {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Envolvente ADSR: ataque lineal, caída y relajación exponenciales.
#[derive(Debug, Clone, Copy, Default)]
pub struct Adsr {
    stage: Stage,
    level: f32,
    /// Nivel desde el que empezó la relajación.
    release_from: f32,
}

/// Duración mínima de cualquier fase, para evitar clics.
const MIN_TIME: f32 = 0.002;

impl Adsr {
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
            self.release_from = self.level;
        }
    }

    /// Corta en seco (para robar voces se usa una relajación muy corta en su lugar).
    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }

    pub fn is_released(&self) -> bool {
        matches!(self.stage, Stage::Release | Stage::Idle)
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    /// Avanza una muestra y devuelve el nivel (0–1).
    #[inline]
    pub fn next(&mut self, p: &AdsrParams, sample_rate: f32) -> f32 {
        let dt = 1.0 / sample_rate;
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += dt / p.attack.max(MIN_TIME);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let s = p.sustain.clamp(0.0, 1.0);
                // Exponencial: llega al ~99 % del camino en el tiempo de decay.
                let k = 4.6 * dt / p.decay.max(MIN_TIME);
                self.level += (s - self.level) * k.min(1.0);
                if (self.level - s).abs() < 1e-4 {
                    self.level = s;
                    self.stage = if s <= 0.0 { Stage::Idle } else { Stage::Sustain };
                }
            }
            Stage::Sustain => {
                // Sigue al sustain si cambia mientras suena.
                let s = p.sustain.clamp(0.0, 1.0);
                self.level += (s - self.level) * 0.001;
            }
            Stage::Release => {
                let k = 4.6 * dt / p.release.max(MIN_TIME);
                self.level -= self.level * k.min(1.0);
                // Además, un descenso lineal pequeño para que termine en un tiempo finito.
                self.level -= self.release_from.max(0.05) * dt / (p.release.max(MIN_TIME) * 4.0);
                if self.level <= 1e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn run(env: &mut Adsr, p: &AdsrParams, secs: f32) -> f32 {
        let mut last = 0.0;
        for _ in 0..(secs * SR) as usize {
            last = env.next(p, SR);
        }
        last
    }

    #[test]
    fn ciclo_completo() {
        let p = AdsrParams::new(0.01, 0.1, 0.5, 0.2);
        let mut e = Adsr::default();
        assert_eq!(e.next(&p, SR), 0.0);
        e.gate_on();
        let l = run(&mut e, &p, 0.005);
        assert!(l > 0.4 && l < 0.6, "{l}");
        let l = run(&mut e, &p, 0.5);
        assert!((l - 0.5).abs() < 0.01, "{l}");
        e.gate_off();
        run(&mut e, &p, 1.0);
        assert!(!e.is_active());
        assert_eq!(e.level(), 0.0);
    }

    #[test]
    fn release_termina_en_tiempo_finito() {
        for p in [AdsrParams::LONG, AdsrParams::SUSTAIN, AdsrParams::KEYS] {
            let mut e = Adsr::default();
            e.gate_on();
            run(&mut e, &p, 2.0);
            e.gate_off();
            run(&mut e, &p, p.release * 1.5);
            assert!(!e.is_active(), "{p:?}");
        }
    }

    #[test]
    fn pluck_se_apaga_solo() {
        let p = AdsrParams::PLUCK;
        let mut e = Adsr::default();
        e.gate_on();
        run(&mut e, &p, 1.0);
        assert!(!e.is_active());
    }

    #[test]
    fn nunca_pasa_de_uno() {
        let p = AdsrParams::new(0.0, 0.0, 1.0, 0.0);
        let mut e = Adsr::default();
        e.gate_on();
        for _ in 0..1000 {
            let l = e.next(&p, SR);
            assert!((0.0..=1.0).contains(&l));
        }
    }
}
