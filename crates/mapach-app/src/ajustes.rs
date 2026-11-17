//! Ajustes que se guardan solos en `~/.config/mapachsynthesizer/ajustes.ron`.

use std::path::PathBuf;

use mapach_dsp::drums::KITS;
use mapach_engine::drums::{STYLES, VARIATIONS};
use mapach_engine::{ARP_PATTERNS, RATES, SOUNDS, STRUM_MS};
use mapach_theory::{Key, ModifierMode, Scale};
use serde::{Deserialize, Serialize};

use crate::state::{BPM_MAX, BPM_MIN, PlayMode, State};

/// Lo que se guarda. Las listas se guardan por posición para no atarnos a los tipos internos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ajustes {
    pub tonica: i32,
    pub escala: usize,
    pub octava: i32,
    pub flechas: usize,
    pub sonido: usize,
    pub volumen: f32,
    pub ataque: f32,
    pub release: f32,
    pub brillo: f32,
    pub tempo: f32,
    pub bajo: bool,
    pub nombres: bool,
    pub modo: usize,
    pub patron: usize,
    pub figura: usize,
    pub rasgueo: usize,
    pub repetir: usize,
    pub kit: usize,
    pub ritmo: usize,
    pub variacion: usize,
    /// "Mi ritmo": 16 pasos, un bit por instrumento.
    pub mi_ritmo: Vec<u8>,
}

impl Default for Ajustes {
    fn default() -> Self {
        Ajustes::from_state(&State::default())
    }
}

fn pos<T: PartialEq>(all: &[T], x: &T) -> usize {
    all.iter().position(|a| a == x).unwrap_or(0)
}

impl Ajustes {
    pub fn from_state(s: &State) -> Self {
        Self {
            tonica: s.key.tonic,
            escala: pos(&Scale::ALL, &s.key.scale),
            octava: s.key.octave,
            flechas: pos(&ModifierMode::ALL, &s.mod_mode),
            sonido: s.sound,
            volumen: s.volume,
            ataque: s.attack,
            release: s.release,
            brillo: s.brightness,
            tempo: s.bpm,
            bajo: s.bass,
            nombres: s.show_names,
            modo: pos(&PlayMode::ALL, &s.play_mode),
            patron: s.arp_pattern,
            figura: s.arp_rate,
            rasgueo: s.strum_speed,
            repetir: s.repeat_rate,
            kit: s.kit,
            ritmo: s.drum_style,
            variacion: s.drum_variation,
            mi_ritmo: s.seq_pattern.to_vec(),
        }
    }

    /// Aplica los ajustes a un estado, corrigiendo valores fuera de rango.
    pub fn apply(&self, s: &mut State) {
        let mut key = Key::new(self.tonica, Scale::ALL[self.escala % Scale::ALL.len()]);
        key.octave = self.octava.clamp(-2, 2);
        s.key = key;
        s.mod_mode = ModifierMode::ALL[self.flechas % ModifierMode::ALL.len()];
        s.sound = self.sonido % SOUNDS.len();
        s.volume = self.volumen.clamp(0.0, 1.0);
        s.attack = self.ataque.clamp(0.0, 5.0);
        s.release = self.release.clamp(0.0, 8.0);
        s.brightness = self.brillo.clamp(0.0, 1.0);
        s.bpm = self.tempo.clamp(BPM_MIN, BPM_MAX);
        s.bass = self.bajo;
        s.show_names = self.nombres;
        s.play_mode = PlayMode::ALL[self.modo % PlayMode::ALL.len()];
        s.arp_pattern = self.patron % ARP_PATTERNS.len();
        s.arp_rate = self.figura % RATES.len();
        s.strum_speed = self.rasgueo % STRUM_MS.len();
        s.repeat_rate = self.repetir % RATES.len();
        s.kit = self.kit % KITS.len();
        s.drum_style = self.ritmo % (STYLES.len() + 1);
        s.drum_variation = self.variacion % VARIATIONS.len();
        for (dst, src) in s.seq_pattern.iter_mut().zip(&self.mi_ritmo) {
            *dst = src & 0x7f;
        }
    }

    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("mapachsynthesizer").join("ajustes.ron"))
    }

    pub fn load() -> Option<Self> {
        let text = std::fs::read_to_string(Self::path()?).ok()?;
        match ron::from_str(&text) {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("Ajustes estropeados, uso los de serie: {e}");
                None
            }
        }
    }

    /// Guarda escribiendo primero a un temporal, para no dejar el fichero a medias.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        let tmp = path.with_extension("ron.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let mut s = State::default();
        s.key = Key::new(7, Scale::Blues);
        s.sound = 3;
        s.play_mode = PlayMode::Arpegio;
        s.bpm = 95.0;
        s.seq_pattern[3] = 0b101;
        s.drum_style = 7;
        let a = Ajustes::from_state(&s);
        let text = ron::to_string(&a).unwrap();
        let b: Ajustes = ron::from_str(&text).unwrap();
        let mut t = State::default();
        b.apply(&mut t);
        assert_eq!(Ajustes::from_state(&t), a);
    }

    #[test]
    fn valores_raros_se_corrigen() {
        let a = Ajustes {
            escala: 999,
            sonido: 999,
            tempo: 5000.0,
            volumen: -3.0,
            octava: 9,
            ..Default::default()
        };
        let mut s = State::default();
        a.apply(&mut s);
        assert_eq!(s.bpm, BPM_MAX);
        assert_eq!(s.volume, 0.0);
        assert_eq!(s.key.octave, 2);
    }

    #[test]
    fn campos_que_faltan_toman_su_valor_de_serie() {
        let a: Ajustes = ron::from_str("(tempo: 90.0)").unwrap();
        assert_eq!(a.tempo, 90.0);
        assert_eq!(a.sonido, 0);
    }
}
