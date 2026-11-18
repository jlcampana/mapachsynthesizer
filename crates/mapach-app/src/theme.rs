//! Paleta de colores (ver `docs/plan.md` §6) y fuentes.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily};

pub const fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

// Interfaz oscura y sobria, como un estudio de grabación (inspirada en GarageBand).
pub const FONDO: Color32 = hex(0x1B1B1D);
pub const PANEL: Color32 = hex(0x26262A);
pub const PANEL_2: Color32 = hex(0x313136);
pub const BORDE: Color32 = hex(0x3F3F46);
pub const TEXTO: Color32 = hex(0xEDEDF0);
pub const TEXTO_2: Color32 = hex(0x9C9CA6);
pub const TEXTO_OSCURO: Color32 = hex(0x1C1C1E);
/// Ámbar para lo elegido.
pub const ACENTO: Color32 = hex(0xF5B83D);
/// Fondo de la pantallita LCD de arriba.
pub const LCD: Color32 = hex(0x0E0F11);
pub const VERDE: Color32 = hex(0x4CD964);

// Nombres de antes, ya con la paleta nueva.
pub const NARANJA: Color32 = BORDE;
pub const OLED: Color32 = hex(0x151517);
pub const OLED_APAGADO: Color32 = hex(0x2C2C31);
pub const NEON_AMARILLO: Color32 = TEXTO;
pub const NEON_CIAN: Color32 = TEXTO_2;
pub const NEON_ROSA: Color32 = hex(0x5AC8FA);
pub const MARRON: Color32 = TEXTO_2;
pub const BLANCO: Color32 = Color32::WHITE;

/// Colores de las teclas de menú del mando (gris, amarillo y rojo).
pub const MENU_GRIS: Color32 = hex(0x8E8A99);
pub const MENU_AMARILLO: Color32 = hex(0xF2CF2E);
pub const MENU_ROJO: Color32 = hex(0xE5483D);

/// Un color por tecla de acorde, en arcoíris.
pub const ACORDES: [Color32; 7] = [
    hex(0xFF3B3B),
    hex(0xFF8A00),
    hex(0xFFD400),
    hex(0x2ED573),
    hex(0x1EC8FF),
    hex(0x3D7BFF),
    hex(0xB45CFF),
];

/// ¿Va mejor texto oscuro encima de este color?
pub fn texto_sobre(c: Color32) -> Color32 {
    let l = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if l > 170.0 { TEXTO_OSCURO } else { BLANCO }
}

/// Color de cada una de las 12 tonalidades (rueda de color), para no tener que poner nombres.
pub fn color_tonalidad(tonic: i32) -> Color32 {
    let h = (tonic.rem_euclid(12) as f32 * 7.0 % 12.0) / 12.0;
    egui::ecolor::Hsva::new(h, 0.75, 1.0, 1.0).into()
}

/// Fredoka (OFL) en negrita, incluida en el binario. Las fuentes de serie de egui quedan de
/// respaldo para flechas, emojis y símbolos.
pub fn fuentes(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let tweak = egui::FontTweak {
        coords: egui::epaint::text::VariationCoords::new([(b"wght", 600.0)]),
        ..Default::default()
    };
    defs.font_data.insert(
        "fredoka".into(),
        FontData::from_static(include_bytes!("../../../assets/fuentes/Fredoka.ttf"))
            .tweak(tweak)
            .into(),
    );
    for fam in [FontFamily::Proportional, FontFamily::Monospace] {
        defs.families
            .entry(fam)
            .or_default()
            .insert(0, "fredoka".into());
    }
    ctx.set_fonts(defs);
}

/// Colores de las notas, los de los tubos y campanas de las clases de música (Rainbow mode):
/// Do rojo, Re naranja, Mi amarillo, Fa verde, Sol turquesa, La morado, Si rosa. Los sostenidos,
/// el color intermedio.
pub fn note_color(pc: i32) -> Color32 {
    const NATURALS: [(i32, Color32); 7] = [
        (0, hex(0xFF3B3B)),
        (2, hex(0xFF8A00)),
        (4, hex(0xFFD400)),
        (5, hex(0x2ED573)),
        (7, hex(0x1ED6C8)),
        (9, hex(0xB45CFF)),
        (11, hex(0xFF5CE1)),
    ];
    let pc = pc.rem_euclid(12);
    if let Some(&(_, c)) = NATURALS.iter().find(|(p, _)| *p == pc) {
        return c;
    }
    let below = NATURALS
        .iter()
        .rev()
        .find(|(p, _)| *p < pc)
        .map_or(NATURALS[6].1, |x| x.1);
    let above = NATURALS
        .iter()
        .find(|(p, _)| *p > pc)
        .map_or(NATURALS[0].1, |x| x.1);
    below.lerp_to_gamma(above, 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colores_por_nota() {
        assert_eq!(note_color(0), hex(0xFF3B3B));
        assert_eq!(note_color(12), note_color(0));
        assert_eq!(note_color(7), hex(0x1ED6C8));
        // Do♯ queda entre rojo y naranja.
        let cs = note_color(1);
        assert!(cs != note_color(0) && cs != note_color(2));
    }
}
