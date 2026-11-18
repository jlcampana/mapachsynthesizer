//! Entrada de Mapachsynthesizer: controles lógicos comunes al teclado y al mando.
//!
//! El teclado del ordenador (por `egui`) y el macropad (por `evdev`) se traducen a los mismos
//! [`Control`]s, así el resto de la app no sabe de dónde viene cada pulsación.

pub mod mando;

/// Las 4 flechas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

/// Los 3 botones de menú del HiChord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Menu {
    /// Gris: tonalidad y ajustes.
    Tono,
    /// Amarillo: sonidos y efectos.
    Sonido,
    /// Rojo: modos y tempo.
    Modo,
}

impl Menu {
    pub const ALL: [Menu; 3] = [Menu::Tono, Menu::Sonido, Menu::Modo];
}

/// Las 3 perillas del mando (en el teclado se imitan con otras teclas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Knob {
    /// P1 (izquierda): volumen. Pulsar: silencio.
    Volumen,
    /// P2 (centro): tempo. Pulsar: tap tempo.
    Tempo,
    /// P3 (derecha, la grande): rueda-cursor, como la de un iPod. Pulsar: elegir.
    Cursor,
}

/// Un control que se pulsa y se suelta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Control {
    /// Tecla de acorde 0–6.
    Chord(u8),
    /// Tecla de acorde 0–6, una octava más aguda (fila QWERTY).
    ChordHigh(u8),
    Arrow(Arrow),
    Menu(Menu),
    /// Grabar / armar el looper (Espacio o la tecla 8 del mando).
    Record,
    /// Play / pausa (Intro o la tecla 16 del mando).
    Play,
    /// Pulsar una perilla.
    KnobPress(Knob),
    /// Seleccionar pista 1–6 del looper.
    Track(u8),
    /// Cargar preset 1–4 (mantener = guardar).
    Preset(u8),
    /// Tap tempo.
    Tap,
    /// Deshacer.
    Undo,
    /// Taller de samples.
    Taller,
    /// Pantalla de canciones.
    Canciones,
    /// Modo papá.
    Papa,
    /// Tecla `n` (1–16) del mando, tal cual. La app decide qué hace según el modo
    /// (ver [`mando::key_control`]); en el secuenciador son los 16 pasos.
    Pad(u8),
    /// Paso 0–15 del secuenciador.
    Step(u8),
}

/// Un evento de entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Press(Control),
    Release(Control),
    /// Giro de perilla: positivo a la derecha.
    Turn(Knob, i32),
}
