//! Teclado del ordenador → controles lógicos (ver `docs/plan.md` §2).

use eframe::egui::{Key, Modifiers};
use mapach_input::{Arrow, Control, Event, Knob, Menu};

/// Lo que hace una tecla.
pub enum KeyAction {
    Event(Event),
    /// Abrir o cerrar el modo papá.
    Papa,
}

/// Traduce una tecla de `egui`. `pressed` = pulsada (sin repeticiones).
pub fn translate(key: Key, mods: Modifiers, pressed: bool) -> Option<KeyAction> {
    if key == Key::P && mods.ctrl && mods.alt {
        return pressed.then_some(KeyAction::Papa);
    }
    let ev = |c: Control| {
        Some(KeyAction::Event(if pressed {
            Event::Press(c)
        } else {
            Event::Release(c)
        }))
    };
    let turn = |k: Knob, d: i32| pressed.then_some(KeyAction::Event(Event::Turn(k, d)));
    use Key as K;
    match key {
        K::A => ev(Control::Chord(0)),
        K::S => ev(Control::Chord(1)),
        K::D => ev(Control::Chord(2)),
        K::F => ev(Control::Chord(3)),
        K::G => ev(Control::Chord(4)),
        K::H => ev(Control::Chord(5)),
        K::J => ev(Control::Chord(6)),
        K::Q => ev(Control::ChordHigh(0)),
        K::W => ev(Control::ChordHigh(1)),
        K::E => ev(Control::ChordHigh(2)),
        K::R => ev(Control::ChordHigh(3)),
        K::T => ev(Control::ChordHigh(4)),
        K::Y => ev(Control::ChordHigh(5)),
        K::U => ev(Control::ChordHigh(6)),
        K::ArrowUp => ev(Control::Arrow(Arrow::Up)),
        K::ArrowDown => ev(Control::Arrow(Arrow::Down)),
        K::ArrowLeft => ev(Control::Arrow(Arrow::Left)),
        K::ArrowRight => ev(Control::Arrow(Arrow::Right)),
        K::Z => ev(Control::Menu(Menu::Tono)),
        K::X => ev(Control::Menu(Menu::Sonido)),
        K::C => ev(Control::Menu(Menu::Modo)),
        K::Space => ev(Control::Record),
        K::Enter => ev(Control::Play),
        K::Num1 => ev(Control::Track(0)),
        K::Num2 => ev(Control::Track(1)),
        K::Num3 => ev(Control::Track(2)),
        K::Num4 => ev(Control::Track(3)),
        K::Num5 => ev(Control::Track(4)),
        K::Num6 => ev(Control::Track(5)),
        // 7, 8 e I solo sirven en el secuenciador (pasos 7, 8 y 16).
        K::Num7 => ev(Control::Track(6)),
        K::Num8 => ev(Control::Track(7)),
        K::I => ev(Control::ChordHigh(7)),
        K::F1 => ev(Control::Preset(0)),
        K::F2 => ev(Control::Preset(1)),
        K::F3 => ev(Control::Preset(2)),
        K::F4 => ev(Control::Preset(3)),
        K::F5 => ev(Control::Canciones),
        K::Tab => ev(Control::Taller),
        K::B => ev(Control::Tap),
        // Atajos a samples (en el mando, la fila de las teclas 9–12).
        K::V => ev(Control::Shortcut(0)),
        K::N => ev(Control::Shortcut(1)),
        K::M => ev(Control::Shortcut(2)),
        K::Comma => ev(Control::Shortcut(3)),
        K::Backspace => ev(Control::Undo),
        // Perillas sin hardware: RePág/AvPág = volumen, -/+ = tempo. La mágica va con la rueda
        // del ratón.
        K::PageUp => turn(Knob::Volumen, 1),
        K::PageDown => turn(Knob::Volumen, -1),
        K::Plus | K::Equals => turn(Knob::Tempo, 1),
        K::Minus => turn(Knob::Tempo, -1),
        // El mando, cuando no se ha podido abrir por evdev, llega como teclas normales.
        K::F13 => mando_key(1, pressed),
        K::F14 => mando_key(2, pressed),
        K::F15 => mando_key(3, pressed),
        K::F16 => mando_key(4, pressed),
        K::F17 => mando_key(5, pressed),
        K::F18 => mando_key(6, pressed),
        K::F19 => mando_key(7, pressed),
        K::F20 => mando_key(8, pressed),
        K::F21 => mando_key(9, pressed),
        K::F22 => mando_key(10, pressed),
        K::F23 => mando_key(11, pressed),
        K::F24 => mando_key(12, pressed),
        _ => None,
    }
}

fn mando_key(n: u8, pressed: bool) -> Option<KeyAction> {
    let c = Control::Pad(n);
    Some(KeyAction::Event(if pressed {
        Event::Press(c)
    } else {
        Event::Release(c)
    }))
}
