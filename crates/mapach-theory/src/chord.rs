//! Tipos de acorde y sus intervalos.

use crate::scale::NOTE_NAMES;

/// Familia del acorde, que decide qué hace cada flecha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Major,
    Minor,
    Dominant,
}

/// Los tipos de acorde. El orden es el de fm1-chord (índice persistente) más `Power` al final.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordType {
    Major,
    Minor,
    Dim,
    Flat5,
    Aug,
    Sus4,
    Sus2,
    Maj7,
    Min7,
    Dom7,
    Maj6,
    Min6,
    Maj9,
    Min9,
    Dom7Sharp9,
    HalfDim7,
    Dom9,
    Add9,
    Add11,
    Min11,
    Sus4Dom7,
    MinMaj7,
    Maj13,
    Six9,
    Dom7Flat9,
    Dom7Alt,
    Maj7Sharp11,
    Dom13,
    Dim7,
    /// Quinta sola (raíz y quinta), para escalas sin tercera.
    Power,
    /// Menor con quinta aumentada (suena como la primera inversión del acorde mayor de la
    /// sexta menor de abajo). Sale en las pentatónicas.
    MinorSharp5,
}

impl ChordType {
    /// Todos los tipos, en orden.
    pub const ALL: [ChordType; 31] = {
        use ChordType::*;
        [
            Major,
            Minor,
            Dim,
            Flat5,
            Aug,
            Sus4,
            Sus2,
            Maj7,
            Min7,
            Dom7,
            Maj6,
            Min6,
            Maj9,
            Min9,
            Dom7Sharp9,
            HalfDim7,
            Dom9,
            Add9,
            Add11,
            Min11,
            Sus4Dom7,
            MinMaj7,
            Maj13,
            Six9,
            Dom7Flat9,
            Dom7Alt,
            Maj7Sharp11,
            Dom13,
            Dim7,
            Power,
            MinorSharp5,
        ]
    };

    /// Intervalos en semitonos desde la raíz.
    pub fn intervals(self) -> &'static [i32] {
        use ChordType::*;
        match self {
            Major => &[0, 4, 7],
            Minor => &[0, 3, 7],
            Dim => &[0, 3, 6],
            Flat5 => &[0, 4, 6],
            Aug => &[0, 4, 8],
            Sus4 => &[0, 5, 7],
            Sus2 => &[0, 2, 7],
            Maj7 => &[0, 4, 7, 11],
            Min7 => &[0, 3, 7, 10],
            Dom7 => &[0, 4, 7, 10],
            Maj6 => &[0, 4, 7, 9],
            Min6 => &[0, 3, 7, 9],
            Maj9 => &[0, 4, 7, 11, 14],
            Min9 => &[0, 3, 7, 10, 14],
            Dom7Sharp9 => &[0, 4, 7, 10, 15],
            HalfDim7 => &[0, 3, 6, 10],
            Dom9 => &[0, 4, 7, 10, 14],
            Add9 => &[0, 4, 7, 14],
            Add11 => &[0, 4, 7, 17],
            Min11 => &[0, 3, 7, 10, 14, 17],
            Sus4Dom7 => &[0, 5, 7, 10],
            MinMaj7 => &[0, 3, 7, 11],
            Maj13 => &[0, 4, 7, 11, 14, 21],
            Six9 => &[0, 4, 7, 9, 14],
            Dom7Flat9 => &[0, 4, 7, 10, 13],
            Dom7Alt => &[0, 4, 10, 13, 18],
            Maj7Sharp11 => &[0, 4, 7, 11, 18],
            Dom13 => &[0, 4, 7, 10, 14, 21],
            Dim7 => &[0, 3, 6, 9],
            Power => &[0, 7],
            MinorSharp5 => &[0, 3, 8],
        }
    }

    /// Notas de extensión (más allá de la tríada) que caben en las 2 ranuras extra.
    pub fn extensions(self) -> &'static [i32] {
        use ChordType::*;
        match self {
            Maj7 | MinMaj7 => &[11],
            Min7 | Dom7 | HalfDim7 | Sus4Dom7 => &[10],
            Maj6 | Min6 | Dim7 => &[9],
            Maj9 => &[11, 14],
            Min9 | Dom9 => &[10, 14],
            Dom7Sharp9 => &[10, 15],
            Add9 => &[14],
            Add11 => &[17],
            Min11 => &[10, 17],
            Maj13 => &[11, 21],
            Six9 => &[9, 14],
            Dom7Flat9 | Dom7Alt => &[10, 13],
            Maj7Sharp11 => &[11, 18],
            Dom13 => &[10, 21],
            _ => &[],
        }
    }

    /// Tercera (o lo que hace de tercera: 2ª o 4ª en los sus). `None` en la quinta sola.
    pub fn third(self) -> Option<i32> {
        match self {
            ChordType::Power => None,
            _ => Some(self.intervals()[1]),
        }
    }

    /// Quinta (justa, disminuida o aumentada).
    pub fn fifth(self) -> i32 {
        match self {
            ChordType::Power => 7,
            ChordType::Dom7Alt => 6,
            _ => self.intervals()[2],
        }
    }

    pub fn family(self) -> Family {
        use ChordType::*;
        match self {
            Minor | Min7 | Min6 | Min9 | Min11 | MinMaj7 | Dim | HalfDim7 | Dim7 | MinorSharp5 => {
                Family::Minor
            }
            Dom7 | Dom9 | Dom7Sharp9 | Dom7Flat9 | Dom7Alt | Dom13 | Sus4Dom7 => Family::Dominant,
            _ => Family::Major,
        }
    }

    /// Símbolo corto, como en los cifrados: "", "m", "7", "maj7"…
    pub fn symbol(self) -> &'static str {
        use ChordType::*;
        match self {
            Major => "",
            Minor => "m",
            Dim => "dim",
            Flat5 => "♭5",
            Aug => "aug",
            Sus4 => "sus4",
            Sus2 => "sus2",
            Maj7 => "maj7",
            Min7 => "m7",
            Dom7 => "7",
            Maj6 => "6",
            Min6 => "m6",
            Maj9 => "maj9",
            Min9 => "m9",
            Dom7Sharp9 => "7♯9",
            HalfDim7 => "m7♭5",
            Dom9 => "9",
            Add9 => "add9",
            Add11 => "add11",
            Min11 => "m11",
            Sus4Dom7 => "7sus4",
            MinMaj7 => "m(maj7)",
            Maj13 => "maj13",
            Six9 => "6/9",
            Dom7Flat9 => "7♭9",
            Dom7Alt => "7alt",
            Maj7Sharp11 => "maj7♯11",
            Dom13 => "13",
            Dim7 => "dim7",
            Power => "5",
            MinorSharp5 => "m♯5",
        }
    }

    /// Cambia mayor por menor y al revés (y sus parientes con 7ª, 6ª y 9ª).
    pub fn flip_major_minor(self) -> ChordType {
        use ChordType::*;
        match self {
            Major => Minor,
            Minor => Major,
            Maj7 => Min7,
            Min7 => Maj7,
            Maj6 => Min6,
            Min6 => Maj6,
            Maj9 => Min9,
            Min9 => Maj9,
            Dim => Minor,
            other => match other.family() {
                Family::Minor => Major,
                _ => Minor,
            },
        }
    }

    /// Busca el tipo con exactamente estos intervalos (tríada).
    pub fn from_triad(third: i32, fifth: i32) -> Option<ChordType> {
        use ChordType::*;
        Some(match (third, fifth) {
            (4, 7) => Major,
            (3, 7) => Minor,
            (3, 6) => Dim,
            (4, 6) => Flat5,
            (4, 8) => Aug,
            (5, 7) => Sus4,
            (2, 7) => Sus2,
            (3, 8) => MinorSharp5,
            _ => return None,
        })
    }
}

/// Un acorde concreto: raíz (nota MIDI) y tipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: i32,
    pub kind: ChordType,
}

impl Chord {
    pub fn new(root: i32, kind: ChordType) -> Self {
        Self { root, kind }
    }

    /// Notas del acorde en posición fundamental, sin voicing.
    pub fn notes(&self) -> Vec<i32> {
        self.kind
            .intervals()
            .iter()
            .map(|i| self.root + i)
            .collect()
    }

    /// Clase de altura de la raíz (0 = Do … 11 = Si).
    pub fn root_pc(&self) -> usize {
        self.root.rem_euclid(12) as usize
    }

    /// Nombre del acorde: "Do", "Rem7", "Sol7"…
    pub fn name(&self) -> String {
        format!("{}{}", NOTE_NAMES[self.root_pc()], self.kind.symbol())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervalos_crecientes_y_empiezan_en_cero() {
        for t in ChordType::ALL {
            let iv = t.intervals();
            assert_eq!(iv[0], 0, "{t:?}");
            assert!(iv.windows(2).all(|w| w[0] < w[1]), "{t:?}");
        }
    }

    #[test]
    fn extensiones_forman_parte_del_acorde() {
        for t in ChordType::ALL {
            for e in t.extensions() {
                assert!(t.intervals().contains(e), "{t:?} {e}");
            }
            assert!(t.extensions().len() <= 2);
        }
    }

    #[test]
    fn flip_ida_y_vuelta() {
        for t in [
            ChordType::Major,
            ChordType::Minor,
            ChordType::Maj7,
            ChordType::Min9,
        ] {
            assert_eq!(t.flip_major_minor().flip_major_minor(), t);
        }
    }

    #[test]
    fn nombres() {
        assert_eq!(Chord::new(60, ChordType::Major).name(), "Do");
        assert_eq!(Chord::new(62, ChordType::Minor).name(), "Rem");
        assert_eq!(Chord::new(67, ChordType::Dom7).name(), "Sol7");
    }
}
