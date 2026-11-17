//! Modificadores: qué hace cada flecha con el acorde que se mantiene pulsado.

use crate::chord::{Chord, ChordType, Family};

/// Las 8 direcciones del joystick (flechas, y diagonales con dos flechas a la vez).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

impl Direction {
    /// Dirección a partir de las flechas pulsadas. Las opuestas se anulan.
    pub fn from_arrows(up: bool, down: bool, left: bool, right: bool) -> Option<Direction> {
        let y = up as i32 - down as i32;
        let x = right as i32 - left as i32;
        use Direction::*;
        Some(match (x, y) {
            (0, 1) => Up,
            (1, 1) => UpRight,
            (1, 0) => Right,
            (1, -1) => DownRight,
            (0, -1) => Down,
            (-1, -1) => DownLeft,
            (-1, 0) => Left,
            (-1, 1) => UpLeft,
            _ => return None,
        })
    }

    /// Flecha para dibujar.
    pub fn arrow(self) -> &'static str {
        use Direction::*;
        match self {
            Up => "↑",
            UpRight => "↗",
            Right => "→",
            DownRight => "↘",
            Down => "↓",
            DownLeft => "↙",
            Left => "←",
            UpLeft => "↖",
        }
    }

    /// Orden de "colores" que recorre la perilla mágica (P2): normal → mayor/menor → 7ª →
    /// 7ª dominante → 9ª → sus4 → disminuido → aumentado.
    pub const KNOB_CYCLE: [Option<Direction>; 8] = [
        None,
        Some(Direction::Up),
        Some(Direction::Right),
        Some(Direction::UpRight),
        Some(Direction::DownRight),
        Some(Direction::Down),
        Some(Direction::Left),
        Some(Direction::UpLeft),
    ];
}

/// Tablas de modificadores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModifierMode {
    /// Básico: mayor/menor, 7ª, 9ª, sus, disminuido, aumentado.
    #[default]
    Basic,
    /// Jazz: acordes extendidos (9ª, 11ª, 7♯9…).
    Jazz,
    /// Prestados: dominantes secundarias, napolitano, sustituto tritonal…
    Borrow,
}

impl ModifierMode {
    pub const ALL: [ModifierMode; 3] = [ModifierMode::Basic, ModifierMode::Jazz, ModifierMode::Borrow];

    pub fn name(self) -> &'static str {
        match self {
            ModifierMode::Basic => "Básico",
            ModifierMode::Jazz => "Jazz",
            ModifierMode::Borrow => "Prestados",
        }
    }

    /// Aplica la dirección `dir` al acorde por defecto `base` de una tecla.
    pub fn apply(self, base: Chord, dir: Option<Direction>) -> Chord {
        let Some(dir) = dir else { return base };
        use ChordType::*;
        use Direction::*;
        let family = base.kind.family();
        let is_dim = matches!(base.kind, Dim | HalfDim7 | Dim7);
        let minor = family == Family::Minor;
        let (offset, kind) = match self {
            ModifierMode::Basic => {
                let k = match dir {
                    Up if is_dim => Minor,
                    Up => base.kind.flip_major_minor(),
                    UpRight => Dom7,
                    Right if is_dim => HalfDim7,
                    Right if minor => Min7,
                    Right => Maj7,
                    DownRight if is_dim => HalfDim7,
                    DownRight if minor => Min9,
                    DownRight => Maj9,
                    Down => Sus4,
                    DownLeft if minor => Sus2,
                    DownLeft => Maj6,
                    Left if is_dim => Minor,
                    Left => Dim,
                    UpLeft => Aug,
                };
                (0, k)
            }
            ModifierMode::Jazz => {
                let k = match dir {
                    Up => base.kind.flip_major_minor(),
                    UpRight => Dom9,
                    Right if minor => Min11,
                    Right => Add11,
                    DownRight => Min11,
                    Down => Dom7Sharp9,
                    DownLeft => Add9,
                    Left => Sus4Dom7,
                    UpLeft => HalfDim7,
                };
                (0, k)
            }
            ModifierMode::Borrow => match dir {
                Up => (0, Dom7),
                UpRight => (-2, Dom7),
                Right => (1, Dim7),
                DownRight => (0, HalfDim7),
                Down => (0, base.kind.flip_major_minor()),
                DownLeft if minor => (0, Maj7),
                DownLeft => (0, Min7),
                Left => (-1, Major),
                UpLeft => (6, Dom7),
            },
        };
        Chord::new(base.root + offset, kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ChordType::*;
    use Direction::*;

    const C: Chord = Chord { root: 48, kind: Major };
    const DM: Chord = Chord { root: 50, kind: Minor };
    const BDIM: Chord = Chord { root: 59, kind: Dim };

    #[test]
    fn flechas() {
        assert_eq!(Direction::from_arrows(true, false, false, true), Some(UpRight));
        assert_eq!(Direction::from_arrows(true, true, false, false), None);
        assert_eq!(Direction::from_arrows(false, false, false, false), None);
        assert_eq!(Direction::from_arrows(false, true, true, false), Some(DownLeft));
    }

    #[test]
    fn tabla_basica() {
        let m = ModifierMode::Basic;
        assert_eq!(m.apply(C, None), C);
        assert_eq!(m.apply(C, Some(Up)).kind, Minor);
        assert_eq!(m.apply(DM, Some(Up)).kind, Major);
        assert_eq!(m.apply(BDIM, Some(Up)).kind, Minor);
        assert_eq!(m.apply(C, Some(Right)).kind, Maj7);
        assert_eq!(m.apply(DM, Some(Right)).kind, Min7);
        assert_eq!(m.apply(BDIM, Some(Right)).kind, HalfDim7);
        assert_eq!(m.apply(C, Some(UpRight)).kind, Dom7);
        assert_eq!(m.apply(C, Some(DownLeft)).kind, Maj6);
        assert_eq!(m.apply(DM, Some(DownLeft)).kind, Sus2);
        assert_eq!(m.apply(C, Some(Left)).kind, Dim);
        assert_eq!(m.apply(BDIM, Some(Left)).kind, Minor);
        assert_eq!(m.apply(C, Some(UpLeft)).kind, Aug);
    }

    #[test]
    fn prestados_mueven_la_raiz() {
        let m = ModifierMode::Borrow;
        assert_eq!(m.apply(C, Some(UpLeft)), Chord::new(54, Dom7));
        assert_eq!(m.apply(C, Some(Left)), Chord::new(47, Major));
        assert_eq!(m.apply(C, Some(Right)), Chord::new(49, Dim7));
    }

    #[test]
    fn todas_las_combinaciones_dan_un_acorde() {
        for mode in ModifierMode::ALL {
            for base in [C, DM, BDIM] {
                for dir in Direction::KNOB_CYCLE {
                    let c = mode.apply(base, dir);
                    assert!((c.root - base.root).abs() <= 6);
                }
            }
        }
    }
}
