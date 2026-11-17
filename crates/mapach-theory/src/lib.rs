//! Teoría musical de Mapachsynthesizer: escalas, acordes, modificadores y voice leading.
//!
//! No sabe nada de audio. Las tablas están reescritas a partir de la documentación pública de
//! [fm1-chord](https://github.com/math0ne/fm1-chord) (ver `docs/investigacion.md` §6.1).
//!
//! Las notas son números MIDI (`i32`, Do central = 60).

mod chord;
mod modifier;
mod scale;
mod voicing;

pub use chord::{Chord, ChordType, Family};
pub use modifier::{Direction, ModifierMode};
pub use scale::{Key, NOTE_NAMES, Scale};
pub use voicing::{Voicing, VoicingOptions, voice_lead};

/// Número de teclas de acorde.
pub const BUTTONS: usize = 7;

/// Nota MIDI de la raíz más grave de la tonalidad en la octava 0 (Do3).
pub const BASE_ROOT: i32 = 48;
