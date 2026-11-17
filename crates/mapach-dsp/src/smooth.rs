//! Parámetros suavizados, para que girar la perilla no haga clics.

#[derive(Debug, Clone, Copy)]
pub struct Smoothed {
    current: f32,
    target: f32,
    coef: f32,
}

impl Smoothed {
    /// `time` en segundos (constante de tiempo).
    pub fn new(value: f32, time: f32, sample_rate: f32) -> Self {
        Self {
            current: value,
            target: value,
            coef: 1.0 - (-1.0 / (time * sample_rate)).exp(),
        }
    }

    pub fn set(&mut self, target: f32) {
        self.target = target;
    }

    pub fn set_immediate(&mut self, value: f32) {
        self.target = value;
        self.current = value;
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        self.current += (self.target - self.current) * self.coef;
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llega_al_objetivo() {
        let mut s = Smoothed::new(0.0, 0.01, 48_000.0);
        s.set(1.0);
        let first = s.tick();
        assert!(first > 0.0 && first < 0.05);
        for _ in 0..48_000 {
            s.tick();
        }
        assert!((s.tick() - 1.0).abs() < 1e-4);
    }
}
