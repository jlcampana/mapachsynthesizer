//! Asistente "Aprender mando": pide pulsar cada control y guarda qué código manda.
//!
//! Sirve si el mando se reprograma o si se usa otro macropad. El resultado va a
//! `~/.config/mapachsynthesizer/mando.ron`.

use std::path::PathBuf;

use mapach_input::mando::{Mapping, Target};

/// Qué hace cada tecla del mando en modo Tocar (ver `docs/mando.md`).
const ROLES: [&str; 16] = [
    "acorde 1 (rojo)",
    "acorde 2 (naranja)",
    "acorde 3 (amarillo)",
    "acorde 4 (verde)",
    "acorde 5 (cian)",
    "acorde 6 (azul)",
    "acorde 7 (morado)",
    "Grabar",
    "flecha ←",
    "flecha ↑",
    "flecha ↓",
    "flecha →",
    "Tono (gris)",
    "Sonido (amarillo)",
    "Modo (rojo)",
    "Play",
];

/// Texto que pide pulsar un control.
pub fn prompt(t: Target) -> String {
    match t {
        Target::Key(n) => {
            let (row, col) = ((n - 1) / 4 + 1, (n - 1) % 4 + 1);
            format!(
                "Pulsa la tecla de la fila {row}, columna {col}\n({})",
                ROLES[(n - 1) as usize]
            )
        }
        Target::Right(k) => format!("Gira la perilla {} a la derecha ↻", k + 1),
        Target::Left(k) => format!("Gira la perilla {} a la izquierda ↺", k + 1),
        Target::Push(k) => format!("Pulsa la perilla {}", k + 1),
    }
}

pub fn path() -> Option<PathBuf> {
    crate::ajustes::Ajustes::path().map(|p| p.with_file_name("mando.ron"))
}

/// La tabla guardada, o la de serie.
pub fn load() -> Mapping {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| ron::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(m: &Mapping) -> std::io::Result<()> {
    let Some(p) = path() else { return Ok(()) };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = ron::ser::to_string_pretty(m, ron::ser::PrettyConfig::default())
        .map_err(std::io::Error::other)?;
    std::fs::write(p, text)
}

/// El asistente en marcha.
pub struct Learn {
    targets: Vec<Target>,
    codes: Vec<(u16, Target)>,
    /// Aviso (por ejemplo, "esa ya la has pulsado").
    pub message: Option<String>,
}

impl Learn {
    pub fn new() -> Self {
        Self {
            targets: Target::all(),
            codes: Vec::new(),
            message: None,
        }
    }

    pub fn step(&self) -> usize {
        self.codes.len()
    }

    pub fn total(&self) -> usize {
        self.targets.len()
    }

    pub fn current(&self) -> Option<Target> {
        self.targets.get(self.codes.len()).copied()
    }

    /// Llega un código. Devuelve la tabla completa cuando ya están todos.
    pub fn code(&mut self, code: u16) -> Option<Mapping> {
        let t = self.current()?;
        if self.codes.iter().any(|(c, _)| *c == code) {
            self.message = Some("Esa ya la has pulsado: prueba con la que se pide".into());
            return None;
        }
        self.message = None;
        self.codes.push((code, t));
        if self.current().is_none() {
            return Some(Mapping {
                codes: self.codes.clone(),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aprende_los_25_y_avisa_de_repetidos() {
        let mut l = Learn::new();
        assert_eq!(l.total(), 25);
        assert_eq!(l.current(), Some(Target::Key(1)));
        assert!(l.code(100).is_none());
        assert!(l.code(100).is_none());
        assert!(l.message.is_some());
        assert_eq!(l.step(), 1);
        let mut done = None;
        for c in 101..125 {
            done = l.code(c);
        }
        let m = done.unwrap();
        assert_eq!(m.codes.len(), 25);
        assert_eq!(
            m.translate(100, 1),
            Some(mapach_input::Event::Press(mapach_input::Control::Pad(1)))
        );
    }

    #[test]
    fn textos() {
        assert!(prompt(Target::Key(8)).contains("Grabar"));
        assert!(prompt(Target::Key(5)).contains("fila 2, columna 1"));
        assert!(prompt(Target::Left(2)).contains("perilla 3"));
    }
}
