//! Limitador final estéreo con lookahead: protege los oídos.
//!
//! Garantiza que ninguna muestra de salida pasa del techo, aunque se sumen muchas voces
//! con distorsión.

/// Lookahead en muestras (unos 1,3 ms a 48 kHz).
const LOOKAHEAD: usize = 64;

#[derive(Debug, Clone)]
pub struct Limiter {
    ceiling: f32,
    gain: f32,
    release_coef: f32,
    delay_l: [f32; LOOKAHEAD],
    delay_r: [f32; LOOKAHEAD],
    /// Ganancia necesaria para cada muestra del retardo.
    needed: [f32; LOOKAHEAD],
    pos: usize,
}

impl Limiter {
    /// `ceiling` es el pico máximo de salida (lineal). `release` en segundos.
    pub fn new(ceiling: f32, release: f32, sample_rate: f32) -> Self {
        Self {
            ceiling,
            gain: 1.0,
            release_coef: 1.0 - (-1.0 / (release * sample_rate)).exp(),
            delay_l: [0.0; LOOKAHEAD],
            delay_r: [0.0; LOOKAHEAD],
            needed: [1.0; LOOKAHEAD],
            pos: 0,
        }
    }

    pub fn set_ceiling(&mut self, ceiling: f32) {
        self.ceiling = ceiling.max(1e-4);
    }

    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    /// Reducción de ganancia actual (1 = nada).
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Procesa una muestra estéreo. Devuelve la muestra retrasada `LOOKAHEAD` muestras.
    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // Sanea NaN e infinitos antes de nada.
        let l = if l.is_finite() { l } else { 0.0 };
        let r = if r.is_finite() { r } else { 0.0 };
        let peak = l.abs().max(r.abs());
        let need = if peak > self.ceiling {
            self.ceiling / peak
        } else {
            1.0
        };

        let out_l = self.delay_l[self.pos];
        let out_r = self.delay_r[self.pos];
        self.delay_l[self.pos] = l;
        self.delay_r[self.pos] = r;
        self.needed[self.pos] = need;
        self.pos = (self.pos + 1) % LOOKAHEAD;

        // La ganancia objetivo es la mínima de toda la ventana: así baja antes de que llegue el pico.
        let target = self.needed.iter().copied().fold(1.0f32, f32::min);
        if target < self.gain {
            // Bajada suave pero que llega a tiempo (en la mitad de la ventana).
            let step = (self.gain - target) / (LOOKAHEAD as f32 * 0.5);
            self.gain = (self.gain - step.max(1e-6)).max(target);
        } else {
            self.gain += (target - self.gain) * self.release_coef;
        }
        // Red de seguridad: recorte duro en el techo.
        let c = self.ceiling;
        (
            (out_l * self.gain).clamp(-c, c),
            (out_r * self.gain).clamp(-c, c),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_to_gain;

    #[test]
    fn nunca_pasa_del_techo() {
        let c = db_to_gain(-12.0);
        let mut lim = Limiter::new(c, 0.1, 48_000.0);
        for i in 0..96_000 {
            let x = ((i as f32) * 0.05).sin() * 8.0 + if i % 1000 == 0 { 50.0 } else { 0.0 };
            let (l, r) = lim.process(x, -x);
            assert!(l.abs() <= c + 1e-6 && r.abs() <= c + 1e-6);
        }
    }

    #[test]
    fn transparente_por_debajo_del_techo() {
        let mut lim = Limiter::new(0.5, 0.1, 48_000.0);
        let input: Vec<f32> = (0..2000).map(|i| (i as f32 * 0.01).sin() * 0.3).collect();
        let out: Vec<f32> = input.iter().map(|&x| lim.process(x, x).0).collect();
        for i in 0..(2000 - LOOKAHEAD) {
            assert!((out[i + LOOKAHEAD] - input[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn nan_se_convierte_en_silencio() {
        let mut lim = Limiter::new(0.5, 0.1, 48_000.0);
        for _ in 0..200 {
            let (l, r) = lim.process(f32::NAN, f32::INFINITY);
            assert!(l.is_finite() && r.is_finite());
        }
    }
}
