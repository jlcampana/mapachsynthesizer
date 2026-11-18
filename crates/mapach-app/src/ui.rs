//! Dibujo de la pantalla principal, al estilo del boceto `docs/bocetos/01-principal.svg`.
//!
//! Todo se dibuja en un lienzo virtual de 960×540 que se escala para llenar la pantalla.

use std::f32::consts::TAU;

use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};
use mapach_dsp::drums::{Drum, KITS};
use mapach_engine::drums::{STYLES, VARIATIONS};
use mapach_engine::looper::TRACKS;
use mapach_engine::{ARP_PATTERNS, RATES, SOUNDS, STRUM_MS, midi_to_freq};
use mapach_input::{Arrow, Menu};
use mapach_theory::{Direction, ModifierMode, NOTE_NAMES};

use crate::state::{Item, PlayMode, Screen, SongFocus, State};
use crate::theme::*;
use mapach_project::SongInfo;

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
        self.painter
            .circle_filled(self.p(x, y), self.s(radius), fill);
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
    /// Paso de la caja de ritmos que suena.
    pub drum_step: Option<usize>,
    pub songs: Option<SongsView<'a>>,
    pub taller: Option<TallerView<'a>>,
}

/// Lo que hace falta para dibujar el taller.
pub struct TallerView<'a> {
    pub list: &'a [mapach_samples::library::Entry],
    pub overview: &'a [f32],
    pub root: Option<f32>,
    pub recording: bool,
    pub rec_level: f32,
    pub rec_seconds: f32,
    pub seconds: f32,
}

/// Lo que hace falta para dibujar la pantalla de canciones.
pub struct SongsView<'a> {
    pub current: &'a str,
    pub current_color: [u8; 3],
    pub list: &'a [SongInfo],
    pub busy: Option<&'a str>,
    /// Segundos desde el último guardado.
    pub saved_ago: Option<f64>,
    pub last_export: Option<&'a std::path::Path>,
}

pub fn draw(c: &Canvas, st: &State, extra: &Extra) {
    // Carcasa.
    c.painter.rect_filled(c.painter.clip_rect(), 0.0, NARANJA);
    c.rect(12.0, 12.0, 936.0, 516.0, 36.0, AMARILLO);
    c.rect(12.0, 12.0, 936.0, 258.0, 36.0, AMARILLO_CLARO);
    c.rect(
        12.0,
        200.0,
        936.0,
        120.0,
        0.0,
        AMARILLO_CLARO.lerp_to_gamma(AMARILLO, 0.5),
    );

    if st.screen == Screen::Canciones {
        songs_screen(c, st, extra);
    } else if st.screen == Screen::Taller {
        taller_screen(c, st, extra);
    } else {
        top_bar(c, st);
        menu_buttons(c, st);
        screen(c, st, extra);
        joystick(c, st);
        chord_buttons(c, st);
    }
    if let Some(t) = st.toast_text() {
        let w = 40.0 + t.chars().count() as f32 * 11.0;
        c.rect(480.0 - w / 2.0, 76.0, w, 40.0, 20.0, hex(0x2ED573));
        c.rect_stroke(480.0 - w / 2.0, 76.0, w, 40.0, 20.0, 3.0, BLANCO);
        c.text(480.0, 96.0, Align2::CENTER_CENTER, t, 19.0, BLANCO);
    }

    if !extra.status.is_empty() {
        c.text(
            940.0,
            534.0,
            Align2::RIGHT_BOTTOM,
            extra.status,
            9.0,
            MARRON.gamma_multiply(0.6),
        );
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
    c.text(
        506.0,
        50.0,
        Align2::CENTER_CENTER,
        if st.effect == mapach_dsp::effects::Effect::Ninguno {
            format!("{} {}", s.icon, s.name)
        } else {
            format!("{} {} · {}", s.icon, s.name, st.effect.icon())
        },
        19.0,
        MARRON,
    );

    pill(
        c,
        616.0,
        120.0,
        BLANCO,
        &format!("♩ {:.0} BPM", st.bpm),
        NARANJA,
    );

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

    let menu = st
        .menu_held()
        .map(|m| (m, None))
        .or(st.open_menu.map(|o| (o.menu, Some(o.item))));
    if let Some((m, sel)) = menu {
        menu_page(c, st, m, sel);
    } else if let Some(g) = &st.game {
        game_view(c, g, extra, st);
        stars_line(c, st);
    } else if let Some(sm) = &st.simon {
        simon_view(c, sm, st);
        stars_line(c, st);
    } else if let Some(f) = &st.follow {
        follow_view(c, f, st);
        stars_line(c, st);
    } else if st.looper_visible() {
        looper_view(c, st, extra);
        return;
    } else if st.play_mode == PlayMode::Secuencia {
        sequencer_view(c, st, extra);
        return;
    } else if st.play_mode == PlayMode::Bateria
        || (st.play_mode == PlayMode::Ritmos && st.current.is_none())
    {
        drums_view(c, st, extra);
    } else if st.play_mode == PlayMode::Melodia {
        melody_view(c, st);
    } else if let Some(s) = &st.current {
        let col = button_color(st, s.button);
        if st.show_names {
            c.text(
                430.0,
                165.0,
                Align2::CENTER_CENTER,
                s.chord.name(),
                84.0,
                NEON_AMARILLO,
            );
        } else {
            c.circle(430.0, 160.0, 50.0, col);
            c.ring(430.0, 160.0, 56.0, 4.0, col.gamma_multiply(0.5));
            c.text(
                430.0,
                160.0,
                Align2::CENTER_CENTER,
                s.button + 1,
                56.0,
                texto_sobre(col),
            );
        }
        let color = match s.dir {
            Some(d) => format!("{}  {}", d.arrow(), dir_label(st.mod_mode, d)),
            None => "normal".into(),
        };
        c.text(430.0, 235.0, Align2::CENTER_CENTER, color, 22.0, NEON_CIAN);
    } else {
        c.text(
            430.0,
            140.0,
            Align2::CENTER_CENTER,
            "🦝",
            64.0,
            NEON_AMARILLO,
        );
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
    looper_dots(c, st, extra);
}

/// Los 6 circulitos del looper, abajo a la derecha de la pantalla.
fn looper_dots(c: &Canvas, st: &State, extra: &Extra) {
    let lp = &st.looper;
    for (i, &col) in ACORDES.iter().enumerate().take(TRACKS) {
        let (x, y) = (596.0 + i as f32 * 18.0, 333.0);
        if lp.recording == Some(i) {
            let blink = (extra.time * 4.0).sin() > 0.0;
            c.circle(x, y, 7.0, if blink { MENU_ROJO } else { OLED_APAGADO });
        } else if lp.has(i) {
            c.circle(x, y, 7.0, col);
        } else {
            c.ring(x, y, 7.0, 2.0, OLED_APAGADO.lerp_to_gamma(BLANCO, 0.2));
        }
        if i == st.track {
            c.ring(x, y, 9.5, 1.5, BLANCO);
        }
    }
}

/// Pantalla del looper (boceto 03-looper.svg, en pequeño).
fn looper_view(c: &Canvas, st: &State, extra: &Extra) {
    let lp = &st.looper;
    let (x0, x1) = (250.0, 690.0);
    let title = if lp.length > 0.0 {
        format!("● LOOPER · {:.1} s", lp.length)
    } else {
        "● LOOPER".to_string()
    };
    c.text(176.0, 112.0, Align2::LEFT_CENTER, title, 18.0, MENU_ROJO);
    c.text(
        690.0,
        112.0,
        Align2::RIGHT_CENTER,
        "1–6 pista · Espacio grabar · Intro ▶",
        13.0,
        NEON_CIAN.gamma_multiply(0.7),
    );
    for (i, &col) in ACORDES.iter().enumerate().take(TRACKS) {
        let y = 140.0 + i as f32 * 35.0;
        let sel = i == st.track;
        let rec = lp.recording == Some(i);
        // Número de pista.
        if lp.has(i) || rec {
            c.circle(186.0, y, 14.0, col);
        } else {
            c.ring(186.0, y, 13.0, 3.0, col.gamma_multiply(0.7));
        }
        if sel {
            c.ring(186.0, y, 17.0, 2.5, BLANCO);
        }
        let tc = if lp.has(i) { texto_sobre(col) } else { col };
        c.text(186.0, y, Align2::CENTER_CENTER, i + 1, 16.0, tc);
        // Volumen.
        let vol = st.track_volume[i];
        for k in 0..5 {
            let on = vol > k as f32 * 0.2 + 0.01;
            c.rect(
                208.0 + k as f32 * 7.0,
                y - 4.0,
                5.0,
                8.0,
                1.0,
                if on { NEON_CIAN } else { OLED_APAGADO },
            );
        }
        // Barra de la pista.
        let w = x1 - x0;
        if rec {
            let filled = if lp.length > 0.0 { w } else { w * lp.pos };
            c.rect(x0, y - 13.0, filled.max(4.0), 26.0, 8.0, col);
            c.rect_stroke(x0, y - 13.0, w, 26.0, 8.0, 2.0, MENU_ROJO);
            c.text(x0 + 8.0, y, Align2::LEFT_CENTER, "● REC", 13.0, BLANCO);
        } else if lp.has(i) {
            let muted = lp.muted & (1 << i) != 0;
            c.rect(
                x0,
                y - 13.0,
                w,
                26.0,
                8.0,
                col.gamma_multiply(if muted { 0.3 } else { 1.0 }),
            );
            // Rayitas decorativas, como una forma de onda.
            for k in 0..40 {
                let x = x0 + 6.0 + k as f32 * (w - 12.0) / 40.0;
                let h = 4.0 + ((k * 7 + i * 3) % 9) as f32 * 1.2;
                c.rect(x, y - h / 2.0, 2.0, h, 1.0, BLANCO.gamma_multiply(0.5));
            }
        } else {
            c.rect_stroke(
                x0,
                y - 13.0,
                w,
                26.0,
                8.0,
                1.5,
                OLED_APAGADO.lerp_to_gamma(BLANCO, 0.15),
            );
            if sel {
                c.text(
                    (x0 + x1) / 2.0,
                    y,
                    Align2::CENTER_CENTER,
                    "vacía · Espacio para grabar",
                    13.0,
                    NEON_CIAN.gamma_multiply(0.6),
                );
            }
        }
        // Barra de "mantén para borrar".
        if let Some((t, since)) = st.track_hold
            && t == i
            && since.is_finite()
        {
            let f = ((extra.time - since) / State::HOLD_TO_CLEAR).clamp(0.0, 1.0) as f32;
            c.rect(x0, y + 10.0, w * f, 4.0, 2.0, MENU_ROJO);
            if f > 0.15 {
                c.text(x1, y, Align2::RIGHT_CENTER, "🗑 borrando…", 13.0, BLANCO);
            }
        }
    }
    // Cabeza de reproducción.
    if lp.length > 0.0 && lp.playing {
        let x = x0 + (x1 - x0) * lp.pos;
        c.line(&[(x, 124.0), (x, 342.0)], 3.0, BLANCO);
    }
}

/// Osciloscopio: dibuja la suma de las notas que suenan (con la amplitud real de la salida).
fn scope(c: &Canvas, st: &State, extra: &Extra) {
    let notes: Vec<i32> = match st.play_mode {
        PlayMode::Melodia => st.melody.iter().map(|&(_, n)| n).collect(),
        _ => st
            .current
            .as_ref()
            .map(|s| s.notes.clone())
            .unwrap_or_default(),
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
    let col = if amp > 0.01 {
        NEON_ROSA
    } else {
        NEON_ROSA.gamma_multiply(0.3)
    };
    c.line(&pts, 3.0, col);
}

fn mini_piano(c: &Canvas, st: &State) {
    let notes: Vec<i32> = match st.play_mode {
        PlayMode::Melodia => st.melody.iter().map(|&(_, n)| n).collect(),
        _ => st
            .current
            .as_ref()
            .map(|s| s.notes.clone())
            .unwrap_or_default(),
    };
    let col = st
        .current
        .as_ref()
        .map(|s| button_color(st, s.button))
        .unwrap_or(NEON_AMARILLO);
    // 3 octavas desde el Do de la nota más grave que suene (o Do2).
    let lo = notes
        .iter()
        .map(|n| n - n.rem_euclid(12))
        .min()
        .unwrap_or(36);
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
        c.text(
            430.0,
            165.0,
            Align2::CENTER_CENTER,
            label,
            44.0,
            texto_sobre(col),
        );
    } else {
        c.text(
            430.0,
            150.0,
            Align2::CENTER_CENTER,
            "♪ Melodía",
            52.0,
            NEON_AMARILLO,
        );
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
    c.text(
        234.0,
        121.0,
        Align2::CENTER_CENTER,
        title,
        18.0,
        texto_sobre(col),
    );

    if sel.is_none() {
        // Menú mantenido: chuleta de combinaciones.
        let tips: &[&str] = match m {
            Menu::Tono => &[
                "← →  tonalidad",
                "↑ ↓  octava",
                "Espacio  🎲 al azar",
                "Suelta para ver más",
            ],
            Menu::Sonido => &[
                "← →  sonido",
                "↑ ↓  efecto",
                "Espacio  🎲 al azar",
                "Suelta para ver más",
            ],
            Menu::Modo => &[
                "← →  modo",
                "↑ ↓  tempo",
                "Espacio  🎲 al azar",
                "Suelta para ver más",
            ],
        };
        for (i, t) in tips.iter().enumerate() {
            c.text(
                430.0,
                160.0 + i as f32 * 32.0,
                Align2::CENTER_CENTER,
                t,
                22.0,
                NEON_CIAN,
            );
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
        let tc = if on {
            NEON_AMARILLO
        } else {
            NEON_CIAN.gamma_multiply(0.7)
        };
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
        Item::Kit => format!("{} {}", KITS[st.kit].icon, KITS[st.kit].name),
        Item::Estilo => st.rhythm_name(),
        Item::Variacion => VARIATIONS[st.drum_variation].to_string(),
        Item::Canciones => "▶".to_string(),
        Item::Efecto => format!("{} {}", st.effect.icon(), st.effect.name()),
        Item::Cantidad => format!("{:.0} %", st.effect_amount * 100.0),
        Item::Sala => format!("{:.0} %", st.reverb * 100.0),
        Item::Juego | Item::Copiame | Item::Sigue => "▶".to_string(),
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
        let on =
            !menu && (active == Some(d) || (st.current.is_none() && st.arrow_dir() == Some(d)));
        if on {
            c.circle(
                x,
                y,
                22.0,
                button_color(st, st.current.as_ref().map_or(0, |s| s.button)),
            );
        }
        let label = format!("{}\n{}", d.arrow(), dir_label(st.mod_mode, d));
        c.text(
            x,
            y,
            Align2::CENTER_CENTER,
            label,
            12.0,
            if on { BLANCO } else { MARRON },
        );
    }
    let held = [Arrow::Up, Arrow::Down, Arrow::Left, Arrow::Right]
        .iter()
        .any(|&a| st.arrow_held(a));
    c.circle(cx, cy, 26.0, if held { MENU_ROJO } else { NARANJA });
    c.text(cx, cy, Align2::CENTER_CENTER, "flechas", 11.0, BLANCO);
    c.text(
        cx,
        330.0,
        Align2::CENTER_CENTER,
        "Mantén un acorde",
        15.0,
        MARRON,
    );
    c.text(
        cx,
        349.0,
        Align2::CENTER_CENTER,
        "y pulsa una flecha",
        15.0,
        MARRON,
    );
}

const LETRAS: [&str; 7] = ["A", "S", "D", "F", "G", "H", "J"];
const ROMANOS: [&str; 7] = ["I", "ii", "iii", "IV", "V", "vi", "vii"];

/// Color de una tecla de acorde: arcoíris por tecla o, si se ha elegido, por nota.
pub fn button_color(st: &State, b: u8) -> Color32 {
    if st.note_colors {
        note_color(st.button_pc(b))
    } else {
        ACORDES[b as usize % 7]
    }
}

/// Estrellas ganadas, abajo a la izquierda de la pantalla.
fn stars_line(c: &Canvas, st: &State) {
    c.text(
        176.0,
        112.0,
        Align2::LEFT_CENTER,
        format!("⭐ {}", st.stars),
        16.0,
        NEON_AMARILLO,
    );
}

/// *Cópiame*.
fn simon_view(c: &Canvas, sm: &crate::state::Simon, st: &State) {
    c.text(
        430.0,
        116.0,
        Align2::CENTER_CENTER,
        "🧠 Cópiame",
        26.0,
        NEON_AMARILLO,
    );
    let (text, col) = match (sm.feedback, sm.showing.is_some()) {
        (Some((true, _)), _) => ("¡Muy bien! Una más…", hex(0x2ED573)),
        (Some((false, _)), _) => ("¡Uy! Escucha otra vez", MENU_ROJO),
        (None, true) => ("Escucha y mira…", NEON_CIAN),
        (None, false) => ("¡Te toca!", NEON_AMARILLO),
    };
    c.text(430.0, 170.0, Align2::CENTER_CENTER, text, 26.0, col);
    // Bolitas: una por acorde de la serie; las ya repetidas, encendidas.
    let n = sm.seq.len();
    for i in 0..n {
        let x = 430.0 + (i as f32 - (n as f32 - 1.0) / 2.0) * 26.0;
        let on = sm.showing.is_none() && i < sm.pos;
        let fill = if on {
            button_color(st, sm.seq[i])
        } else {
            OLED_APAGADO.lerp_to_gamma(BLANCO, 0.25)
        };
        c.circle(x, 222.0, 9.0, fill);
    }
    c.text(
        430.0,
        262.0,
        Align2::CENTER_CENTER,
        format!("Serie de {n} · récord {}", sm.best),
        14.0,
        NEON_CIAN.gamma_multiply(0.7),
    );
}

/// *Sigue la canción*.
fn follow_view(c: &Canvas, f: &crate::state::Follow, st: &State) {
    c.text(
        430.0,
        116.0,
        Align2::CENTER_CENTER,
        format!("🎶 {}", f.title()),
        26.0,
        NEON_AMARILLO,
    );
    if f.done_until.is_some() {
        c.text(
            430.0,
            190.0,
            Align2::CENTER_CENTER,
            "🎉 ¡Canción completa!",
            32.0,
            hex(0x2ED573),
        );
        return;
    }
    // Las próximas notas, de colores; la primera, grande.
    let notes = f.notes();
    for (k, &d) in notes.iter().skip(f.pos).take(9).enumerate() {
        let col = if st.note_colors {
            note_color(st.key.tonic + [0, 2, 4, 5, 7, 9, 11][(d % 7) as usize])
        } else {
            ACORDES[(d % 7) as usize]
        };
        let x = 250.0 + k as f32 * 46.0;
        let r = if k == 0 { 24.0 } else { 15.0 };
        c.circle(x, 186.0, r, col);
        let label = if d >= 7 {
            format!("{}↑", d % 7 + 1)
        } else {
            (d + 1).to_string()
        };
        c.text(
            x,
            186.0,
            Align2::CENTER_CENTER,
            label,
            if k == 0 { 20.0 } else { 13.0 },
            texto_sobre(col),
        );
    }
    let f_done = f.pos as f32 / notes.len() as f32;
    c.rect(250.0, 236.0, 360.0, 10.0, 5.0, OLED_APAGADO);
    c.rect(250.0, 236.0, 360.0 * f_done, 10.0, 5.0, NEON_AMARILLO);
    let hint = if st.highlight().is_some_and(|(_, high)| high) {
        "↑ = la fila de arriba (Q W E R T Y U)"
    } else {
        "Pulsa la tecla que brilla · Intro: desde el principio"
    };
    c.text(
        430.0,
        270.0,
        Align2::CENTER_CENTER,
        hint,
        13.0,
        NEON_CIAN.gamma_multiply(0.7),
    );
}

/// Juego de oído.
fn game_view(c: &Canvas, g: &crate::state::EarGame, extra: &Extra, st: &State) {
    c.text(
        430.0,
        116.0,
        Align2::CENTER_CENTER,
        "🎧 ¿Qué color suena?",
        26.0,
        NEON_AMARILLO,
    );
    match g.feedback {
        Some((true, _)) => {
            c.circle(430.0, 186.0, 46.0, hex(0x2ED573));
            c.text(430.0, 186.0, Align2::CENTER_CENTER, "✔", 54.0, BLANCO);
            c.text(530.0, 186.0, Align2::LEFT_CENTER, "¡Bien!", 30.0, NEON_CIAN);
        }
        Some((false, _)) => {
            c.circle(430.0, 186.0, 46.0, MENU_ROJO);
            c.text(430.0, 186.0, Align2::CENTER_CENTER, "✖", 50.0, BLANCO);
            c.text(
                530.0,
                186.0,
                Align2::LEFT_CENTER,
                "¡Otra vez!",
                26.0,
                NEON_CIAN,
            );
        }
        None => {
            // Las teclas que entran en juego.
            for i in 0..g.level as usize {
                let x = 430.0 + (i as f32 - (g.level as f32 - 1.0) / 2.0) * 44.0;
                let pulse = if g.playing_until.is_some() {
                    4.0 * ((extra.time * 6.0).sin() as f32 * 0.5 + 0.5)
                } else {
                    0.0
                };
                let col = button_color(st, i as u8);
                c.circle(x, 186.0, 17.0 + pulse, col);
                c.text(
                    x,
                    186.0,
                    Align2::CENTER_CENTER,
                    i + 1,
                    16.0,
                    texto_sobre(col),
                );
            }
        }
    }
    let stars = "⭐".repeat((g.score as usize).min(12));
    c.text(
        430.0,
        250.0,
        Align2::CENTER_CENTER,
        stars,
        18.0,
        NEON_AMARILLO,
    );
    c.text(
        430.0,
        276.0,
        Align2::CENTER_CENTER,
        "Intro: otra vez · perilla o menú: salir",
        13.0,
        NEON_CIAN.gamma_multiply(0.7),
    );
}

/// 16 puntos con el paso que suena.
fn step_dots(c: &Canvas, pattern: &[u8; 16], step: Option<usize>, y: f32) {
    for (i, &bits) in pattern.iter().enumerate() {
        let x = 196.0 + i as f32 * 29.5;
        let on = bits != 0;
        let col = if step == Some(i) {
            NEON_AMARILLO
        } else if on {
            NEON_ROSA.gamma_multiply(if i % 4 == 0 { 1.0 } else { 0.6 })
        } else {
            OLED_APAGADO
        };
        let r = if step == Some(i) { 9.0 } else { 7.0 };
        c.circle(x, y, r, col);
    }
}

fn drums_view(c: &Canvas, st: &State, extra: &Extra) {
    let kit = &KITS[st.kit];
    if st.play_mode == PlayMode::Ritmos || st.drums_playing {
        let title = format!(
            "{}  ·  {}",
            st.rhythm_name(),
            if st.drum_style < STYLES.len() {
                VARIATIONS[st.drum_variation]
            } else {
                "16 pasos"
            }
        );
        c.text(
            430.0,
            124.0,
            Align2::CENTER_CENTER,
            title,
            26.0,
            NEON_AMARILLO,
        );
    }
    match st.last_hit {
        Some(b) if st.play_mode == PlayMode::Bateria => {
            let d = Drum::ALL[b as usize];
            let col = ACORDES[b as usize];
            c.circle(430.0, 186.0, 44.0, col);
            c.text(430.0, 186.0, Align2::CENTER_CENTER, d.icon(), 40.0, BLANCO);
            c.text(530.0, 186.0, Align2::LEFT_CENTER, d.name(), 24.0, NEON_CIAN);
        }
        _ => {
            let hint = if st.drums_playing {
                "■ Intro para parar"
            } else {
                "▶ Intro para empezar"
            };
            c.text(430.0, 180.0, Align2::CENTER_CENTER, hint, 22.0, NEON_CIAN);
        }
    }
    c.text(
        686.0,
        110.0,
        Align2::RIGHT_TOP,
        format!("{} {}", kit.icon, kit.name),
        15.0,
        NEON_CIAN.gamma_multiply(0.7),
    );
    if st.play_mode == PlayMode::Ritmos || st.drums_playing {
        step_dots(c, &st.drum_pattern(), extra.drum_step, 248.0);
    }
}

/// Rejilla del secuenciador: 7 instrumentos × 16 pasos.
fn sequencer_view(c: &Canvas, st: &State, extra: &Extra) {
    let (x0, y0, cw, rh) = (232.0, 104.0, 26.0, 32.0);
    for (r, d) in Drum::ALL.iter().enumerate() {
        let y = y0 + r as f32 * rh;
        let sel = r == st.seq_row;
        if sel {
            c.rect(162.0, y - 2.0, 536.0, rh - 2.0, 10.0, OLED_APAGADO);
        }
        c.text(176.0, y + 15.0, Align2::LEFT_CENTER, d.icon(), 18.0, BLANCO);
        let col = ACORDES[r];
        for (i, &bits) in st.seq_pattern.iter().enumerate() {
            let x = x0 + i as f32 * cw + (i / 4) as f32 * 4.0;
            let on = bits & (1 << r) != 0;
            let fill = if on {
                col
            } else if extra.drum_step == Some(i) {
                OLED_APAGADO.lerp_to_gamma(BLANCO, 0.35)
            } else {
                OLED_APAGADO.lerp_to_gamma(BLANCO, if i % 4 == 0 { 0.15 } else { 0.07 })
            };
            c.rect(x, y + 3.0, cw - 5.0, rh - 10.0, 6.0, fill);
            if on && extra.drum_step == Some(i) {
                c.rect_stroke(x - 2.0, y + 1.0, cw - 1.0, rh - 6.0, 7.0, 2.0, BLANCO);
            }
        }
    }
    c.text(
        430.0,
        352.0,
        Align2::CENTER_BOTTOM,
        "1–8 y Q–I encienden pasos · ↑↓ instrumento · Intro ▶",
        13.0,
        NEON_CIAN.gamma_multiply(0.7),
    );
}

fn chord_buttons(c: &Canvas, st: &State) {
    for b in 0..7usize {
        let r = chord_button_rect(b);
        let col = if matches!(st.play_mode, PlayMode::Bateria | PlayMode::Secuencia) {
            ACORDES[b]
        } else {
            button_color(st, b as u8)
        };
        let pressed = st.chord_held(b as u8);
        let lift = if pressed { 6.0 } else { 0.0 };
        let (x, y, w, h) = (r.min.x, r.min.y - lift, r.width(), r.height());
        if pressed {
            c.rect(x - 6.0, y - 6.0, w + 12.0, h + 12.0, 26.0, BLANCO);
        }
        if let Some((hb, _)) = st.highlight()
            && hb as usize == b
        {
            let t = (c.painter.ctx().input(|i| i.time) * 5.0).sin() as f32 * 0.5 + 0.5;
            let grow = 8.0 + 4.0 * t;
            c.rect(
                x - grow,
                y - grow,
                w + grow * 2.0,
                h + grow * 2.0,
                30.0,
                NEON_AMARILLO.gamma_multiply(0.7 + 0.3 * t),
            );
            c.text(
                x + w / 2.0,
                y - grow - 10.0,
                Align2::CENTER_BOTTOM,
                "⬇",
                22.0,
                MENU_ROJO,
            );
        }
        c.rect(x, y, w, h, 22.0, col.gamma_multiply(0.75));
        c.rect(x + 4.0, y + 4.0, w - 8.0, h - 10.0, 18.0, col);
        let tc = texto_sobre(col);
        let cx = x + w / 2.0;
        if matches!(st.play_mode, PlayMode::Bateria | PlayMode::Secuencia) {
            let d = Drum::ALL[b];
            let custom = st.drum_samples[b].as_deref().map(|p| {
                std::path::Path::new(p)
                    .file_stem()
                    .map(|f| f.to_string_lossy().chars().take(10).collect::<String>())
                    .unwrap_or_default()
            });
            let (icon, name) = match &custom {
                Some(n) => ("🎤", n.as_str()),
                None => (d.icon(), d.name()),
            };
            c.text(cx, y + 42.0, Align2::CENTER_CENTER, icon, 38.0, tc);
            let size = if name.chars().count() > 8 { 13.0 } else { 17.0 };
            c.text(cx, y + 80.0, Align2::CENTER_CENTER, name, size, tc);
        } else if st.play_mode == PlayMode::Melodia {
            c.text(cx, y + 48.0, Align2::CENTER_CENTER, "♪", 40.0, tc);
        } else if st.show_names {
            let name = st.chord_for(b as u8, false).name();
            let size = if name.chars().count() > 5 { 24.0 } else { 34.0 };
            c.text(cx, y + 42.0, Align2::CENTER_CENTER, name, size, tc);
            c.text(
                cx,
                y + 78.0,
                Align2::CENTER_CENTER,
                ROMANOS[b],
                18.0,
                tc.gamma_multiply(0.8),
            );
        } else {
            c.text(cx, y + 52.0, Align2::CENTER_CENTER, b + 1, 52.0, tc);
        }
        if st.locks[b].is_some() && !matches!(st.play_mode, PlayMode::Bateria | PlayMode::Secuencia)
        {
            c.text(
                x + w - 18.0,
                y + 18.0,
                Align2::CENTER_CENTER,
                "🔒",
                14.0,
                tc,
            );
        }
        c.text(
            cx,
            y + 104.0,
            Align2::CENTER_CENTER,
            format!("[{}]", LETRAS[b]),
            17.0,
            tc.gamma_multiply(0.8),
        );
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Pantalla "Mis canciones" (boceto 05-canciones-exportar.svg).
fn songs_screen(c: &Canvas, st: &State, extra: &Extra) {
    c.rect(36.0, 30.0, 300.0, 48.0, 24.0, hex(0x1EC8FF));
    c.text(
        186.0,
        54.0,
        Align2::CENTER_CENTER,
        "♫ MIS CANCIONES",
        24.0,
        BLANCO,
    );
    let Some(sv) = &extra.songs else {
        c.text(
            480.0,
            270.0,
            Align2::CENTER_CENTER,
            "Sin audio no hay canciones",
            28.0,
            MARRON,
        );
        return;
    };
    // Estado del guardado.
    let (txt, col) = match (sv.busy, sv.saved_ago) {
        (Some(b), _) => (b.to_string(), NARANJA),
        (None, Some(ago)) if ago < 60.0 => (format!("✔ guardado hace {:.0} s", ago), hex(0x2ED573)),
        (None, Some(_)) => ("✔ guardado".to_string(), hex(0x2ED573)),
        (None, None) => ("se guarda solo".to_string(), MENU_GRIS),
    };
    let w = 30.0 + txt.chars().count() as f32 * 9.5;
    c.rect(348.0, 36.0, w, 36.0, 18.0, col);
    c.text(
        348.0 + w / 2.0,
        54.0,
        Align2::CENTER_CENTER,
        txt,
        16.0,
        BLANCO,
    );
    c.text(
        924.0,
        54.0,
        Align2::RIGHT_CENTER,
        "← → elegir · Espacio abrir · ↓ exportar · F5 salir",
        14.0,
        MARRON,
    );

    // Tarjetas: canciones y "Nueva" al final; se ven 4 a la vez.
    let total = sv.list.len() + 1;
    let focus = match st.song_focus {
        SongFocus::Card(i) => Some(i.min(total - 1)),
        SongFocus::Export(_) => None,
    };
    let start = focus
        .map_or(0, |f| f.saturating_sub(3))
        .min(total.saturating_sub(4));
    for slot in 0..4 {
        let i = start + slot;
        if i >= total {
            break;
        }
        let x = 36.0 + slot as f32 * 226.0;
        let (y, w, h) = (96.0, 210.0, 210.0);
        let sel = focus == Some(i);
        if sel {
            c.rect(x - 6.0, y - 6.0, w + 12.0, h + 12.0, 30.0, BLANCO);
        }
        if i == sv.list.len() {
            c.rect(x, y, w, h, 26.0, hex(0xFFD23F));
            c.rect_stroke(x, y, w, h, 26.0, 4.0, NARANJA);
            c.circle(x + w / 2.0, y + 80.0, 38.0, NARANJA);
            c.text(
                x + w / 2.0,
                y + 80.0,
                Align2::CENTER_CENTER,
                "+",
                54.0,
                BLANCO,
            );
            c.text(
                x + w / 2.0,
                y + 160.0,
                Align2::CENTER_CENTER,
                "Nueva",
                26.0,
                NARANJA,
            );
            continue;
        }
        let song = &sv.list[i];
        let col = rgb(song.color);
        c.rect(x, y, w, h, 26.0, col);
        let tc = texto_sobre(col);
        c.text(
            x + w / 2.0,
            y + 78.0,
            Align2::CENTER_CENTER,
            mapach_project::names::icon_for(&song.nombre),
            64.0,
            tc,
        );
        let size = if song.nombre.chars().count() > 16 {
            18.0
        } else {
            23.0
        };
        c.text(
            x + w / 2.0,
            y + 150.0,
            Align2::CENTER_CENTER,
            &song.nombre,
            size,
            tc,
        );
        let pistas = match song.pistas {
            1 => "1 pista".to_string(),
            n => format!("{n} pistas"),
        };
        c.text(
            x + w / 2.0,
            y + 180.0,
            Align2::CENTER_CENTER,
            pistas,
            15.0,
            tc.gamma_multiply(0.85),
        );
        if song.nombre == sv.current {
            c.rect(x + w - 82.0, y + 10.0, 72.0, 24.0, 12.0, BLANCO);
            c.text(
                x + w - 46.0,
                y + 22.0,
                Align2::CENTER_CENTER,
                "★ ahora",
                13.0,
                MARRON,
            );
        }
    }
    if start > 0 {
        c.text(22.0, 200.0, Align2::CENTER_CENTER, "◀", 24.0, MARRON);
    }
    if start + 4 < total {
        c.text(938.0, 200.0, Align2::CENTER_CENTER, "▶", 24.0, MARRON);
    }

    // Exportar.
    c.rect(32.0, 326.0, 896.0, 188.0, 30.0, NARANJA);
    c.rect(40.0, 334.0, 880.0, 172.0, 24.0, OLED);
    c.text(
        66.0,
        362.0,
        Align2::LEFT_CENTER,
        format!("Enviar \"{}\" a la familia:", sv.current),
        22.0,
        NEON_CIAN,
    );
    let buttons = [
        ("MP3", "para el móvil", hex(0xFF3B30)),
        ("OGG", "libre y ligero", hex(0xFF8A00)),
        ("WAV", "máxima calidad", hex(0x3D7BFF)),
    ];
    for (i, (name, sub, col)) in buttons.into_iter().enumerate() {
        let x = 66.0 + i as f32 * 210.0;
        let sel = st.song_focus == SongFocus::Export(i);
        if sel {
            c.rect(x - 6.0, 384.0, 202.0, 102.0, 22.0, NEON_AMARILLO);
        }
        c.rect(x, 390.0, 190.0, 90.0, 18.0, col);
        c.text(x + 95.0, 425.0, Align2::CENTER_CENTER, name, 34.0, BLANCO);
        c.text(x + 95.0, 461.0, Align2::CENTER_CENTER, sub, 15.0, BLANCO);
    }
    let x = 712.0;
    if let Some(b) = sv.busy {
        c.text(x, 410.0, Align2::LEFT_CENTER, b, 18.0, NEON_AMARILLO);
        c.rect(x, 430.0, 196.0, 22.0, 11.0, OLED_APAGADO);
        // Barra que va y viene: no sabemos cuánto falta.
        let t = (extra.time * 1.5).sin() as f32 * 0.5 + 0.5;
        c.rect(x + t * 136.0, 430.0, 60.0, 22.0, 11.0, NEON_AMARILLO);
    } else if let Some(p) = sv.last_export {
        c.text(
            x,
            410.0,
            Align2::LEFT_CENTER,
            "✔ Exportada",
            18.0,
            hex(0x2ED573),
        );
        let name = p
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();
        c.text(x, 440.0, Align2::LEFT_CENTER, name, 13.0, NEON_CIAN);
    }
    c.text(
        x,
        474.0,
        Align2::LEFT_CENTER,
        "Música › Mapachsynthesizer\n› Exportadas",
        11.0,
        NEON_CIAN.gamma_multiply(0.6),
    );
    let _ = sv.current_color;
}

/// Taller de samples (boceto 04-taller-samples.svg).
fn taller_screen(c: &Canvas, st: &State, extra: &Extra) {
    use mapach_samples::destroy::Destroy;
    c.rect(36.0, 30.0, 320.0, 48.0, 24.0, hex(0xB45CFF));
    c.text(
        196.0,
        54.0,
        Align2::CENTER_CENTER,
        "🧪 TALLER DE SAMPLES",
        22.0,
        BLANCO,
    );
    c.text(
        924.0,
        54.0,
        Align2::RIGHT_CENTER,
        "↑↓ elegir · Espacio grabar · Intro escuchar · Tab salir",
        14.0,
        MARRON,
    );
    let Some(tv) = &extra.taller else { return };

    // Biblioteca (5 visibles).
    let n = tv.list.len();
    let start = st.sample_sel.saturating_sub(2).min(n.saturating_sub(5));
    for slot in 0..5 {
        let i = start + slot;
        let y = 96.0 + slot as f32 * 66.0;
        if i >= n {
            if i == n {
                // Tarjeta de grabar.
                c.rect_stroke(36.0, y, 230.0, 58.0, 18.0, 3.0, MENU_ROJO);
                c.circle(68.0, y + 29.0, 16.0, MENU_ROJO);
                c.circle(68.0, y + 29.0, 7.0, BLANCO);
                c.text(
                    96.0,
                    y + 29.0,
                    Align2::LEFT_CENTER,
                    "Espacio: grabar",
                    17.0,
                    MENU_ROJO,
                );
            }
            continue;
        }
        let e = &tv.list[i];
        let col = rgb(e.color);
        let sel = i == st.sample_sel;
        if sel {
            c.rect(31.0, y - 5.0, 240.0, 68.0, 22.0, BLANCO);
        }
        c.rect(36.0, y, 230.0, 58.0, 18.0, col);
        let tc = texto_sobre(col);
        c.circle(64.0, y + 29.0, 16.0, BLANCO.gamma_multiply(0.35));
        c.text(65.0, y + 29.0, Align2::CENTER_CENTER, "▶", 15.0, tc);
        let name: String = e.name.chars().take(16).collect();
        c.text(90.0, y + 22.0, Align2::LEFT_CENTER, name, 17.0, tc);
        let sub = if sel {
            format!("{:.1} s", tv.seconds)
        } else {
            String::new()
        };
        let star = if st.instrument.as_deref() == Some(&*e.path.to_string_lossy()) {
            " · 🎹"
        } else {
            ""
        };
        c.text(
            90.0,
            y + 42.0,
            Align2::LEFT_CENTER,
            format!("{sub}{star}"),
            12.0,
            tc,
        );
    }
    if n == 0 {
        c.text(
            150.0,
            200.0,
            Align2::CENTER_CENTER,
            "Aún no hay sonidos.\nGraba tu voz\no deja ficheros en\nMúsica › Mapachsynthesizer\n› Importar",
            14.0,
            MARRON,
        );
    }

    // Forma de onda.
    c.rect(282.0, 92.0, 646.0, 250.0, 28.0, NARANJA);
    c.rect(290.0, 100.0, 630.0, 234.0, 22.0, OLED);
    let title = if tv.recording {
        format!("🔴 Grabando… {:.1} s (Espacio para parar)", tv.rec_seconds)
    } else {
        match (tv.list.get(st.sample_sel), tv.root) {
            (Some(e), Some(r)) if st.show_names => format!(
                "{} · nota {}",
                e.name,
                NOTE_NAMES[(r.round() as i32).rem_euclid(12) as usize]
            ),
            (Some(e), Some(_)) => format!("{} · 🎵 tiene nota", e.name),
            (Some(e), None) => format!("{} · 🥁 sin nota", e.name),
            (None, _) => "Elige un sonido o graba uno".into(),
        }
    };
    c.text(310.0, 124.0, Align2::LEFT_CENTER, title, 19.0, NEON_CIAN);
    if tv.recording {
        let lvl = (tv.rec_level * 3.0).clamp(0.0, 1.0);
        c.rect(310.0, 210.0, 590.0, 30.0, 15.0, OLED_APAGADO);
        c.rect(310.0, 210.0, 590.0 * lvl, 30.0, 15.0, MENU_ROJO);
        let f = (tv.rec_seconds / mapach_samples::MAX_SECONDS).clamp(0.0, 1.0);
        c.rect(310.0, 260.0, 590.0 * f, 8.0, 4.0, NEON_AMARILLO);
    } else {
        let cols = tv.overview.len().max(1);
        let w = 590.0 / cols as f32;
        for (i, &v) in tv.overview.iter().enumerate() {
            let h = (v * 170.0).max(3.0);
            let x = 310.0 + i as f32 * w;
            c.rect(
                x,
                225.0 - h / 2.0,
                (w - 1.5).max(1.0),
                h,
                1.5,
                hex(0x2ED573),
            );
        }
    }

    // Botones de destrozar.
    c.text(
        290.0,
        362.0,
        Align2::LEFT_CENTER,
        "¡DESTRÓZALO!  Z aplicar · C deshacer · X acordes · X + tecla: batería",
        17.0,
        MARRON,
    );
    let colors = [
        hex(0xFF3B30),
        hex(0xB45CFF),
        hex(0x1EC8FF),
        hex(0xFF8A00),
        hex(0x3D7BFF),
        hex(0x2ED573),
        hex(0xFF5CE1),
        hex(0xFFD400),
        hex(0x8E8A99),
        hex(0xC86B2E),
    ];
    for (i, d) in Destroy::ALL.iter().enumerate() {
        let (row, col) = (i / 5, i % 5);
        let x = 290.0 + col as f32 * 128.0;
        let y = 384.0 + row as f32 * 70.0;
        let sel = i == st.destroy_sel;
        if sel {
            c.rect(x - 5.0, y - 5.0, 126.0, 66.0, 18.0, BLANCO);
        }
        c.rect(x, y, 116.0, 56.0, 14.0, colors[i]);
        let tc = texto_sobre(colors[i]);
        c.text(
            x + 58.0,
            y + 20.0,
            Align2::CENTER_CENTER,
            d.icon(),
            20.0,
            tc,
        );
        c.text(
            x + 58.0,
            y + 42.0,
            Align2::CENTER_CENTER,
            d.name(),
            14.0,
            tc,
        );
    }
    // Leyenda de los botones de colores.
    for (i, (txt, col, tc)) in [
        ("Z aplicar", MENU_GRIS, BLANCO),
        ("X usar", MENU_AMARILLO, MARRON),
        ("C deshacer", MENU_ROJO, BLANCO),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 440.0 + i as f32 * 28.0;
        c.rect(36.0, y - 12.0, 230.0, 24.0, 12.0, col);
        c.text(151.0, y, Align2::CENTER_CENTER, txt, 15.0, tc);
    }
}

/// Asistente "Aprender mando".
pub fn draw_learn(c: &Canvas, step: usize, total: usize, prompt: &str, msg: Option<&str>) {
    c.painter
        .rect_filled(c.painter.clip_rect(), 0.0, Color32::from_black_alpha(210));
    c.rect(160.0, 90.0, 640.0, 360.0, 28.0, OLED);
    c.rect_stroke(160.0, 90.0, 640.0, 360.0, 28.0, 4.0, hex(0x1EC8FF));
    c.text(
        480.0,
        130.0,
        Align2::CENTER_CENTER,
        "🎮 Aprender mando",
        30.0,
        NEON_AMARILLO,
    );
    c.text(480.0, 230.0, Align2::CENTER_CENTER, prompt, 26.0, BLANCO);
    if let Some(m) = msg {
        c.text(480.0, 310.0, Align2::CENTER_CENTER, m, 17.0, MENU_ROJO);
    }
    let f = step as f32 / total.max(1) as f32;
    c.rect(220.0, 350.0, 520.0, 16.0, 8.0, OLED_APAGADO);
    c.rect(220.0, 350.0, 520.0 * f, 16.0, 8.0, hex(0x2ED573));
    c.text(
        480.0,
        390.0,
        Align2::CENTER_CENTER,
        format!("{step} de {total}"),
        16.0,
        NEON_CIAN,
    );
    c.text(
        480.0,
        425.0,
        Align2::CENTER_CENTER,
        "Esc cancelar",
        15.0,
        NEON_CIAN.gamma_multiply(0.6),
    );
}

/// Pantalla del modo papá.
pub fn draw_papa(c: &Canvas, lines: &[String]) {
    c.painter
        .rect_filled(c.painter.clip_rect(), 0.0, Color32::from_black_alpha(200));
    c.rect(180.0, 60.0, 600.0, 430.0, 28.0, OLED);
    c.rect_stroke(180.0, 60.0, 600.0, 430.0, 28.0, 4.0, NARANJA);
    c.text(
        480.0,
        100.0,
        Align2::CENTER_CENTER,
        "🔧 Modo papá",
        32.0,
        NEON_AMARILLO,
    );
    for (i, l) in lines.iter().enumerate() {
        c.text(
            220.0,
            148.0 + i as f32 * 26.0,
            Align2::LEFT_CENTER,
            l,
            18.0,
            NEON_CIAN,
        );
    }
    c.text(
        480.0,
        468.0,
        Align2::CENTER_CENTER,
        "Q salir del programa · Esc volver",
        20.0,
        NEON_AMARILLO,
    );
}

/// Para los tests y el ratón: qué tecla de acorde hay en un punto del lienzo.
pub fn chord_at(p: Pos2) -> Option<u8> {
    (0..7)
        .find(|&b| chord_button_rect(b).contains(p))
        .map(|b| b as u8)
}
