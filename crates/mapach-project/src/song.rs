//! Guardar y cargar canciones.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::paths::{safe_file_name, songs_dir};

pub const TRACKS: usize = 6;

/// Audio de una pista: estéreo intercalado a `sample_rate`.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackAudio {
    pub samples: Vec<f32>,
}

impl TrackAudio {
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }
}

/// Datos de una canción. `S` son los ajustes de la app (tonalidad, sonido…).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "S: DeserializeOwned + Default"))]
pub struct Song<S> {
    pub nombre: String,
    pub color: [u8; 3],
    /// Segundos desde 1970 de la última vez que se guardó.
    #[serde(default)]
    pub guardada: u64,
    #[serde(default)]
    pub ajustes: S,
    #[serde(default = "default_rate")]
    pub frecuencia: u32,
    /// Frames de la vuelta del looper.
    #[serde(default)]
    pub vuelta: usize,
    #[serde(default = "default_volumes")]
    pub volumenes: [f32; TRACKS],
    #[serde(default)]
    pub silenciadas: [bool; TRACKS],
    /// Pistas del looper (no van en el `.ron`, sino en `pistas/N.wav`).
    #[serde(skip)]
    pub pistas: [Option<TrackAudio>; TRACKS],
}

fn default_rate() -> u32 {
    48_000
}

fn default_volumes() -> [f32; TRACKS] {
    [1.0; TRACKS]
}

/// Resumen de una canción para la lista (sin cargar el audio).
#[derive(Debug, Clone, PartialEq)]
pub struct SongInfo {
    pub nombre: String,
    pub color: [u8; 3],
    pub guardada: u64,
    pub carpeta: PathBuf,
    /// Pistas con audio.
    pub pistas: usize,
}

impl<S: Serialize + DeserializeOwned + Default> Song<S> {
    pub fn new(nombre: String, color: [u8; 3], ajustes: S) -> Self {
        Self {
            nombre,
            color,
            guardada: 0,
            ajustes,
            frecuencia: default_rate(),
            vuelta: 0,
            volumenes: default_volumes(),
            silenciadas: [false; TRACKS],
            pistas: Default::default(),
        }
    }

    /// Carpeta de la canción dentro de `base`.
    pub fn folder_in(&self, base: &Path) -> PathBuf {
        base.join(safe_file_name(&self.nombre))
    }

    /// Guarda en `~/Música/Mapachsynthesizer/Canciones/<nombre>/`.
    pub fn save(&mut self) -> Result<PathBuf> {
        self.save_in(&songs_dir())
    }

    /// Guarda en `base/<nombre>/`. Escribe primero en una carpeta temporal y luego la cambia
    /// por la buena, para que una canción nunca quede a medias.
    pub fn save_in(&mut self, base: &Path) -> Result<PathBuf> {
        self.guardada = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = self.folder_in(base);
        let tmp = base.join(format!(".{}.tmp", safe_file_name(&self.nombre)));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(tmp.join("pistas"))
            .with_context(|| format!("no se pudo crear {}", tmp.display()))?;
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        std::fs::write(tmp.join("cancion.ron"), text)?;
        for (i, p) in self.pistas.iter().enumerate() {
            if let Some(t) = p {
                write_wav_f32(
                    &tmp.join("pistas").join(format!("{}.wav", i + 1)),
                    t,
                    self.frecuencia,
                )?;
            }
        }
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::rename(&tmp, &dir)?;
        Ok(dir)
    }

    /// Carga una canción de su carpeta.
    pub fn load(dir: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(dir.join("cancion.ron"))
            .with_context(|| format!("no se pudo leer {}", dir.display()))?;
        let mut song: Song<S> = ron::from_str(&text)?;
        for (i, slot) in song.pistas.iter_mut().enumerate() {
            let path = dir.join("pistas").join(format!("{}.wav", i + 1));
            if path.exists() {
                *slot = Some(read_wav(&path)?);
            }
        }
        Ok(song)
    }
}

/// Lista las canciones guardadas en `base`, la más reciente primero.
pub fn list_in(base: &Path) -> Vec<SongInfo> {
    #[derive(Deserialize)]
    struct Head {
        nombre: String,
        color: [u8; 3],
        #[serde(default)]
        guardada: u64,
    }
    let mut v = Vec::new();
    let Ok(dir) = std::fs::read_dir(base) else {
        return v;
    };
    for e in dir.flatten() {
        let path = e.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path.join("cancion.ron")) else {
            continue;
        };
        // Leemos solo la cabecera; serde ignora los demás campos.
        let Ok(h) = ron::from_str::<Head>(&text) else {
            continue;
        };
        let pistas = (1..=TRACKS)
            .filter(|i| path.join("pistas").join(format!("{i}.wav")).exists())
            .count();
        v.push(SongInfo {
            nombre: h.nombre,
            color: h.color,
            guardada: h.guardada,
            carpeta: path,
            pistas,
        });
    }
    v.sort_by(|a, b| b.guardada.cmp(&a.guardada).then(a.nombre.cmp(&b.nombre)));
    v
}

/// Lista las canciones de `~/Música/Mapachsynthesizer/Canciones`.
pub fn list() -> Vec<SongInfo> {
    list_in(&songs_dir())
}

fn write_wav_f32(path: &Path, t: &TrackAudio, rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for &s in &t.samples {
        w.write_sample(s)?;
    }
    w.finalize()?;
    Ok(())
}

/// Lee un WAV estéreo (float o enteros); si es mono, lo duplica.
pub fn read_wav(path: &Path) -> Result<TrackAudio> {
    let mut r = hound::WavReader::open(path)?;
    let spec = r.spec();
    let mut samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()?
        }
    };
    if spec.channels == 1 {
        samples = samples.iter().flat_map(|&s| [s, s]).collect();
    } else if spec.channels > 2 {
        let ch = spec.channels as usize;
        samples = samples
            .chunks_exact(ch)
            .flat_map(|f| [f[0], f[1]])
            .collect();
    }
    Ok(TrackAudio { samples })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
    struct Ajustes {
        tempo: f32,
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mapach-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn guardar_y_cargar() {
        let base = tmpdir("song");
        let mut s = Song::new("Pulpo Saltarín".into(), [1, 2, 3], Ajustes { tempo: 99.0 });
        s.vuelta = 4;
        s.volumenes[2] = 0.5;
        s.pistas[2] = Some(TrackAudio {
            samples: vec![0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4],
        });
        let dir = s.save_in(&base).unwrap();
        assert!(dir.join("pistas/3.wav").exists());
        let back: Song<Ajustes> = Song::load(&dir).unwrap();
        assert_eq!(back.nombre, "Pulpo Saltarín");
        assert_eq!(back.ajustes.tempo, 99.0);
        assert_eq!(back.volumenes[2], 0.5);
        assert_eq!(back.pistas[2], s.pistas[2]);
        assert!(back.pistas[0].is_none());

        // Guardar otra vez sobrescribe sin dejar temporales.
        s.pistas[2] = None;
        s.save_in(&base).unwrap();
        let back: Song<Ajustes> = Song::load(&dir).unwrap();
        assert!(back.pistas[2].is_none());

        let list = list_in(&base);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].nombre, "Pulpo Saltarín");
        assert_eq!(list[0].pistas, 0);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn la_lista_ignora_basura() {
        let base = tmpdir("basura");
        std::fs::create_dir_all(base.join("rara")).unwrap();
        std::fs::write(base.join("rara/cancion.ron"), "esto no es ron (").unwrap();
        std::fs::write(base.join("suelto.txt"), "hola").unwrap();
        assert!(list_in(&base).is_empty());
        std::fs::remove_dir_all(base).unwrap();
    }
}
