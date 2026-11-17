//! Piezas de DSP de Mapachsynthesizer: osciladores, envolventes, filtros y limitador.
//!
//! Todo trabaja muestra a muestra, sin reservar memoria después de construirse, para poder
//! usarse en el hilo de audio.

pub mod envelope;
pub mod filter;
pub mod limiter;
pub mod oscillator;
pub mod smooth;

pub use envelope::{Adsr, AdsrParams};
pub use filter::Svf;
pub use limiter::Limiter;
pub use oscillator::{Oscillator, Waveform};
pub use smooth::Smoothed;

/// Decibelios a ganancia lineal.
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Ganancia lineal a decibelios.
pub fn gain_to_db(g: f32) -> f32 {
    20.0 * g.max(1e-9).log10()
}

/// Panorama de potencia constante: -1 izquierda, 0 centro, 1 derecha. Devuelve (izq, der).
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (a.cos(), a.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decibelios() {
        assert!((db_to_gain(-6.0) - 0.501).abs() < 1e-3);
        assert!((gain_to_db(db_to_gain(-12.0)) + 12.0).abs() < 1e-4);
    }

    #[test]
    fn panorama_centrado() {
        let (l, r) = pan_gains(0.0);
        assert!((l - r).abs() < 1e-6);
        assert!((l * l + r * r - 1.0).abs() < 1e-5);
    }
}
