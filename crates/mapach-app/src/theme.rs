//! Paleta de colores (ver `docs/plan.md` §6) y fuentes.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily};

pub const fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub const AMARILLO: Color32 = hex(0xFFC21A);
pub const AMARILLO_CLARO: Color32 = hex(0xFFD23F);
pub const NARANJA: Color32 = hex(0xFF7A1A);
pub const OLED: Color32 = hex(0x1C1230);
pub const OLED_APAGADO: Color32 = hex(0x2E2248);
pub const NEON_AMARILLO: Color32 = hex(0xFFE45C);
pub const NEON_CIAN: Color32 = hex(0x4DF0FF);
pub const NEON_ROSA: Color32 = hex(0xFF5CE1);
pub const MENU_GRIS: Color32 = hex(0x8E8A99);
pub const MENU_AMARILLO: Color32 = hex(0xFFE600);
pub const MENU_ROJO: Color32 = hex(0xFF3B30);
pub const MARRON: Color32 = hex(0x5A3A00);
pub const BLANCO: Color32 = Color32::WHITE;

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
    if l > 170.0 { MARRON } else { BLANCO }
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
