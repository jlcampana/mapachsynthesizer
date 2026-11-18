//! El macropad de 16 teclas y 3 perillas (USB `514c:8850`), leído con `evdev`.
//!
//! Se abre en exclusiva (`EVIOCGRAB`) para que sus códigos no lleguen al escritorio. Si no se
//! puede abrir (sin permisos o desenchufado), la app sigue con el teclado, y como el mando manda
//! códigos de teclado normales (`F13`…`F24` y teclado numérico), también se puede tocar a través
//! de la ventana. Ver `docs/mando.md`.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use evdev::{Device, EventSummary, KeyCode};

use crate::{Control, Event, Knob, Menu};

pub const VENDOR: u16 = 0x514c;
pub const PRODUCT: u16 = 0x8850;

/// Qué hace cada código que manda el mando (ver la tabla "Códigos grabados" de `docs/mando.md`).
pub enum Mapped {
    Control(Control),
    Turn(Knob, i32),
}

/// A qué control físico del mando corresponde un código.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Target {
    /// Tecla 1–16 (la 1 arriba a la izquierda).
    Key(u8),
    /// Perilla 0–2 girada a la izquierda.
    Left(u8),
    /// Perilla 0–2 girada a la derecha.
    Right(u8),
    /// Perilla 0–2 pulsada.
    Push(u8),
}

impl Target {
    /// Los 25 controles del mando, en el orden del asistente "Aprender mando".
    pub fn all() -> Vec<Target> {
        let mut v: Vec<Target> = (1..=16).map(Target::Key).collect();
        for k in 0..3 {
            v.extend([Target::Right(k), Target::Left(k), Target::Push(k)]);
        }
        v
    }

    /// Perilla física (0 izquierda, 1 centro, 2 derecha) → su papel.
    fn knob(k: u8) -> Knob {
        match k {
            0 => Knob::Volumen,
            1 => Knob::Tempo,
            _ => Knob::Cursor,
        }
    }

    fn mapped(self) -> Mapped {
        match self {
            Target::Key(n) => Mapped::Control(Control::Pad(n)),
            Target::Left(k) => Mapped::Turn(Self::knob(k), -1),
            Target::Right(k) => Mapped::Turn(Self::knob(k), 1),
            Target::Push(k) => Mapped::Control(Control::KnobPress(Self::knob(k))),
        }
    }
}

/// Tabla de códigos de `evdev` → controles del mando. Se puede volver a aprender.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mapping {
    pub codes: Vec<(u16, Target)>,
}

impl Default for Mapping {
    /// Los códigos grabados con `herramientas/mando.py` (ver `docs/mando.md`).
    fn default() -> Self {
        use KeyCode as K;
        let keys = [
            K::KEY_F13,
            K::KEY_F14,
            K::KEY_F15,
            K::KEY_F16,
            K::KEY_F17,
            K::KEY_F18,
            K::KEY_F19,
            K::KEY_F20,
            K::KEY_F21,
            K::KEY_F22,
            K::KEY_F23,
            K::KEY_F24,
            K::KEY_KP1,
            K::KEY_KP2,
            K::KEY_KP3,
            K::KEY_KP4,
        ];
        let mut codes: Vec<(u16, Target)> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.code(), Target::Key(i as u8 + 1)))
            .collect();
        let knobs = [
            (K::KEY_KP5, K::KEY_KP6, K::KEY_KP7),
            (K::KEY_KP8, K::KEY_KP9, K::KEY_KP0),
            (K::KEY_KPSLASH, K::KEY_KPASTERISK, K::KEY_KPMINUS),
        ];
        // Comprobado con el mando: el primer código de cada perilla llega al girar a la
        // derecha y el segundo, a la izquierda.
        for (i, (r, l, p)) in knobs.iter().enumerate() {
            let k = i as u8;
            codes.push((r.code(), Target::Right(k)));
            codes.push((l.code(), Target::Left(k)));
            codes.push((p.code(), Target::Push(k)));
        }
        Self { codes }
    }
}

impl Mapping {
    /// Traduce un evento de tecla (`value`: 1 pulsar, 0 soltar, 2 repetición).
    pub fn translate(&self, code: u16, value: i32) -> Option<Event> {
        let target = self.codes.iter().find(|(c, _)| *c == code)?.1;
        match (target.mapped(), value) {
            (Mapped::Control(c), 1) => Some(Event::Press(c)),
            (Mapped::Control(c), 0) => Some(Event::Release(c)),
            (Mapped::Turn(k, d), 1) => Some(Event::Turn(k, d)),
            _ => None,
        }
    }
}

/// Lo que comparten el hilo del mando y la app.
#[derive(Debug, Default)]
pub struct Shared {
    pub mapping: Mapping,
    /// Aprendiendo: los códigos se mandan crudos, sin traducir.
    pub learning: bool,
}

/// Control lógico de la tecla `n` (1–16) del mando, en modo Tocar.
pub fn key_control(n: u8) -> Option<Control> {
    Some(match n {
        1..=7 => Control::Chord(n - 1),
        8 => Control::Record,
        // 9–12 (donde estaban las flechas): atajos a samples.
        9..=12 => Control::Shortcut(n - 9),
        13 => Control::Menu(Menu::Tono),
        14 => Control::Menu(Menu::Sonido),
        15 => Control::Menu(Menu::Modo),
        16 => Control::Play,
        _ => return None,
    })
}

/// Estado de la conexión con el mando, para enseñarlo en pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Buscando,
    Conectado,
    /// Está, pero no hay permiso para abrirlo (falta la regla udev).
    SinPermiso,
}

/// Lanza un hilo que busca el mando, lo abre en exclusiva y manda sus eventos.
/// Si se desconecta, lo vuelve a buscar cada 2 s. `on_event` se llama tras cada envío
/// (para despertar a la interfaz).
pub fn spawn(
    tx: Sender<Event>,
    status: Sender<Status>,
    raw: Sender<u16>,
    shared: Arc<Mutex<Shared>>,
    on_event: impl Fn() + Send + 'static,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("mando".into())
        .spawn(move || {
            let mut last = None;
            loop {
                let found = open_devices();
                let st = match &found {
                    Ok(v) if !v.is_empty() => Status::Conectado,
                    Ok(_) => Status::Buscando,
                    Err(()) => Status::SinPermiso,
                };
                if last != Some(st) {
                    if status.send(st).is_err() {
                        return;
                    }
                    on_event();
                    last = Some(st);
                }
                if let Ok(devices) = found
                    && !devices.is_empty()
                {
                    if read_all(devices, &tx, &raw, &shared, &on_event).is_err() {
                        return; // La interfaz se ha cerrado.
                    }
                    continue;
                }
                thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("no se pudo crear el hilo del mando")
}

/// Abre todas las interfaces de entrada del mando (teclado y "ratón").
/// `Err` si están pero no hay permiso.
fn open_devices() -> Result<Vec<Device>, ()> {
    let mut devices = Vec::new();
    let mut denied = false;
    if let Ok(dir) = std::fs::read_dir("/dev/input") {
        for entry in dir.flatten() {
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("event"))
            {
                continue;
            }
            match Device::open(&path) {
                Ok(mut d) => {
                    let id = d.input_id();
                    if id.vendor() == VENDOR && id.product() == PRODUCT {
                        if let Err(e) = d.grab() {
                            eprintln!("No se pudo coger el mando en exclusiva: {e}");
                        }
                        devices.push(d);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    // Solo nos importa si es el mando: miramos el id por sysfs.
                    if is_mando_sysfs(&path) {
                        denied = true;
                    }
                }
                Err(_) => {}
            }
        }
    }
    if devices.is_empty() && denied {
        Err(())
    } else {
        Ok(devices)
    }
}

fn is_mando_sysfs(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let base = format!("/sys/class/input/{name}/device/id");
    let read = |f: &str| std::fs::read_to_string(format!("{base}/{f}")).unwrap_or_default();
    read("vendor").trim() == "514c" && read("product").trim() == "8850"
}

/// Lee de todas las interfaces a la vez (con hilos) hasta que alguna falle.
/// `Err` solo si el canal se ha cerrado.
fn read_all(
    devices: Vec<Device>,
    tx: &Sender<Event>,
    raw: &Sender<u16>,
    shared: &Arc<Mutex<Shared>>,
    on_event: &(impl Fn() + Send + 'static),
) -> Result<(), ()> {
    // Los hilos de cada interfaz mandan códigos crudos (código, valor).
    let (itx, irx) = std::sync::mpsc::channel::<Option<(u16, i32)>>();
    for mut dev in devices {
        let itx = itx.clone();
        thread::spawn(move || {
            loop {
                match dev.fetch_events() {
                    Ok(events) => {
                        for ev in events {
                            if let EventSummary::Key(_, code, value) = ev.destructure()
                                && itx.send(Some((code.code(), value))).is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(_) => {
                        let _ = itx.send(None);
                        return;
                    }
                }
            }
        });
    }
    drop(itx);
    for msg in irx {
        let Some((code, value)) = msg else {
            // Una interfaz se ha caído: volvemos a buscar.
            return Ok(());
        };
        let (learning, event) = {
            let sh = shared.lock().map_err(|_| ())?;
            (sh.learning, sh.mapping.translate(code, value))
        };
        if learning {
            if value == 1 {
                raw.send(code).map_err(|_| ())?;
                on_event();
            }
        } else if let Some(e) = event {
            tx.send(e).map_err(|_| ())?;
            on_event();
        }
    }
    Ok(())
}

/// Traduce un evento de tecla con la tabla de serie.
pub fn translate(code: KeyCode, value: i32) -> Option<Event> {
    Mapping::default().translate(code.code(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teclas_del_mando() {
        assert_eq!(
            translate(KeyCode::KEY_F13, 1),
            Some(Event::Press(Control::Pad(1)))
        );
        assert_eq!(
            translate(KeyCode::KEY_F19, 0),
            Some(Event::Release(Control::Pad(7)))
        );
        assert_eq!(
            translate(KeyCode::KEY_KP4, 1),
            Some(Event::Press(Control::Pad(16)))
        );
        assert_eq!(key_control(1), Some(Control::Chord(0)));
        assert_eq!(key_control(13), Some(Control::Menu(Menu::Tono)));
        assert_eq!(key_control(16), Some(Control::Play));
        assert_eq!(translate(KeyCode::KEY_F20, 2), None);
    }

    #[test]
    fn perillas() {
        assert_eq!(
            translate(KeyCode::KEY_KP5, 1),
            Some(Event::Turn(Knob::Volumen, 1))
        );
        // La del centro es el tempo y la de la derecha, la rueda-cursor.
        assert_eq!(
            translate(KeyCode::KEY_KP9, 1),
            Some(Event::Turn(Knob::Tempo, -1))
        );
        assert_eq!(
            translate(KeyCode::KEY_KPSLASH, 1),
            Some(Event::Turn(Knob::Cursor, 1))
        );
        // Soltar después de un clic de giro no hace nada.
        assert_eq!(translate(KeyCode::KEY_KP6, 0), None);
        assert_eq!(
            translate(KeyCode::KEY_KPMINUS, 1),
            Some(Event::Press(Control::KnobPress(Knob::Cursor)))
        );
    }

    #[test]
    fn teclas_con_funcion() {
        for n in 1..=16 {
            assert!(key_control(n).is_some(), "{n}");
        }
        assert_eq!(key_control(9), Some(Control::Shortcut(0)));
        assert_eq!(key_control(12), Some(Control::Shortcut(3)));
        assert!(key_control(17).is_none());
    }

    #[test]
    fn la_tabla_de_serie_tiene_los_25_controles() {
        let m = Mapping::default();
        assert_eq!(m.codes.len(), 25);
        for t in Target::all() {
            assert!(m.codes.iter().any(|(_, x)| *x == t), "{t:?}");
        }
        let mut codes: Vec<u16> = m.codes.iter().map(|c| c.0).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), 25);
    }
}
