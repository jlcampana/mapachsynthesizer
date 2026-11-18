//! Remuestreo con sinc enventanado (offline, para samples cortos).

use std::f32::consts::PI;

/// Mitad del ancho del núcleo, en muestras de entrada.
const HALF: usize = 16;

/// Cambia la frecuencia de muestreo de `input` de `from` a `to` Hz.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    // Al bajar la frecuencia, el filtro corta antes para que no haya aliasing.
    let cutoff = ratio.min(1.0) as f32 * 0.95;
    let half = (HALF as f32 / cutoff).ceil() as isize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 / ratio;
            let center = pos.floor() as isize;
            let frac = (pos - center as f64) as f32;
            let mut acc = 0.0f32;
            let mut norm = 0.0f32;
            for k in -half + 1..=half {
                let idx = center + k;
                if idx < 0 || idx as usize >= input.len() {
                    continue;
                }
                let x = (k as f32 - frac) * cutoff;
                let sinc = if x.abs() < 1e-6 {
                    1.0
                } else {
                    (PI * x).sin() / (PI * x)
                };
                // Ventana de Blackman.
                let t = (k as f32 - frac) / half as f32;
                let w = if t.abs() >= 1.0 {
                    0.0
                } else {
                    0.42 + 0.5 * (PI * t).cos() + 0.08 * (2.0 * PI * t).cos()
                };
                let h = sinc * w;
                acc += input[idx as usize] * h;
                norm += h;
            }
            if norm.abs() > 1e-6 { acc / norm } else { 0.0 }
        })
        .collect()
}

/// Cambia la velocidad (y el tono): `factor` 2 = el doble de rápido y una octava más agudo.
pub fn speed(input: &[f32], factor: f32) -> Vec<f32> {
    let factor = factor.clamp(0.1, 10.0);
    // Remuestrear "como si" viniera a otra frecuencia.
    resample(input, (48_000.0 * factor) as u32, 48_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, rate: u32, secs: f32) -> Vec<f32> {
        (0..(rate as f32 * secs) as usize)
            .map(|i| (i as f32 * freq * 2.0 * PI / rate as f32).sin())
            .collect()
    }

    /// Frecuencia por cruces por cero.
    fn freq(x: &[f32], rate: u32) -> f32 {
        let crossings = x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        crossings as f32 * rate as f32 / x.len() as f32
    }

    #[test]
    fn de_44100_a_48000_conserva_el_tono() {
        let x = tone(440.0, 44_100, 1.0);
        let y = resample(&x, 44_100, 48_000);
        assert_eq!(y.len(), 48_000);
        assert!((freq(&y, 48_000) - 440.0).abs() < 2.0);
        let peak = y[1000..47_000].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 1.0).abs() < 0.02, "{peak}");
    }

    #[test]
    fn bajar_la_frecuencia_quita_lo_que_no_cabe() {
        // 20 kHz no cabe a 16 kHz: debe desaparecer casi del todo.
        let x = tone(20_000.0, 48_000, 0.5);
        let y = resample(&x, 48_000, 16_000);
        let peak = y[500..7500].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak < 0.05, "{peak}");
    }

    #[test]
    fn velocidad_doble_es_una_octava() {
        let x = tone(220.0, 48_000, 1.0);
        let y = speed(&x, 2.0);
        assert!((y.len() as i32 - 24_000).abs() < 2);
        assert!((freq(&y, 48_000) - 440.0).abs() < 3.0);
    }
}
