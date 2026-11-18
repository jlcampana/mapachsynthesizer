//! Carpetas: `~/Música/Mapachsynthesizer/{Canciones,Exportadas,Importar}`.

use std::path::{Path, PathBuf};

/// Carpeta de música del usuario (la de `user-dirs.dirs`, o `~/Música`, o `~/Music`).
pub fn music_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if let Some(dir) = xdg_music_dir(&home) {
        return dir;
    }
    let musica = home.join("Música");
    if musica.is_dir() {
        musica
    } else {
        home.join("Music")
    }
}

/// Lee `XDG_MUSIC_DIR` de `~/.config/user-dirs.dirs`.
fn xdg_music_dir(home: &Path) -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let text = std::fs::read_to_string(config.join("user-dirs.dirs")).ok()?;
    parse_user_dirs(&text, home)
}

fn parse_user_dirs(text: &str, home: &Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("XDG_MUSIC_DIR="))?;
    let value = line.split_once('=')?.1.trim().trim_matches('"');
    let value = value.replace("$HOME", &home.to_string_lossy());
    let p = PathBuf::from(value);
    (p.is_absolute() && p != home).then_some(p)
}

pub fn base_dir() -> PathBuf {
    music_dir().join("Mapachsynthesizer")
}

pub fn songs_dir() -> PathBuf {
    base_dir().join("Canciones")
}

pub fn exports_dir() -> PathBuf {
    base_dir().join("Exportadas")
}

/// Nombre de fichero seguro a partir del nombre de una canción (deja tildes y espacios).
pub fn safe_file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() { "Canción".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_dirs() {
        let home = Path::new("/home/ana");
        let text =
            "# comentario\nXDG_DESKTOP_DIR=\"$HOME/Escritorio\"\nXDG_MUSIC_DIR=\"$HOME/Música\"\n";
        assert_eq!(
            parse_user_dirs(text, home),
            Some(PathBuf::from("/home/ana/Música"))
        );
        // Si apunta a la carpeta personal, no vale.
        assert_eq!(parse_user_dirs("XDG_MUSIC_DIR=\"$HOME/\"", home), None);
    }

    #[test]
    fn nombres_de_fichero() {
        assert_eq!(safe_file_name("Dragón Naranja"), "Dragón Naranja");
        assert_eq!(safe_file_name("a/b:c"), "a_b_c");
        assert_eq!(safe_file_name("   "), "Canción");
    }
}
