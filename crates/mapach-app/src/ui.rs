//! Dibujo de la pantalla principal, al estilo del boceto `docs/bocetos/01-principal.svg`.
//!
//! Todo se dibuja en un lienzo virtual de 960×540 que se escala para llenar la pantalla.

use std::f32::consts::TAU;

use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};
use mapach_engine::{ARP_PATTERNS, RATES, SOUNDS, STRUM_MS, midi_to_freq};
use mapach_input::{Arrow, Menu};
use mapach_theory::{Direction, ModifierMode, NOTE_NAMES};

use crate::state::{Item, PlayMode, State};
use crate::theme::*;

pub const W: f32 = 960.0;
pub const H: f32 = 540.0;

/// Lienzo virtual escalado.
pub struct Canvas<'a> {
    pub painter: &'a Painter,
    origin: Pos2,
    scale: f32,
}

impl<'a> Canvas<'a> {
    pub fn new(painter: &'a Painter, screen: Rect) -> Self {
        let scale = (screen.width() / W).min(screen.height() / H);
        let size = Vec2::new(W, H) * scale;
        let origin = screen.center() - size / 2.0;
        Self {
            painter,
            origin,
            scale,
        }
    }

    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.scale
    }

    pub fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), Vec2::new(w, h) * self.scale)
    }

    /// Pasa de coordenadas de pantalla al lienzo.
    pub fn to_canvas(&self, pos: Pos2) -> Pos2 {
        ((pos - self.origin) / self.scale).to_pos2()
    }

    fn s(&self, v: f32) -> f32 {
        v * self.scale
    }

    fn font(&self, size: f32) -> FontId {
        FontId::proportional(self.s(size))
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, radius: f32, fill: Color32) {
        self.painter
            .rect_filled(self.r(x, y, w, h), self.s(radius).min(255.0), fill);
    }

    pub fn rect_stroke(&self, x: f32, y: f32, w: f32, h: f32, radius: f32, width: f32, c: Color32) {
        self.painter.rect_stroke(
            self.r(x, y, w, h),
            self.s(radius).min(255.0),
            Stroke::new(self.s(width), c),
            StrokeKind::Inside,
        );
    }

    pub fn circle(&self, x: f32, y: f32, radius: f32, fill: Color32) {
        self.painter.circle_filled(self.p(x, y), self.s(radius), fill);
    }

    pub fn ring(&self, x: f32, y: f32, radius: f32, width: f32, c: Color32) {
        self.painter
            .circle_stroke(self.p(x, y), self.s(radius), Stroke::new(self.s(width), c));
    }

    pub fn text(&self, x: f32, y: f32, align: Align2, text: impl ToString, size: f32, c: Color32) {
        self.painter
            .text(self.p(x, y), align, text, self.font(size), c);
    }

    pub fn line(&self, pts: &[(f32, f32)], width: f32, c: Color32) {
        let pts = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.line(pts, Stroke::new(self.s(width), c));
    }
}

/// Zonas pulsables con el ratón o el dedo.
pub fn chord_button_rect(b: usize) -> Rect {
    let x = 36.0 + b as f32 * 130.0;
    Rect::from_min_size(Pos2::new(x, 390.0), Vec2::new(120.0, 126.0))
}

/// Información extra para la pantalla.
pub struct Extra<'a> {
    pub time: f64,
    pub peak: f32,
    pub status: &'a str,
}

pub fn draw(c: &Canvas, st: &State, extra: &Extra) {
    // Carcasa.
    c.painter.rect_filled(c.painter.clip_rect(), 0.0, NARANJA);
    c.rect(12.0, 12.0, 936.0, 516.0, 36.0, AMARILLO);
    c.rect(12.0, 12.0, 936.0, 258.0, 36.0, AMARILLO_CLARO);
    c.rect(12.0, 200.0, 936.0, 120.0, 0.0, AMARILLO_CLARO.lerp_to_gamma(AMARILLO, 0.5));

    top_bar(c, st);
    menu_buttons(c, st);
    screen(c, st, extra);
    joystick(c, st);
    chord_buttons(c, st);

    if !extra.status.is_empty() {
        c.text(940.0, 534.0, Align2::RIGHT_BOTTOM, extra.status, 9.0, MARRON.gamma_multiply(0.6));
    }
}

fn pill(c: &Canvas, x: f32, w: f32, fill: Color32, text: &str, tc: Color32) {
    c.rect(x, 30.0, w, 40.0, 20.0, fill);
    c.text(x + w / 2.0, 50.0, Align2::CENTER_CENTER, text, 19.0, tc);
}

fn top_bar(c: &Canvas, st: &State) {
    pill(c, 36.0, 150.0, MENU_ROJO, st.play_mode.badge(), BLANCO);

    let oct = format!("{:+}", st.key.octave).replace("+0", "0");
    let tono = if st.show_names {
        format!("{} · oct {oct}", st.key.name())
    } else {
        format!("   {} · oct {oct}", st.key.scale.name())
    };
    pill(c, 196.0, 200.0, MENU_GRIS, &tono, BLANCO);
    if !st.show_names {
        c.circle(216.0, 50.0, 11.0, color_tonalidad(st.key.tonic));
        c.ring(216.0, 50.0, 11.0, 2.0, BLANCO);
    }

    let s = &SOUNDS[st.sound];
    c.rect(406.0, 30.0, 200.0, 40.0, 20.0, MENU_AMARILLO);
    c.rect_stroke(406.0, 30.0, 200.0, 40.0, 20.0, 3.0, hex(0xE0A800));
    c.text(506.0, 50.0, Align2::CENTER_CENTER, format!("{} {}", s.icon, s.name), 19.0, MARRON);

    pill(c, 616.0, 120.0, BLANCO, &format!("♩ {:.0} BPM", st.bpm), NARANJA);

    // Volumen: 8 segmentos.
    c.rect(746.0, 34.0, 178.0, 32.0, 16.0, hex(0xE8A600));
    let lit = if st.muted {
        0
    } else {
        (st.volume * 8.0).round() as usize
    };
    for i in 0..8 {
        let col = match i {
            _ if i >= lit => hex(0xF5DFA0),
            0..=3 => hex(0x2ED573),
            4..=5 => hex(0xFFD400),
            _ => hex(0xFF8A00),
        };
        c.rect(754.0 + i as f32 * 15.0, 40.0, 12.0, 20.0, 4.0, col);
    }
    let vol = if st.muted { "🔇" } else { "VOL" };
    c.text(898.0, 50.0, Align2::CENTER_CENTER, vol, 13.0, MARRON);
}

fn menu_buttons(c: &Canvas, st: &State) {
    let held = st.menu_held();
    let open = st.open_menu.map(|o| o.menu);
    for (i, (m, letter, label, col, tc)) in [
        (Menu::Tono, "Z", "Tono", MENU_GRIS, BLANCO),
        (Menu::Sonido, "X", "Sonido", MENU_AMARILLO, MARRON),
        (Menu::Modo, "C", "Modo", MENU_ROJO, BLANCO),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 140.0 + i as f32 * 90.0;
        let active = held == Some(m) || open == Some(m);
        let r = if active { 42.0 } else { 38.0 };
        c.circle(80.0, y, r + 4.0, col.gamma_multiply(0.7));
        c.circle(80.0, y, r, col);
        if active {
            c.ring(80.0, y, r + 4.0, 4.0, BLANCO);
        }
        c.text(80.0, y - 10.0, Align2::CENTER_CENTER, letter, 22.0, tc);
        c.text(80.0, y + 14.0, Align2::CENTER_CENTER, label, 15.0, tc);
    }
}

/// Pantalla "OLED" central.
fn screen(c: &Canvas, st: &State, extra: &Extra) {
    c.rect(146.0, 86.0, 568.0, 278.0, 28.0, NARANJA);
    c.rect(154.0, 94.0, 552.0, 262.0, 22.0, OLED);

    let menu = st.menu_held().map(|m| (m, None)).or(st.open_menu.map(|o| (o.menu, Some(o.item))));
    if let Some((m, sel)) = menu {
        menu_page(c, st, m, sel);
    } else if st.play_mode == PlayMode::Melodia {
        melody_view(c, st);
    } else if let Some(s) = &st.current {
        let col = ACORDES[s.button as usize];
        if st.show_names {
            c.text(430.0, 165.0, Align2::CENTER_CENTER, s.chord.name(), 84.0, NEON_AMARILLO);
        } else {
            c.circle(430.0, 160.0, 50.0, col);
            c.ring(430.0, 160.0, 56.0, 4.0, col.gamma_multiply(0.5));
            c.text(430.0, 160.0, Align2::CENTER_CENTER, s.button + 1, 56.0, texto_sobre(col));
        }
        let color = match s.dir {
            Some(d) => format!("{}  {}", d.arrow(), dir_label(st.mod_mode, d)),
            None => "normal".into(),
        };
        c.text(430.0, 235.0, Align2::CENTER_CENTER, color, 22.0, NEON_CIAN);
    } else {
        c.text(430.0, 140.0, Align2::CENTER_CENTER, "🦝", 64.0, NEON_AMARILLO);
        c.text(
            430.0,
            215.0,
            Align2::CENTER_CENTER,
            "Pulsa  A S D F G H J",
            30.0,
            NEON_AMARILLO,
        );
        c.text(
            430.0,
            248.0,
            Align2::CENTER_CENTER,
            "Z tono · X sonido · C modo",
            18.0,
            NEON_CIAN,
        );
    }

    scope(c, st, extra);
    mini_piano(c, st);
    // Looper (de momento vacío).
    for i in 0..6 {
        c.ring(596.0 + i as f32 * 18.0, 333.0, 7.0, 2.0, OLED_APAGADO.lerp_to_gamma(BLANCO, 0.2));
    }
}

/// Osciloscopio: dibuja la suma de las notas que suenan (con la amplitud real de la salida).
fn scope(c: &Canvas, st: &State, extra: &Extra) {
    let notes: Vec<i32> = match st.play_mode {
        PlayMode::Melodia => st.melody.iter().map(|&(_, n)| n).collect(),
        _ => st.current.as_ref().map(|s| s.notes.clone()).unwrap_or_default(),
    };
    let amp = (extra.peak * 4.0).clamp(0.0, 1.0);
    let x0 = 176.0;
    let w = 508.0;
    let y0 = 290.0;
    let n = 160;
    let t = extra.time as f32;
    let pts: Vec<(f32, f32)> = (0..=n)
        .map(|i| {
            let u = i as f32 / n as f32;
            let mut v = 0.0;
            for &note in &notes {
                // Frecuencias muy reducidas para que se vea la forma.
                let f = midi_to_freq(note as f32) / 110.0;
                v += (TAU * (f * u * 3.0 + f * t * 0.5)).sin();
            }
            if !notes.is_empty() {
                v /= (notes.len() as f32).sqrt();
            }
            // Bordes suaves.
            let win = (u * TAU / 2.0).sin();
            (x0 + u * w, y0 - v * amp * 16.0 * win)
        })
        .collect();
    let col = if amp > 0.01 { NEON_ROSA } else { NEON_ROSA.gamma_multiply(0.3) };
    c.line(&pts, 3.0, col);
}

fn mini_piano(c: &Canvas, st: &State) {
    let notes: Vec<i32> = match st.play_mode {
        PlayMode::Melodia => st.melody.iter().map(|&(_, n)| n).collect(),
        _ => st.current.as_ref().map(|s| s.notes.clone()).unwrap_or_default(),
    };
    let col = st
        .current
        .as_ref()
        .map(|s| ACORDES[s.button as usize])
        .unwrap_or(NEON_AMARILLO);
    // 3 octavas desde el Do de la nota más grave que suene (o Do2).
    let lo = notes.iter().map(|n| n - n.rem_euclid(12)).min().unwrap_or(36);
    let (x0, y0, kw) = (176.0, 318.0, 19.0);
    let white_x = |n: i32| {
        let oct = (n - lo).div_euclid(12);
        let pos = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6][n.rem_euclid(12) as usize];
        x0 + (oct * 7 + pos) as f32 * kw
    };
    let is_black = |n: i32| matches!(n.rem_euclid(12), 1 | 3 | 6 | 8 | 10);
    let off_white = OLED_APAGADO.lerp_to_gamma(BLANCO, 0.3);
    for n in lo..lo + 36 {
        if !is_black(n) {
            let fill = if notes.contains(&n) { col } else { off_white };
            c.rect(white_x(n), y0, kw - 2.0, 30.0, 3.0, fill);
        }
    }
    for n in lo..lo + 36 {
        if is_black(n) {
            let fill = if notes.contains(&n) { col } else { OLED };
            let x = white_x(n) + kw * 0.62;
            c.rect(x - 1.0, y0 - 1.0, kw * 0.7 + 2.0, 20.0, 3.0, OLED);
            c.rect(x, y0, kw * 0.7, 18.0, 3.0, fill);
        }
    }
}

fn melody_view(c: &Canvas, st: &State) {
    if let Some(&(_, note)) = st.melody.last() {
        let deg = st.key.scale.degrees();
        let rel = (note - st.key.tonic_note()).rem_euclid(12);
        let i = deg.iter().position(|&d| d == rel).unwrap_or(0);
        let col = ACORDES[i % 7];
        c.circle(430.0, 165.0, 52.0, col);
        let label = if st.show_names {
            NOTE_NAMES[note.rem_euclid(12) as usize].to_string()
        } else {
            "♪".to_string()
        };
        c.text(430.0, 165.0, Align2::CENTER_CENTER, label, 44.0, texto_sobre(col));
    } else {
        c.text(430.0, 150.0, Align2::CENTER_CENTER, "♪ Melodía", 52.0, NEON_AMARILLO);
        c.text(
            430.0,
            215.0,
            Align2::CENTER_CENTER,
            "Cada tecla, una nota",
            24.0,
            NEON_CIAN,
        );
    }
}

fn menu_page(c: &Canvas, st: &State, m: Menu, sel: Option<usize>) {
    let (title, col) = match m {
        Menu::Tono => ("TONO", MENU_GRIS),
        Menu::Sonido => ("SONIDO", MENU_AMARILLO),
        Menu::Modo => ("MODO", MENU_ROJO),
    };
    c.rect(174.0, 106.0, 120.0, 30.0, 15.0, col);
    c.text(234.0, 121.0, Align2::CENTER_CENTER, title, 18.0, texto_sobre(col));

    if sel.is_none() {
        // Menú mantenido: chuleta de combinaciones.
        let tips: &[&str] = match m {
            Menu::Tono => &["← →  tonalidad", "↑ ↓  octava", "Espacio  🎲 al azar", "Suelta para ver más"],
            Menu::Sonido => &["← →  sonido", "↑ ↓  brillo", "Espacio  🎲 al azar", "Suelta para ver más"],
            Menu::Modo => &["← →  modo", "↑ ↓  tempo", "Espacio  🎲 al azar", "Suelta para ver más"],
        };
        for (i, t) in tips.iter().enumerate() {
            c.text(430.0, 160.0 + i as f32 * 32.0, Align2::CENTER_CENTER, t, 22.0, NEON_CIAN);
        }
        let big = match m {
            Menu::Tono => value_text(st, Item::Tonalidad),
            Menu::Sonido => value_text(st, Item::Sonido),
            Menu::Modo => value_text(st, Item::Modo),
        };
        c.text(686.0, 121.0, Align2::RIGHT_CENTER, big, 20.0, NEON_AMARILLO);
        return;
    }
    let sel = sel.unwrap_or(0);
    let items = st.items(m);
    let sel = sel.min(items.len().saturating_sub(1));
    for (i, &item) in items.iter().enumerate() {
        let y = 156.0 + i as f32 * 28.0;
        let on = i == sel;
        if on {
            c.rect(174.0, y - 13.0, 512.0, 26.0, 13.0, OLED_APAGADO);
        }
        let tc = if on { NEON_AMARILLO } else { NEON_CIAN.gamma_multiply(0.7) };
        c.text(190.0, y, Align2::LEFT_CENTER, item.label(), 19.0, tc);
        let v = value_text(st, item);
        let v = if on { format!("◀  {v}  ▶") } else { v };
        c.text(670.0, y, Align2::RIGHT_CENTER, v, 19.0, tc);
    }
    c.text(
        430.0,
        124.0,
        Align2::LEFT_CENTER,
        "↑↓ elegir · ←→ cambiar",
        13.0,
        NEON_CIAN.gamma_multiply(0.6),
    );
}

fn value_text(st: &State, item: Item) -> String {
    let si_no = |b| if b { "sí" } else { "no" }.to_string();
    match item {
        Item::Tonalidad if st.show_names => NOTE_NAMES[st.key.tonic as usize].to_string(),
        Item::Tonalidad => format!("{}", st.key.tonic + 1),
        Item::Escala => st.key.scale.name().to_string(),
        Item::Octava => format!("{:+}", st.key.octave),
        Item::Acordes => st.mod_mode.name().to_string(),
        Item::Bajo => si_no(st.bass),
        Item::Nombres => si_no(st.show_names),
        Item::Sonido => format!("{} {}", SOUNDS[st.sound].icon, SOUNDS[st.sound].name),
        Item::Ataque => format!("{:.0} ms", st.attack * 1000.0),
        Item::Release => format!("{:.0} ms", st.release * 1000.0),
        Item::Brillo => format!("{:.0} %", st.brightness * 100.0),
        Item::Modo => st.play_mode.name().to_string(),
        Item::Tempo => format!("{:.0} BPM", st.bpm),
        Item::Patron => ARP_PATTERNS[st.arp_pattern].0.to_string(),
        Item::Figura if st.play_mode == PlayMode::Repetir => RATES[st.repeat_rate].0.to_string(),
        Item::Figura => RATES[st.arp_rate].0.to_string(),
        Item::Velocidad => STRUM_MS[st.strum_speed].0.to_string(),
    }
}

/// Nombre corto del "color" que da cada flecha en cada tabla.
pub fn dir_label(mode: ModifierMode, d: Direction) -> &'static str {
    use Direction::*;
    match mode {
        ModifierMode::Basic => match d {
            Up => "may/men",
            UpRight => "dom7",
            Right => "7ª",
            DownRight => "9ª",
            Down => "sus4",
            DownLeft => "6ª",
            Left => "dim",
            UpLeft => "aum",
        },
        ModifierMode::Jazz => match d {
            Up => "may/men",
            UpRight => "9",
            Right => "add11",
            DownRight => "m11",
            Down => "7♯9",
            DownLeft => "add9",
            Left => "7sus4",
            UpLeft => "ø7",
        },
        ModifierMode::Borrow => match d {
            Up => "dom. sec.",
            UpRight => "backdoor",
            Right => "dim7",
            DownRight => "ø7",
            Down => "paralelo",
            DownLeft => "7ª prest.",
            Left => "napolit.",
            UpLeft => "tritono",
        },
    }
}

/// Rueda de flechas (joystick).
fn joystick(c: &Canvas, st: &State) {
    let (cx, cy) = (830.0, 215.0);
    c.circle(cx, cy, 94.0, NARANJA);
    c.circle(cx, cy, 88.0, hex(0xFFF1C9));
    let active = st.current.as_ref().and_then(|s| s.dir);
    let arrows = [
        (Direction::Up, 0.0, -1.0),
        (Direction::UpRight, 0.72, -0.72),
        (Direction::Right, 1.0, 0.0),
        (Direction::DownRight, 0.72, 0.72),
        (Direction::Down, 0.0, 1.0),
        (Direction::DownLeft, -0.72, 0.72),
        (Direction::Left, -1.0, 0.0),
        (Direction::UpLeft, -0.72, -0.72),
    ];
    for (d, dx, dy) in arrows {
        let (x, y) = (cx + dx * 62.0, cy + dy * 62.0);
        let menu = st.open_menu.is_some() || st.menu_held().is_some();
        let on = !menu
            && (active == Some(d) || (st.current.is_none() && st.arrow_dir() == Some(d)));
        if on {
            c.circle(x, y, 22.0, ACORDES[st.current.as_ref().map_or(0, |s| s.button as usize)]);
        }
        let label = format!("{}\n{}", d.arrow(), dir_label(st.mod_mode, d));
        c.text(x, y, Align2::CENTER_CENTER, label, 12.0, if on { BLANCO } else { MARRON });
    }
    let held = [Arrow::Up, Arrow::Down, Arrow::Left, Arrow::Right]
        .iter()
        .any(|&a| st.arrow_held(a));
    c.circle(cx, cy, 26.0, if held { MENU_ROJO } else { NARANJA });
    c.text(cx, cy, Align2::CENTER_CENTER, "flechas", 11.0, BLANCO);
    c.text(cx, 330.0, Align2::CENTER_CENTER, "Mantén un acorde", 15.0, MARRON);
    c.text(cx, 349.0, Align2::CENTER_CENTER, "y pulsa una flecha", 15.0, MARRON);
}

const LETRAS: [&str; 7] = ["A", "S", "D", "F", "G", "H", "J"];
const ROMANOS: [&str; 7] = ["I", "ii", "iii", "IV", "V", "vi", "vii"];

fn chord_buttons(c: &Canvas, st: &State) {
    for b in 0..7usize {
        let r = chord_button_rect(b);
        let col = ACORDES[b];
        let pressed = st.chord_held(b as u8);
        let lift = if pressed { 6.0 } else { 0.0 };
        let (x, y, w, h) = (r.min.x, r.min.y - lift, r.width(), r.height());
        if pressed {
            c.rect(x - 6.0, y - 6.0, w + 12.0, h + 12.0, 26.0, BLANCO);
        }
        c.rect(x, y, w, h, 22.0, col.gamma_multiply(0.75));
        c.rect(x + 4.0, y + 4.0, w - 8.0, h - 10.0, 18.0, col);
        let tc = texto_sobre(col);
        let cx = x + w / 2.0;
        if st.play_mode == PlayMode::Melodia {
            c.text(cx, y + 48.0, Align2::CENTER_CENTER, "♪", 40.0, tc);
        } else if st.show_names {
            let name = st.chord_for(b as u8, false).name();
            let size = if name.chars().count() > 5 { 24.0 } else { 34.0 };
            c.text(cx, y + 42.0, Align2::CENTER_CENTER, name, size, tc);
            c.text(cx, y + 78.0, Align2::CENTER_CENTER, ROMANOS[b], 18.0, tc.gamma_multiply(0.8));
        } else {
            c.text(cx, y + 52.0, Align2::CENTER_CENTER, b + 1, 52.0, tc);
        }
        c.text(cx, y + 104.0, Align2::CENTER_CENTER, format!("[{}]", LETRAS[b]), 17.0, tc.gamma_multiply(0.8));
    }
}

/// Pantalla del modo papá.
pub fn draw_papa(c: &Canvas, lines: &[String]) {
    c.painter
        .rect_filled(c.painter.clip_rect(), 0.0, Color32::from_black_alpha(200));
    c.rect(180.0, 80.0, 600.0, 380.0, 28.0, OLED);
    c.rect_stroke(180.0, 80.0, 600.0, 380.0, 28.0, 4.0, NARANJA);
    c.text(480.0, 120.0, Align2::CENTER_CENTER, "🔧 Modo papá", 32.0, NEON_AMARILLO);
    for (i, l) in lines.iter().enumerate() {
        c.text(220.0, 170.0 + i as f32 * 30.0, Align2::LEFT_CENTER, l, 19.0, NEON_CIAN);
    }
    c.text(
        480.0,
        430.0,
        Align2::CENTER_CENTER,
        "Q salir del programa · Esc volver",
        20.0,
        NEON_AMARILLO,
    );
}

/// Para los tests y el ratón: qué tecla de acorde hay en un punto del lienzo.
pub fn chord_at(p: Pos2) -> Option<u8> {
    (0..7).find(|&b| chord_button_rect(b).contains(p)).map(|b| b as u8)
}
