//! Biblioteca de samples: `~/Música/Mapachsynthesizer/Samples/`.
//!
//! Se leen directamente WAV, MP3, OGG y FLAC: basta con descargar ficheros ahí. (También sigue
//! funcionando la carpeta `Importar/`: lo que se deja allí se convierte a WAV y se mueve aquí.)

use std::path::{Path, PathBuf};

use anyhow::Result;
use mapach_project::paths::{base_dir, safe_file_name};

use crate::decode::{EXTENSIONS, decode_file};
use crate::{RATE, Sample};

pub fn samples_dir() -> PathBuf {
    base_dir().join("Samples")
}

pub fn import_dir() -> PathBuf {
    base_dir().join("Importar")
}

/// Un sample de la biblioteca.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub color: [u8; 3],
}

/// Color fijo según el nombre.
pub fn color_for(name: &str) -> [u8; 3] {
    let h = name.bytes().fold(2_166_136_261u32, |h, b| {
        (h ^ b as u32).wrapping_mul(16_777_619)
    });
    mapach_project::names::COLORS[h as usize % mapach_project::names::COLORS.len()]
}

/// Samples de `dir`, el más nuevo primero.
pub fn list_in(dir: &Path) -> Vec<Entry> {
    let mut v: Vec<(std::time::SystemTime, Entry)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() || !is_audio(&path) {
                return None;
            }
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let time = e.metadata().and_then(|m| m.modified()).ok()?;
            Some((
                time,
                Entry {
                    color: color_for(&name),
                    name,
                    path,
                },
            ))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.name.cmp(&b.1.name)));
    v.into_iter().map(|x| x.1).collect()
}

/// ¿Es un fichero de audio que sabemos leer (por su extensión)?
pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| EXTENSIONS.contains(&x.to_lowercase().as_str()))
}

/// Pone primero los favoritos (por nombre de fichero), sin cambiar el orden de los demás.
pub fn favorites_first(list: &mut [Entry], favorites: &[String]) {
    let is_fav = |e: &Entry| {
        e.path
            .file_name()
            .is_some_and(|f| favorites.iter().any(|x| x.as_str() == f.to_string_lossy()))
    };
    list.sort_by_key(|e| !is_fav(e));
}

/// Ruta donde guardar los cambios de un sample. Los WAV se sobrescriben; un MP3, OGG o FLAC se
/// guarda como WAV nuevo y el original se mueve a `originales/` (nunca se borra).
pub fn editable_path(path: &Path) -> Result<PathBuf> {
    let is_wav = path
        .extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| x.eq_ignore_ascii_case("wav"));
    if is_wav {
        return Ok(path.to_path_buf());
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path
        .file_stem()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Sample".into());
    let mut wav = dir.join(format!("{stem}.wav"));
    let mut n = 2;
    while wav.exists() {
        wav = dir.join(format!("{stem} {n}.wav"));
        n += 1;
    }
    let originals = dir.join("originales");
    std::fs::create_dir_all(&originals)?;
    if let Some(name) = path.file_name() {
        std::fs::rename(path, originals.join(name))?;
    }
    Ok(wav)
}

/// Guarda un sample con un nombre que no pise a otro. Devuelve la ruta.
pub fn save_in(dir: &Path, name: &str, s: &Sample) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let base = safe_file_name(name);
    let mut path = dir.join(format!("{base}.wav"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{base} {n}.wav"));
        n += 1;
    }
    write_wav(&path, s)?;
    Ok(path)
}

/// Sobrescribe un sample (tras destrozarlo), pasando por un temporal.
pub fn overwrite(path: &Path, s: &Sample) -> Result<()> {
    let tmp = path.with_extension("wav.tmp");
    write_wav(&tmp, s)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn write_wav(path: &Path, s: &Sample) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for &v in &s.data {
        w.write_sample(v)?;
    }
    w.finalize()?;
    Ok(())
}

/// Carga un sample de la biblioteca (cualquier WAV vale: se pasa a mono y 48 kHz).
pub fn load(path: &Path) -> Result<Sample> {
    decode_file(path)
}

/// Resultado de importar un fichero.
#[derive(Debug)]
pub struct Imported {
    pub file: String,
    pub result: Result<PathBuf>,
}

/// Convierte lo que haya en `import` y lo mete en `lib`. Los originales van a
/// `import/importados/` (o a `import/no_se_pudo/` si no se pudieron leer).
pub fn import_pending(import: &Path, lib: &Path) -> Vec<Imported> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(import) else {
        return out;
    };
    for e in entries.flatten() {
        let path = e.path();
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| x.to_lowercase());
        if !path.is_file() || !ext.is_some_and(|x| EXTENSIONS.contains(&x.as_str())) {
            continue;
        }
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = path
            .file_stem()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sample".into());
        let result = decode_file(&path).and_then(|mut s| {
            s.tidy();
            if s.is_empty() {
                anyhow::bail!("solo tiene silencio");
            }
            save_in(lib, &name, &s)
        });
        let dest_dir = import.join(if result.is_ok() {
            "importados"
        } else {
            "no_se_pudo"
        });
        let _ = std::fs::create_dir_all(&dest_dir);
        let _ = std::fs::rename(&path, dest_dir.join(&file));
        out.push(Imported { file, result });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mapach-lib-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn tone() -> Sample {
        Sample::new((0..24_000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect())
    }

    #[test]
    fn guardar_listar_y_cargar() {
        let dir = tmp("save");
        let a = save_in(&dir, "Mi voz", &tone()).unwrap();
        let b = save_in(&dir, "Mi voz", &tone()).unwrap();
        assert_ne!(a, b);
        assert!(b.ends_with("Mi voz 2.wav"));
        let list = list_in(&dir);
        assert_eq!(list.len(), 2);
        let s = load(&a).unwrap();
        assert_eq!(s.data.len(), 24_000);
        overwrite(&a, &Sample::new(vec![0.1; 100])).unwrap();
        assert_eq!(load(&a).unwrap().data.len(), 100);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn importar_mueve_los_originales() {
        let import = tmp("imp");
        let lib = tmp("implib");
        // Uno bueno (WAV) y uno malo.
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 22_050,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(import.join("perro.wav"), spec).unwrap();
        for i in 0..22_050 {
            w.write_sample(((i as f32 * 0.1).sin() * 10_000.0) as i16)
                .unwrap();
        }
        w.finalize().unwrap();
        std::fs::write(import.join("roto.mp3"), b"nada").unwrap();
        std::fs::write(import.join("nota.txt"), b"se ignora").unwrap();

        let res = import_pending(&import, &lib);
        assert_eq!(res.len(), 2);
        assert_eq!(res.iter().filter(|r| r.result.is_ok()).count(), 1);
        assert!(lib.join("perro.wav").exists());
        assert!(import.join("importados/perro.wav").exists());
        assert!(import.join("no_se_pudo/roto.mp3").exists());
        assert!(import.join("nota.txt").exists());
        // La segunda vez no hay nada que importar.
        assert!(import_pending(&import, &lib).is_empty());
        std::fs::remove_dir_all(import).unwrap();
        std::fs::remove_dir_all(lib).unwrap();
    }

    #[test]
    fn lee_mp3_y_demas_directamente_y_favoritos_arriba() {
        let dir = tmp("formatos");
        // Un WAV de verdad con otra extensión no vale; con la suya, sí.
        let a = save_in(&dir, "a", &tone()).unwrap();
        std::fs::copy(&a, dir.join("b.WAV")).unwrap();
        std::fs::write(dir.join("nota.txt"), b"no").unwrap();
        std::fs::create_dir_all(dir.join("originales")).unwrap();
        let mut list = list_in(&dir);
        assert_eq!(list.len(), 2);
        favorites_first(&mut list, &["b.WAV".into()]);
        assert_eq!(list[0].name, "b");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn editar_un_mp3_guarda_un_wav_y_conserva_el_original() {
        let dir = tmp("editable");
        let mp3 = dir.join("perro.mp3");
        std::fs::write(&mp3, b"x").unwrap();
        let wav = editable_path(&mp3).unwrap();
        assert_eq!(wav, dir.join("perro.wav"));
        assert!(dir.join("originales/perro.mp3").exists());
        assert!(!mp3.exists());
        let w = dir.join("gato.wav");
        assert_eq!(editable_path(&w).unwrap(), w);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn colores_fijos() {
        assert_eq!(color_for("Mi voz"), color_for("Mi voz"));
    }
}
