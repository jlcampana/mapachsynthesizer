//! Canciones de Mapachsynthesizer: guardar, cargar y exportar.
//!
//! Una canción es una carpeta:
//!
//! ```text
//! Dragón Naranja/
//! ├── cancion.ron      # nombre, color, ajustes y mezcla
//! └── pistas/1.wav … 6.wav   # audio del looper
//! ```

pub mod export;
pub mod names;
pub mod paths;
pub mod song;

pub use export::{Format, mixdown};
pub use names::funny_name;
pub use song::{Song, SongInfo, TrackAudio};
