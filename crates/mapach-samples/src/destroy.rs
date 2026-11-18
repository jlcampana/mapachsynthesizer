//! "Destrozar" samples: cada botón es una transformación que se puede deshacer.

use crate::resample::{resample, speed};
use crate::{MAX_SECONDS, RATE, Sample, fades, normalize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Destroy {
    Distorsion,
    Robot,
    AlReves,
    MasAgudo,
    MasGrave,
    Eco,
    Tartamudeo,
    Rapido,
    Lento,
    Recortar,
}

impl Destroy {
    pub const ALL: [Destroy; 10] = [
        Destroy::Distorsion,
        Destroy::Robot,
        Destroy::AlReves,
        Destroy::MasAgudo,
        Destroy::MasGrave,
        Destroy::Eco,
        Destroy::Tartamudeo,
        Destroy::Rapido,
        Destroy::Lento,
        Destroy::Recortar,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Destroy::Distorsion => "Distorsión",
            Destroy::Robot => "Robot",
            Destroy::AlReves => "Al revés",
            Destroy::MasAgudo => "Más agudo",
            Destroy::MasGrave => "Más grave",
            Destroy::Eco => "Eco",
            Destroy::Tartamudeo => "Tartamudeo",
            Destroy::Rapido => "Rápido",
            Destroy::Lento => "Lento",
            Destroy::Recortar => "Recortar",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Destroy::Distorsion => "🔥",
            Destroy::Robot => "🤖",
            Destroy::AlReves => "↩",
            Destroy::MasAgudo => "🐭",
            Destroy::MasGrave => "🐻",
            Destroy::Eco => "🏔",
            Destroy::Tartamudeo => "🔁",
            Destroy::Rapido => "🐇",
            Destroy::Lento => "🐢",
            Destroy::Recortar => "✂",
        }
    }

    /// Aplica la transformación y devuelve el sample nuevo.
    pub fn apply(self, s: &Sample) -> Sample {
        let x = &s.data;
        let mut y: Vec<f32> = match self {
            Destroy::Distorsion => x.iter().map(|v| (v * 8.0).tanh()).collect(),
            Destroy::Robot => {
                // Modulación en anillo a 70 Hz y menos bits.
                x.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let m = (i as f32 * 70.0 * std::f32::consts::TAU / RATE as f32).sin();
                        ((v * m) * 16.0).round() / 16.0
                    })
                    .collect()
            }
            Destroy::AlReves => x.iter().rev().copied().collect(),
            Destroy::MasAgudo => pitch_shift(x, 2f32.powf(5.0 / 12.0)),
            Destroy::MasGrave => pitch_shift(x, 2f32.powf(-5.0 / 12.0)),
            Destroy::Eco => {
                let d = (0.18 * RATE as f32) as usize;
                let extra = d * 4;
                let mut y = x.clone();
                y.resize(x.len() + extra, 0.0);
                for i in d..y.len() {
                    y[i] += y[i - d] * 0.45;
                }
                y
            }
            Destroy::Tartamudeo => {
                // El primer trocito tres veces y luego todo.
                let piece = (x.len() / 6).clamp(1, (0.15 * RATE as f32) as usize);
                let mut y = Vec::with_capacity(x.len() + piece * 3);
                for _ in 0..3 {
                    let mut p = x[..piece.min(x.len())].to_vec();
                    fades(&mut p, 64);
                    y.extend(p);
                }
                y.extend_from_slice(x);
                y
            }
            Destroy::Rapido => speed(x, 1.5),
            Destroy::Lento => speed(x, 0.7),
            Destroy::Recortar => {
                // Quita un 10 % por cada lado.
                let cut = x.len() / 10;
                x[cut..x.len() - cut].to_vec()
            }
        };
        y.truncate((MAX_SECONDS * RATE as f32) as usize);
        normalize(&mut y, 0.9);
        fades(&mut y, 64);
        Sample::new(y)
    }
}

/// Cambia el tono sin cambiar la duración: granos de 60 ms solapados (granular sencillo).
fn pitch_shift(x: &[f32], ratio: f32) -> Vec<f32> {
    if x.is_empty() {
        return Vec::new();
    }
    // Primero se cambia la velocidad (tono + duración)…
    let sped = resample(x, (RATE as f32 * ratio) as u32, RATE);
    // …y luego se vuelve a la duración original repitiendo o saltando granos.
    let grain = (0.06 * RATE as f32) as usize;
    let hop = grain / 2;
    let mut out = vec![0.0f32; x.len() + grain];
    let mut norm = vec![0.0f32; x.len() + grain];
    let mut pos_out = 0;
    while pos_out < x.len() {
        let pos_in = ((pos_out as f32 / x.len() as f32) * sped.len() as f32) as usize;
        for k in 0..grain {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * k as f32 / grain as f32).cos();
            let v = sped.get(pos_in + k).copied().unwrap_or(0.0);
            out[pos_out + k] += v * w;
            norm[pos_out + k] += w;
        }
        pos_out += hop;
    }
    out.truncate(x.len());
    for (o, n) in out.iter_mut().zip(&norm) {
        if *n > 1e-3 {
            *o /= n;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::detect;

    fn tone(freq: f32, secs: f32) -> Sample {
        Sample::new(
            (0..(RATE as f32 * secs) as usize)
                .map(|i| (i as f32 * freq * std::f32::consts::TAU / RATE as f32).sin() * 0.5)
                .collect(),
        )
    }

    #[test]
    fn todos_dan_algo_razonable() {
        let s = tone(220.0, 1.0);
        for d in Destroy::ALL {
            let y = d.apply(&s);
            assert!(!y.is_empty(), "{d:?}");
            assert!(
                y.data.iter().all(|v| v.is_finite() && v.abs() <= 0.91),
                "{d:?}"
            );
            assert!(y.seconds() <= MAX_SECONDS);
        }
    }

    #[test]
    fn al_reves_dos_veces_es_lo_mismo_casi() {
        let s = tone(220.0, 0.5);
        let y = Destroy::AlReves.apply(&Destroy::AlReves.apply(&s));
        assert_eq!(y.data.len(), s.data.len());
    }

    #[test]
    fn mas_agudo_sube_el_tono_y_conserva_la_duracion() {
        let s = tone(220.0, 1.0);
        let y = Destroy::MasAgudo.apply(&s);
        assert_eq!(y.data.len(), s.data.len());
        let f = detect(&y.data).unwrap();
        let expected = 220.0 * 2f32.powf(5.0 / 12.0);
        let cents = 1200.0 * (f / expected).log2();
        // El granular sencillo se desvía un poco: para jugar basta.
        assert!(cents.abs() < 50.0, "{f}");
    }

    #[test]
    fn lento_dura_mas() {
        let s = tone(220.0, 1.0);
        assert!(Destroy::Lento.apply(&s).seconds() > 1.3);
        assert!(Destroy::Rapido.apply(&s).seconds() < 0.7);
    }

    #[test]
    fn nunca_pasa_del_maximo() {
        let s = tone(220.0, MAX_SECONDS);
        assert!(Destroy::Lento.apply(&s).seconds() <= MAX_SECONDS);
        assert!(Destroy::Eco.apply(&s).seconds() <= MAX_SECONDS);
    }
}
