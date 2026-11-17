//! Filtro de variable de estado (SVF, topología TPT de Andrew Simper).

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

/// Salidas del filtro.
#[derive(Debug, Clone, Copy)]
pub struct SvfOut {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

impl Svf {
    /// Ajusta frecuencia de corte (Hz) y resonancia (0 = nada, 1 = casi autooscila).
    #[inline]
    pub fn set(&mut self, cutoff: f32, resonance: f32, sample_rate: f32) {
        let fc = cutoff.clamp(20.0, sample_rate * 0.45);
        let g = (PI * fc / sample_rate).tan();
        // Q de 0,5 a ~20.
        let q = 0.5 + resonance.clamp(0.0, 1.0).powi(2) * 19.5;
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> SvfOut {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        SvfOut {
            low: v2,
            band: v1,
            high: x - self.k * v1 - v2,
        }
    }

    #[inline]
    pub fn lowpass(&mut self, x: f32) -> f32 {
        self.process(x).low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oscillator::{Oscillator, Waveform};

    fn rms_after(freq: f32, cutoff: f32) -> f32 {
        let sr = 48_000.0;
        let mut f = Svf::default();
        f.set(cutoff, 0.0, sr);
        let mut o = Oscillator::default();
        let n = 48_000;
        let mut acc = 0.0;
        for i in 0..n {
            let y = f.lowpass(o.next(Waveform::Sine, freq / sr));
            if i > n / 2 {
                acc += y * y;
            }
        }
        (acc / (n / 2) as f32).sqrt()
    }

    #[test]
    fn deja_pasar_graves_y_corta_agudos() {
        let low = rms_after(100.0, 1_000.0);
        let high = rms_after(10_000.0, 1_000.0);
        assert!(low > 0.6, "{low}");
        assert!(high < 0.02, "{high}");
    }

    #[test]
    fn estable_con_resonancia_maxima() {
        let sr = 48_000.0;
        let mut f = Svf::default();
        let mut o = Oscillator::default();
        for i in 0..96_000 {
            // Barrido rápido del corte.
            let fc = 50.0 + (i % 4800) as f32 * 4.0;
            f.set(fc, 1.0, sr);
            let y = f.lowpass(o.next(Waveform::Saw, 110.0 / sr));
            assert!(y.is_finite() && y.abs() < 50.0, "{y}");
        }
    }
}
