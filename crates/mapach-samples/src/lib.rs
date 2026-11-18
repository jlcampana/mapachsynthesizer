//! Samples de Mapachsynthesizer: importar, analizar, "destrozar" y guardar.
//!
//! Todos los samples se guardan en mono a [`RATE`] Hz, como mucho [`MAX_SECONDS`] segundos.

pub mod decode;
pub mod destroy;
pub mod library;
pub mod pitch;
pub mod resample;

/// Frecuencia de muestreo de la biblioteca.
pub const RATE: u32 = 48_000;

/// Duración máxima de un sample.
pub const MAX_SECONDS: f32 = 10.0;

/// Un sample: audio mono a [`RATE`] Hz.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sample {
    pub data: Vec<f32>,
}

impl Sample {
    pub fn new(data: Vec<f32>) -> Self {
        Self { data }
    }

    pub fn seconds(&self) -> f32 {
        self.data.len() as f32 / RATE as f32
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Lo deja listo para la biblioteca: quita silencios al principio y al final, corta a
    /// [`MAX_SECONDS`], normaliza y pone fundidos cortitos para que no haga clic.
    pub fn tidy(&mut self) {
        trim_silence(&mut self.data, 0.01);
        self.data.truncate((MAX_SECONDS * RATE as f32) as usize);
        normalize(&mut self.data, 0.9);
        fades(&mut self.data, (0.003 * RATE as f32) as usize);
    }

    /// Picos para dibujar la forma de onda en `n` columnas.
    pub fn overview(&self, n: usize) -> Vec<f32> {
        if self.data.is_empty() || n == 0 {
            return vec![0.0; n];
        }
        (0..n)
            .map(|i| {
                let a = i * self.data.len() / n;
                let b = ((i + 1) * self.data.len() / n)
                    .max(a + 1)
                    .min(self.data.len());
                self.data[a..b].iter().fold(0.0f32, |m, x| m.max(x.abs()))
            })
            .collect()
    }
}

/// Quita lo que esté por debajo de `threshold` al principio y al final.
pub fn trim_silence(data: &mut Vec<f32>, threshold: f32) {
    let Some(start) = data.iter().position(|x| x.abs() > threshold) else {
        data.clear();
        return;
    };
    let end = data
        .iter()
        .rposition(|x| x.abs() > threshold)
        .map_or(start, |i| i + 1);
    // Un pelín de margen para no cortar el ataque.
    let margin = (0.005 * RATE as f32) as usize;
    let start = start.saturating_sub(margin);
    let end = (end + margin).min(data.len());
    data.truncate(end);
    data.drain(..start.min(data.len()));
}

/// Escala para que el pico sea `peak`.
pub fn normalize(data: &mut [f32], peak: f32) {
    let max = data.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if max > 1e-6 {
        let g = peak / max;
        data.iter_mut().for_each(|x| *x *= g);
    }
}

/// Fundidos de entrada y salida de `n` muestras.
pub fn fades(data: &mut [f32], n: usize) {
    let n = n.min(data.len() / 2);
    let len = data.len();
    for i in 0..n {
        let g = i as f32 / n as f32;
        data[i] *= g;
        data[len - 1 - i] *= g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordenar_un_sample() {
        let mut data = vec![0.0; 10_000];
        data.extend((0..4800).map(|i| (i as f32 * 0.1).sin() * 0.3));
        data.extend(vec![0.0; 10_000]);
        let mut s = Sample::new(data);
        s.tidy();
        assert!(s.data.len() < 6000, "{}", s.data.len());
        let peak = s.data.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!((peak - 0.9).abs() < 0.01);
        assert_eq!(s.data[0], 0.0);
    }

    #[test]
    fn silencio_total_queda_vacio() {
        let mut s = Sample::new(vec![0.0; 1000]);
        s.tidy();
        assert!(s.is_empty());
    }

    #[test]
    fn resumen_para_dibujar() {
        let s = Sample::new(vec![0.5; 100]);
        let o = s.overview(10);
        assert_eq!(o.len(), 10);
        assert!(o.iter().all(|&x| x == 0.5));
        assert_eq!(Sample::default().overview(4), vec![0.0; 4]);
    }
}
