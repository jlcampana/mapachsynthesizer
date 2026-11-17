//! Escalas y tonalidades, y el acorde por defecto de cada tecla.

use crate::chord::{Chord, ChordType};
use crate::{BASE_ROOT, BUTTONS};

/// Nombres de las notas en español (con sostenidos).
pub const NOTE_NAMES: [&str; 12] = [
    "Do", "Do♯", "Re", "Re♯", "Mi", "Fa", "Fa♯", "Sol", "Sol♯", "La", "La♯", "Si",
];

/// Las escalas de Mapachsynthesizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Scale {
    #[default]
    Major,
    Minor,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    Dorian,
    Mixolydian,
    HarmonicMinor,
    /// Japonesa (in-sen).
    InSen,
    /// Árabe (hijaz).
    Hijaz,
}

impl Scale {
    pub const ALL: [Scale; 10] = [
        Scale::Major,
        Scale::Minor,
        Scale::MajorPentatonic,
        Scale::MinorPentatonic,
        Scale::Blues,
        Scale::Dorian,
        Scale::Mixolydian,
        Scale::HarmonicMinor,
        Scale::InSen,
        Scale::Hijaz,
    ];

    /// Grados en semitonos desde la tónica.
    pub fn degrees(self) -> &'static [i32] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::MajorPentatonic => &[0, 2, 4, 7, 9],
            Scale::MinorPentatonic => &[0, 3, 5, 7, 10],
            Scale::Blues => &[0, 3, 5, 6, 7, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Scale::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            Scale::InSen => &[0, 1, 5, 7, 10],
            Scale::Hijaz => &[0, 1, 4, 5, 7, 8, 10],
        }
    }

    /// Nombre para la pantalla.
    pub fn name(self) -> &'static str {
        match self {
            Scale::Major => "Mayor",
            Scale::Minor => "Menor",
            Scale::MajorPentatonic => "Pentatónica mayor",
            Scale::MinorPentatonic => "Pentatónica menor",
            Scale::Blues => "Blues",
            Scale::Dorian => "Dórica",
            Scale::Mixolydian => "Mixolidia",
            Scale::HarmonicMinor => "Menor armónica",
            Scale::InSen => "Japonesa",
            Scale::Hijaz => "Árabe",
        }
    }

    pub fn next(self) -> Scale {
        let i = Scale::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Scale::ALL[(i + 1) % Scale::ALL.len()]
    }

    pub fn prev(self) -> Scale {
        let i = Scale::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Scale::ALL[(i + Scale::ALL.len() - 1) % Scale::ALL.len()]
    }

    /// ¿Está esta clase de altura (relativa a la tónica) en la escala?
    pub fn contains(self, semitones: i32) -> bool {
        self.degrees().contains(&semitones.rem_euclid(12))
    }

    /// Semitonos (desde la tónica, pueden pasar de 12) de la raíz de la tecla `button` (0–6).
    /// Si la escala tiene menos de 7 notas, sigue en la octava siguiente.
    pub fn button_offset(self, button: usize) -> i32 {
        let d = self.degrees();
        d[button % d.len()] + 12 * (button / d.len()) as i32
    }

    /// Tipo de acorde por defecto de la tecla `button`.
    ///
    /// Con las escalas de 7 notas se apilan terceras de la escala (armonización diatónica).
    /// Con las demás se busca la tercera y la quinta más cercanas dentro de la escala; si no hay
    /// tercera, se usa sus4, sus2 o la quinta sola.
    pub fn button_chord_type(self, button: usize) -> ChordType {
        let d = self.degrees();
        let root = self.button_offset(button);
        if d.len() == 7 {
            let i = button % 7;
            let third = (d[(i + 2) % 7] - d[i]).rem_euclid(12);
            let fifth = (d[(i + 4) % 7] - d[i]).rem_euclid(12);
            return ChordType::from_triad(third, fifth).unwrap_or(ChordType::Power);
        }
        let has = |iv: i32| self.contains(root + iv);
        let fifth = [7, 6, 8].into_iter().find(|&f| has(f));
        let third = [4, 3].into_iter().find(|&t| has(t));
        match (third, fifth) {
            (Some(t), Some(f)) => ChordType::from_triad(t, f).unwrap_or(ChordType::Power),
            (Some(4), None) => ChordType::Major,
            (Some(_), None) => ChordType::Minor,
            (None, Some(7)) if has(5) => ChordType::Sus4,
            (None, Some(7)) if has(2) => ChordType::Sus2,
            _ => ChordType::Power,
        }
    }
}

/// Una tonalidad: tónica (0 = Do … 11 = Si), escala y octava (−2…+2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Key {
    pub tonic: i32,
    pub scale: Scale,
    pub octave: i32,
}

impl Key {
    pub fn new(tonic: i32, scale: Scale) -> Self {
        Self {
            tonic: tonic.rem_euclid(12),
            scale,
            octave: 0,
        }
    }

    /// Nombre de la tonalidad: "Do mayor", "La menor"…
    pub fn name(&self) -> String {
        format!(
            "{} {}",
            NOTE_NAMES[self.tonic.rem_euclid(12) as usize],
            self.scale.name().to_lowercase()
        )
    }

    /// Sube o baja la tónica `n` semitonos (da la vuelta en las 12).
    pub fn transpose(&mut self, n: i32) {
        self.tonic = (self.tonic + n).rem_euclid(12);
    }

    /// Cambia la octava, entre −2 y +2.
    pub fn shift_octave(&mut self, n: i32) {
        self.octave = (self.octave + n).clamp(-2, 2);
    }

    /// Nota MIDI de la tónica en la octava actual.
    pub fn tonic_note(&self) -> i32 {
        BASE_ROOT + self.tonic + 12 * self.octave
    }

    /// Acorde por defecto de la tecla `button` (0–6).
    pub fn chord(&self, button: usize) -> Chord {
        assert!(button < BUTTONS);
        Chord::new(
            self.tonic_note() + self.scale.button_offset(button),
            self.scale.button_chord_type(button),
        )
    }

    /// Nota de la escala número `n` (0, 1, 2…), para el modo Melodía.
    pub fn scale_note(&self, n: i32) -> i32 {
        let d = self.scale.degrees();
        let len = d.len() as i32;
        self.tonic_note() + 12 + d[n.rem_euclid(len) as usize] + 12 * n.div_euclid(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ChordType::*;

    fn kinds(scale: Scale) -> Vec<ChordType> {
        (0..7).map(|b| scale.button_chord_type(b)).collect()
    }

    #[test]
    fn do_mayor() {
        let key = Key::new(0, Scale::Major);
        let names: Vec<_> = (0..7).map(|b| key.chord(b).name()).collect();
        assert_eq!(names, ["Do", "Rem", "Mim", "Fa", "Sol", "Lam", "Sidim"]);
        assert_eq!(key.chord(0).root, 48);
    }

    #[test]
    fn tablas_diatonicas_de_fm1_chord() {
        assert_eq!(kinds(Scale::Minor), [Minor, Dim, Major, Minor, Minor, Major, Major]);
        assert_eq!(kinds(Scale::HarmonicMinor), [Minor, Dim, Aug, Minor, Major, Major, Dim]);
        assert_eq!(kinds(Scale::Dorian), [Minor, Minor, Major, Major, Minor, Dim, Major]);
        assert_eq!(kinds(Scale::Mixolydian), [Major, Minor, Dim, Major, Minor, Minor, Major]);
    }

    #[test]
    fn pentatonica_sigue_en_la_octava_siguiente() {
        let s = Scale::MajorPentatonic;
        let roots: Vec<_> = (0..7).map(|b| s.button_offset(b)).collect();
        assert_eq!(roots, [0, 2, 4, 7, 9, 12, 14]);
        let s = Scale::Blues;
        let roots: Vec<_> = (0..7).map(|b| s.button_offset(b)).collect();
        assert_eq!(roots, [0, 3, 5, 6, 7, 10, 12]);
    }

    #[test]
    fn todas_las_notas_de_cada_acorde_por_defecto_estan_en_la_escala() {
        for scale in Scale::ALL {
            for b in 0..7 {
                let root = scale.button_offset(b);
                for iv in scale.button_chord_type(b).intervals() {
                    assert!(
                        scale.contains(root + iv),
                        "{scale:?} tecla {b}: {:?}",
                        scale.button_chord_type(b)
                    );
                }
            }
        }
    }

    #[test]
    fn transponer_da_la_vuelta() {
        let mut k = Key::new(11, Scale::Major);
        k.transpose(1);
        assert_eq!(k.tonic, 0);
        k.transpose(-1);
        assert_eq!(k.tonic, 11);
        k.shift_octave(5);
        assert_eq!(k.octave, 2);
    }

    #[test]
    fn notas_de_escala() {
        let k = Key::new(0, Scale::Major);
        assert_eq!(k.scale_note(0), 60);
        assert_eq!(k.scale_note(7), 72);
        assert_eq!(k.scale_note(-1), 59);
    }
}
