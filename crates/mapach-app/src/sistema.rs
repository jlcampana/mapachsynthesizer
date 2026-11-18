//! Ajustes del sistema del menú F1: volumen máximo y arranque al encender.

use std::path::PathBuf;

pub const MAX_DB_RANGE: (f32, f32) = (-24.0, -3.0);

/// Lanzador de arranque automático (`~/.config/autostart/mapachsynthesizer.desktop`).
pub fn autostart_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("mapachsynthesizer.desktop"))
}

pub fn autostart_enabled() -> bool {
    autostart_path().is_some_and(|p| p.exists())
}

/// Activa o quita el arranque automático.
pub fn set_autostart(on: bool) -> std::io::Result<()> {
    let Some(path) = autostart_path() else {
        return Ok(());
    };
    if on {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "mapachsynthesizer".into());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(
            &path,
            format!(
                "[Desktop Entry]\nType=Application\nName=Mapachsynthesizer\nExec=\"{exe}\"\n\
                 Icon=mapachsynthesizer\nX-GNOME-Autostart-enabled=true\n"
            ),
        )
    } else if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}
