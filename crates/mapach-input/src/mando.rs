//! El macropad de 16 teclas y 3 perillas (USB `514c:8850`), leído con `evdev`.
//!
//! Se abre en exclusiva (`EVIOCGRAB`) para que sus códigos no lleguen al escritorio. Si no se
//! puede abrir (sin permisos o desenchufado), la app sigue con el teclado, y como el mando manda
//! códigos de teclado normales (`F13`…`F24` y teclado numérico), también se puede tocar a través
//! de la ventana. Ver `docs/mando.md`.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use evdev::{Device, EventSummary, KeyCode};

use crate::{Arrow, Control, Event, Knob, Menu};

pub const VENDOR: u16 = 0x514c;
pub const PRODUCT: u16 = 0x8850;

/// Qué hace cada código que manda el mando (ver la tabla "Códigos grabados" de `docs/mando.md`).
pub enum Mapped {
    Control(Control),
    Turn(Knob, i32),
}

/// Control lógico de la tecla `n` (1–16) del mando, en modo Tocar.
pub fn key_control(n: u8) -> Option<Control> {
    Some(match n {
        1..=7 => Control::Chord(n - 1),
        8 => Control::Record,
        9 => Control::Arrow(Arrow::Left),
        10 => Control::Arrow(Arrow::Up),
        11 => Control::Arrow(Arrow::Down),
        12 => Control::Arrow(Arrow::Right),
        13 => Control::Menu(Menu::Tono),
        14 => Control::Menu(Menu::Sonido),
        15 => Control::Menu(Menu::Modo),
        16 => Control::Play,
        _ => return None,
    })
}

/// Traduce un código de `evdev` del mando.
pub fn map_code(code: KeyCode) -> Option<Mapped> {
    use KeyCode as K;
    let key = |n| Some(Mapped::Control(Control::Pad(n)));
    match code {
        K::KEY_F13 => key(1),
        K::KEY_F14 => key(2),
        K::KEY_F15 => key(3),
        K::KEY_F16 => key(4),
        K::KEY_F17 => key(5),
        K::KEY_F18 => key(6),
        K::KEY_F19 => key(7),
        K::KEY_F20 => key(8),
        K::KEY_F21 => key(9),
        K::KEY_F22 => key(10),
        K::KEY_F23 => key(11),
        K::KEY_F24 => key(12),
        K::KEY_KP1 => key(13),
        K::KEY_KP2 => key(14),
        K::KEY_KP3 => key(15),
        K::KEY_KP4 => key(16),
        K::KEY_KP5 => Some(Mapped::Turn(Knob::Volumen, -1)),
        K::KEY_KP6 => Some(Mapped::Turn(Knob::Volumen, 1)),
        K::KEY_KP7 => Some(Mapped::Control(Control::KnobPress(Knob::Volumen))),
        K::KEY_KP8 => Some(Mapped::Turn(Knob::Magica, -1)),
        K::KEY_KP9 => Some(Mapped::Turn(Knob::Magica, 1)),
        K::KEY_KP0 => Some(Mapped::Control(Control::KnobPress(Knob::Magica))),
        K::KEY_KPSLASH => Some(Mapped::Turn(Knob::Tempo, -1)),
        K::KEY_KPASTERISK => Some(Mapped::Turn(Knob::Tempo, 1)),
        K::KEY_KPMINUS => Some(Mapped::Control(Control::KnobPress(Knob::Tempo))),
        _ => None,
    }
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
                    if read_all(devices, &tx, &on_event).is_err() {
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
    on_event: &(impl Fn() + Send + 'static),
) -> Result<(), ()> {
    let (itx, irx) = std::sync::mpsc::channel::<Option<Event>>();
    for mut dev in devices {
        let itx = itx.clone();
        thread::spawn(move || {
            loop {
                match dev.fetch_events() {
                    Ok(events) => {
                        for ev in events {
                            if let EventSummary::Key(_, code, value) = ev.destructure()
                                && let Some(e) = translate(code, value)
                                && itx.send(Some(e)).is_err()
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
        match msg {
            Some(e) => {
                tx.send(e).map_err(|_| ())?;
                on_event();
            }
            // Una interfaz se ha caído: volvemos a buscar.
            None => return Ok(()),
        }
    }
    Ok(())
}

/// Traduce un evento de tecla (`value`: 1 pulsar, 0 soltar, 2 repetición).
pub fn translate(code: KeyCode, value: i32) -> Option<Event> {
    match (map_code(code)?, value) {
        (Mapped::Control(c), 1) => Some(Event::Press(c)),
        (Mapped::Control(c), 0) => Some(Event::Release(c)),
        (Mapped::Turn(k, d), 1) => Some(Event::Turn(k, d)),
        _ => None,
    }
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
            translate(KeyCode::KEY_KP6, 1),
            Some(Event::Turn(Knob::Volumen, 1))
        );
        assert_eq!(
            translate(KeyCode::KEY_KP8, 1),
            Some(Event::Turn(Knob::Magica, -1))
        );
        // Soltar después de un clic de giro no hace nada.
        assert_eq!(translate(KeyCode::KEY_KP6, 0), None);
        assert_eq!(
            translate(KeyCode::KEY_KPMINUS, 1),
            Some(Event::Press(Control::KnobPress(Knob::Tempo)))
        );
    }

    #[test]
    fn las_16_teclas_estan_mapeadas() {
        for n in 1..=16 {
            assert!(key_control(n).is_some(), "{n}");
        }
        assert!(key_control(17).is_none());
    }
}
