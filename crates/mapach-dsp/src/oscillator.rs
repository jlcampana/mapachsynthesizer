//! Osciladores con antialiasing polyBLEP.

use std::f32::consts::TAU;

/// Forma de onda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Waveform {
    #[default]
    Sine,
    Triangle,
    Saw,
    Square,
    /// Pulso estrecho (25 %).
    Pulse,
}

/// Oscilador de fase normalizada (0–1).
#[derive(Debug, Clone, Copy, Default)]
pub struct Oscillator {
    pub phase: f32,
}

/// Corrección polyBLEP alrededor de una discontinuidad en `t` (fase), con incremento `dt`.
#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

impl Oscillator {
    pub fn new(phase: f32) -> Self {
        Self { phase }
    }

    pub fn reset(&mut self, phase: f32) {
        self.phase = phase;
    }

    /// Siguiente muestra. `dt` = frecuencia / frecuencia de muestreo.
    #[inline]
    pub fn next(&mut self, wave: Waveform, dt: f32) -> f32 {
        let t = self.phase;
        let dt = dt.clamp(0.0, 0.5);
        let out = match wave {
            Waveform::Sine => (t * TAU).sin(),
            Waveform::Saw => 2.0 * t - 1.0 - poly_blep(t, dt),
            Waveform::Square => square(t, dt, 0.5),
            Waveform::Pulse => square(t, dt, 0.25),
            // El triángulo solo tiene picos en la pendiente: su aliasing es muy suave.
            Waveform::Triangle => 1.0 - 4.0 * (t - 0.5).abs(),
        };
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        out
    }
}

#[inline]
fn square(t: f32, dt: f32, width: f32) -> f32 {
    let mut v = if t < width { 1.0 } else { -1.0 };
    v += poly_blep(t, dt);
    let t2 = (t + 1.0 - width) % 1.0;
    v -= poly_blep(t2, dt);
    // Cuadrada asimétrica: quitamos la continua para que esté centrada.
    v - (2.0 * width - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(wave: Waveform, freq: f32, n: usize) -> Vec<f32> {
        let mut o = Oscillator::default();
        (0..n).map(|_| o.next(wave, freq / 48_000.0)).collect()
    }

    #[test]
    fn amplitud_acotada_y_finita() {
        for wave in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Pulse,
            Waveform::Triangle,
        ] {
            for freq in [55.0, 440.0, 5_000.0, 20_000.0] {
                let s = render(wave, freq, 48_000);
                assert!(s.iter().all(|x| x.is_finite()), "{wave:?}");
                let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(peak < 1.6, "{wave:?} {freq}: {peak}");
            }
        }
    }

    #[test]
    fn sin_continua() {
        for wave in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Pulse,
            Waveform::Triangle,
        ] {
            let s = render(wave, 100.0, 48_000);
            let mean: f32 = s[24_000..].iter().sum::<f32>() / 24_000.0;
            assert!(mean.abs() < 0.05, "{wave:?}: {mean}");
        }
    }

    #[test]
    fn triangulo_tiene_amplitud() {
        let s = render(Waveform::Triangle, 220.0, 48_000);
        let peak = s[24_000..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.7, "{peak}");
    }
}
