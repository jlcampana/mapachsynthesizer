//! Caja de ritmos: voces de batería, patrones de 16 pasos y biblioteca de ritmos.

use mapach_dsp::drums::{Drum, DrumVoice, KITS};

/// Un patrón: 16 pasos, y en cada paso un bit por instrumento (orden de [`Drum::ALL`]).
pub type Pattern = [u8; 16];

pub const STEPS: usize = 16;

/// Estilos de ritmo.
pub const STYLES: [&str; 7] = [
    "Rock",
    "Pop",
    "Funk",
    "Disco",
    "Reguetón",
    "Hip-hop",
    "Bossa",
];

/// Variaciones de cada estilo.
pub const VARIATIONS: [&str; 8] = [
    "Normal",
    "Fantasma",
    "Charles",
    "Síncopa",
    "Redoble",
    "Mitad",
    "Doble",
    "Percusión",
];

const B: u8 = 1 << 0; // bombo
const S: u8 = 1 << 1; // caja
const H: u8 = 1 << 2; // charles cerrado
const O: u8 = 1 << 3; // charles abierto
const T: u8 = 1 << 4; // tom
const P: u8 = 1 << 5; // palmas
const C: u8 = 1 << 6; // platillo

/// Convierte filas de texto (`x` = golpe) en un patrón.
fn rows(r: &[(u8, &str)]) -> Pattern {
    let mut p = [0u8; STEPS];
    for &(bit, row) in r {
        for (i, ch) in row.bytes().filter(|b| !b.is_ascii_whitespace()).enumerate() {
            if i < STEPS && ch == b'x' {
                p[i] |= bit;
            }
        }
    }
    p
}

/// Patrón base de cada estilo.
fn base(style: usize) -> Pattern {
    match style % STYLES.len() {
        0 => rows(&[
            (B, "x... .... x.x. ...."),
            (S, ".... x... .... x..."),
            (H, "x.x. x.x. x.x. x.x."),
        ]),
        1 => rows(&[
            (B, "x... ..x. x... ...."),
            (S, ".... x... .... x..."),
            (H, "x.x. x.x. x.x. x.xx"),
        ]),
        2 => rows(&[
            (B, "x.x. ...x ..x. ...."),
            (S, ".... x..x .x.. x..."),
            (H, "xxxx xxxx xxxx xxxx"),
        ]),
        3 => rows(&[
            (B, "x... x... x... x..."),
            (P, ".... x... .... x..."),
            (H, "..x. ..x. ..x. ..x."),
            (O, "..x. .... ..x. ...."),
        ]),
        4 => rows(&[
            (B, "x... x... x... x..."),
            (S, "...x ..x. ...x ..x."),
            (H, "x.x. x.x. x.x. x.x."),
        ]),
        5 => rows(&[
            (B, "x... ...x x.x. ...."),
            (S, ".... x... .... x..."),
            (H, "x.x. x.x. x.x. x..."),
            (O, ".... .... .... ..x."),
        ]),
        _ => rows(&[
            (B, "x..x x..x x..x x..x"),
            (T, "..x. .x.. x..x ..x."),
            (H, "x.x. x.x. x.x. x.x."),
        ]),
    }
}

/// El ritmo `variation` del estilo `style`. Las variaciones se generan con reglas a partir
/// del patrón base, así hay 7 × 8 = 56 ritmos distintos.
pub fn rhythm(style: usize, variation: usize) -> Pattern {
    let mut p = base(style);
    match variation % VARIATIONS.len() {
        // Fantasma: cajas suaves entre medias.
        1 => {
            for i in [3, 7, 11] {
                if p[i] & B == 0 {
                    p[i] |= S;
                }
            }
            p[14] |= S;
        }
        // Charles en todas las semicorcheas y abierto a contratiempo.
        2 => {
            for (i, s) in p.iter_mut().enumerate() {
                *s |= if i % 4 == 2 { O } else { H };
            }
        }
        // Síncopa: el bombo del 3er tiempo se adelanta.
        3 => {
            if p[8] & B != 0 {
                p[8] &= !B;
                p[7] |= B;
            } else {
                p[14] |= B;
            }
            p[10] |= B;
        }
        // Redoble al final con toms y caja.
        4 => {
            for (i, s) in p.iter_mut().enumerate().skip(12) {
                *s = (*s & (B | H)) | if i % 2 == 0 { T } else { S };
            }
            p[0] |= C;
        }
        // Mitad: solo una caja, en el 3er tiempo.
        5 => {
            for (i, s) in p.iter_mut().enumerate() {
                *s &= !(S | P);
                if i != 0 && i != 10 {
                    *s &= !B;
                }
            }
            p[8] |= S;
        }
        // Doble: bombo en todos los tiempos (y uno más al final), caja doble y charles en
        // semicorcheas.
        6 => {
            for (i, s) in p.iter_mut().enumerate() {
                if i % 4 == 0 {
                    *s |= B;
                }
                *s |= H;
            }
            p[14] |= B;
            p[13] |= S;
        }
        // Percusión: palmas, tom y platillo.
        7 => {
            p[4] |= P;
            p[12] |= P;
            p[14] |= T;
            p[0] |= C;
        }
        _ => {}
    }
    p
}

/// Ritmo vacío para el secuenciador.
pub const EMPTY: Pattern = [0; STEPS];

/// La caja de ritmos del motor.
#[derive(Debug, Clone)]
pub struct DrumMachine {
    sample_rate: f32,
    voices: [DrumVoice; 7],
    /// Samples que sustituyen a un golpe, con su posición y volumen (`None` = parado).
    custom: [Option<std::sync::Arc<[f32]>>; 7],
    custom_pos: [Option<(usize, f32)>; 7],
    kit: usize,
    pattern: Pattern,
    playing: bool,
    step: usize,
    countdown: f64,
    level: f32,
}

impl DrumMachine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            voices: Drum::ALL.map(DrumVoice::new),
            kit: 0,
            pattern: rhythm(0, 0),
            playing: false,
            step: 0,
            countdown: 0.0,
            level: 0.6,
            custom: Default::default(),
            custom_pos: [None; 7],
        }
    }

    pub fn set_kit(&mut self, kit: usize) {
        self.kit = kit % KITS.len();
    }

    pub fn set_pattern(&mut self, p: Pattern) {
        self.pattern = p;
    }

    /// Arranca (desde el paso 1) o para.
    pub fn set_playing(&mut self, on: bool) {
        if on && !self.playing {
            self.step = 0;
            self.countdown = 0.0;
        }
        self.playing = on;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Paso que está sonando (el último que se disparó).
    pub fn step(&self) -> Option<usize> {
        self.playing.then(|| (self.step + STEPS - 1) % STEPS)
    }

    /// Pone (o quita) un sample en un golpe. Devuelve el anterior, para liberarlo fuera.
    pub fn set_sample(
        &mut self,
        drum: usize,
        data: Option<std::sync::Arc<[f32]>>,
    ) -> Option<std::sync::Arc<[f32]>> {
        let d = drum % 7;
        self.custom_pos[d] = None;
        std::mem::replace(&mut self.custom[d], data)
    }

    /// Golpe en directo.
    pub fn hit(&mut self, drum: usize, velocity: f32) {
        let d = drum % 7;
        if Drum::ALL[d] == Drum::CharlesCerrado {
            self.voices[Drum::CharlesAbierto.index()].choke();
            self.custom_pos[Drum::CharlesAbierto.index()] = None;
        }
        if self.custom[d].is_some() {
            self.custom_pos[d] = Some((0, velocity));
        } else {
            self.voices[d].trigger(velocity);
        }
    }

    /// Avanza `n` muestras (dispara los pasos que toquen) y suma la batería al bloque estéreo.
    pub fn render(&mut self, out: &mut [f32], bpm: f32) {
        let n = out.len() / 2;
        if self.playing {
            while self.countdown <= 0.0 {
                let bits = self.pattern[self.step];
                // Los tiempos fuertes, un poco más fuertes.
                let vel = if self.step.is_multiple_of(4) {
                    1.0
                } else {
                    0.8
                };
                for d in 0..7 {
                    if bits & (1 << d) != 0 {
                        self.hit(d, vel);
                    }
                }
                self.step = (self.step + 1) % STEPS;
                self.countdown += 60.0 / bpm.max(1.0) as f64 / 4.0 * self.sample_rate as f64;
            }
            self.countdown -= n as f64;
        }
        // Golpes con sample.
        for d in 0..7 {
            let (Some(data), Some((pos, vel))) = (&self.custom[d], &mut self.custom_pos[d]) else {
                continue;
            };
            for frame in out.chunks_exact_mut(2) {
                let Some(&v) = data.get(*pos) else {
                    self.custom_pos[d] = None;
                    break;
                };
                let s = v * *vel * self.level;
                frame[0] += s;
                frame[1] += s;
                *pos += 1;
            }
        }
        if self.voices.iter().all(|v| !v.is_active()) {
            return;
        }
        let kit = KITS[self.kit];
        for frame in out.chunks_exact_mut(2) {
            let mut s = 0.0;
            for v in self.voices.iter_mut() {
                s += v.sample(&kit, self.sample_rate);
            }
            s *= self.level;
            frame[0] += s;
            frame[1] += s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hay_56_ritmos_distintos() {
        let mut all = Vec::new();
        for s in 0..STYLES.len() {
            for v in 0..VARIATIONS.len() {
                let p = rhythm(s, v);
                assert!(p.iter().any(|&x| x & B != 0), "{s} {v} sin bombo");
                all.push(p);
            }
        }
        let n = all.len();
        for i in 0..n {
            for j in i + 1..n {
                assert_ne!(all[i], all[j], "ritmos {i} y {j} iguales");
            }
        }
    }

    #[test]
    fn patron_de_texto() {
        let p = rows(&[(B, "x... x..."), (S, ".x")]);
        assert_eq!(p[0], B);
        assert_eq!(p[1], S);
        assert_eq!(p[4], B);
        assert_eq!(p[2], 0);
    }

    #[test]
    fn sigue_el_tempo() {
        let mut d = DrumMachine::new(48_000.0);
        d.set_pattern(rhythm(0, 0));
        d.set_playing(true);
        let mut buf = vec![0.0; 64];
        // 120 BPM: una semicorchea = 6000 muestras. Tras 1 s (32 bloques de 1500 frames)
        // se han disparado 8 pasos.
        for _ in 0..32 {
            buf.resize(3000, 0.0);
            d.render(&mut buf, 120.0);
        }
        assert_eq!(d.step(), Some(7));
    }

    #[test]
    fn un_sample_sustituye_a_un_golpe() {
        let mut d = DrumMachine::new(48_000.0);
        assert!(
            d.set_sample(1, Some(std::sync::Arc::from(vec![0.5f32; 100])))
                .is_none()
        );
        d.hit(1, 1.0);
        let mut buf = vec![0.0; 400];
        d.render(&mut buf, 120.0);
        assert!((buf[0] - 0.3).abs() < 1e-6, "{}", buf[0]);
        assert_eq!(buf[300], 0.0, "se acaba con el sample");
        assert!(d.set_sample(1, None).is_some());
    }

    #[test]
    fn suena_y_para() {
        let mut d = DrumMachine::new(48_000.0);
        d.hit(0, 1.0);
        let mut buf = vec![0.0; 2000];
        d.render(&mut buf, 120.0);
        assert!(buf.iter().any(|&x| x.abs() > 0.05));
        d.set_playing(false);
        assert_eq!(d.step(), None);
    }
}
