//! Detección de tono (YIN simplificado), para afinar los samples a Do.

use crate::RATE;

/// Frecuencia fundamental de la parte más fuerte del sample, o `None` si no tiene tono claro
/// (ruido, golpes…).
pub fn detect(data: &[f32]) -> Option<f32> {
    const WINDOW: usize = 2048;
    if data.len() < WINDOW * 2 {
        return None;
    }
    // Ventana alrededor del punto más fuerte (dejando sitio para el desfase).
    let loudest = data
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map_or(0, |(i, _)| i);
    let start = loudest.min(data.len() - WINDOW * 2);
    let x = &data[start..start + WINDOW * 2];

    // Entre 60 Hz y 1500 Hz.
    let min_lag = (RATE as f32 / 1500.0) as usize;
    let max_lag = (RATE as f32 / 60.0) as usize;
    let mut d = vec![0.0f32; max_lag + 1];
    for (lag, dl) in d.iter_mut().enumerate().skip(1) {
        *dl = (0..WINDOW).map(|i| (x[i] - x[i + lag]).powi(2)).sum();
    }
    // Diferencia normalizada acumulada.
    let mut cmnd = vec![1.0f32; max_lag + 1];
    let mut running = 0.0;
    for lag in 1..=max_lag {
        running += d[lag];
        cmnd[lag] = if running > 0.0 {
            d[lag] * lag as f32 / running
        } else {
            1.0
        };
    }
    // Primer mínimo por debajo del umbral.
    let threshold = 0.15;
    let mut lag = None;
    let mut i = min_lag;
    while i < max_lag {
        if cmnd[i] < threshold {
            while i + 1 < max_lag && cmnd[i + 1] < cmnd[i] {
                i += 1;
            }
            lag = Some(i);
            break;
        }
        i += 1;
    }
    let lag = lag?;
    // Interpolación parabólica para afinar.
    let (a, b, c) = (cmnd[lag - 1], cmnd[lag], cmnd[lag + 1]);
    let denom = a - 2.0 * b + c;
    let shift = if denom.abs() > 1e-9 {
        0.5 * (a - c) / denom
    } else {
        0.0
    };
    Some(RATE as f32 / (lag as f32 + shift))
}

/// Nota MIDI (con decimales) de una frecuencia.
pub fn freq_to_midi(f: f32) -> f32 {
    69.0 + 12.0 * (f / 440.0).log2()
}

/// Nota "raíz" del sample para tocarlo como instrumento: la detectada, o Do4 (60) si no tiene
/// tono claro.
pub fn root_note(data: &[f32]) -> f32 {
    detect(data).map_or(60.0, freq_to_midi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, secs: f32) -> Vec<f32> {
        (0..(RATE as f32 * secs) as usize)
            .map(|i| {
                let t = i as f32 / RATE as f32;
                // Con armónicos, como una voz.
                (t * freq * std::f32::consts::TAU).sin() * 0.6
                    + (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.3
                    + (t * freq * 3.0 * std::f32::consts::TAU).sin() * 0.1
            })
            .collect()
    }

    #[test]
    fn detecta_notas() {
        for f in [110.0, 220.0, 261.63, 440.0, 880.0] {
            let got = detect(&tone(f, 0.5)).unwrap();
            let cents = 1200.0 * (got / f).log2();
            assert!(cents.abs() < 10.0, "{f}: {got}");
        }
    }

    #[test]
    fn el_ruido_no_tiene_tono() {
        let mut n = 12345u32;
        let noise: Vec<f32> = (0..RATE as usize)
            .map(|_| {
                n ^= n << 13;
                n ^= n >> 17;
                n ^= n << 5;
                n as f32 / u32::MAX as f32 - 0.5
            })
            .collect();
        assert!(detect(&noise).is_none());
        assert_eq!(root_note(&noise), 60.0);
    }

    #[test]
    fn la_es_69() {
        assert!((freq_to_midi(440.0) - 69.0).abs() < 1e-4);
        assert!((root_note(&tone(261.63, 0.5)) - 60.0).abs() < 0.1);
    }
}
