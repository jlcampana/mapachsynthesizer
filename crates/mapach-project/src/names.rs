//! Nombres divertidos para las canciones: "Dragón Naranja", "Pulpo Saltarín"…

/// Animales y cosas, con su género (true = femenino).
const NOUNS: &[(&str, bool)] = &[
    ("Dragón", false),
    ("Pulpo", false),
    ("Mapache", false),
    ("Cohete", false),
    ("Pingüino", false),
    ("Dinosaurio", false),
    ("Robot", false),
    ("Unicornio", false),
    ("Volcán", false),
    ("Koala", false),
    ("Tiburón", false),
    ("Gato", false),
    ("Perro", false),
    ("Búho", false),
    ("Caracol", false),
    ("Murciélago", false),
    ("Jirafa", true),
    ("Ballena", true),
    ("Tortuga", true),
    ("Medusa", true),
    ("Nube", true),
    ("Estrella", true),
    ("Mariposa", true),
    ("Rana", true),
    ("Abeja", true),
    ("Luna", true),
    ("Pizza", true),
    ("Galleta", true),
    ("Ardilla", true),
    ("Cebra", true),
];

/// Adjetivos (masculino, femenino).
const ADJECTIVES: &[(&str, &str)] = &[
    ("Naranja", "Naranja"),
    ("Saltarín", "Saltarina"),
    ("Galáctico", "Galáctica"),
    ("Dormilón", "Dormilona"),
    ("Valiente", "Valiente"),
    ("Brillante", "Brillante"),
    ("Morado", "Morada"),
    ("Bailarín", "Bailarina"),
    ("Cósmico", "Cósmica"),
    ("Despistado", "Despistada"),
    ("Gigante", "Gigante"),
    ("Diminuto", "Diminuta"),
    ("Eléctrico", "Eléctrica"),
    ("Musical", "Musical"),
    ("Risueño", "Risueña"),
    ("Veloz", "Veloz"),
    ("Mágico", "Mágica"),
    ("Turquesa", "Turquesa"),
    ("Cantarín", "Cantarina"),
    ("Espacial", "Espacial"),
];

/// Icono para la tarjeta de una canción, según su primera palabra.
pub fn icon_for(name: &str) -> &'static str {
    match name.split(' ').next().unwrap_or("") {
        "Dragón" => "🐉",
        "Pulpo" => "🐙",
        "Mapache" => "🦝",
        "Cohete" => "🚀",
        "Pingüino" => "🐧",
        "Dinosaurio" => "🦕",
        "Robot" => "🤖",
        "Unicornio" => "🦄",
        "Volcán" => "🌋",
        "Koala" => "🐨",
        "Tiburón" => "🦈",
        "Gato" => "🐱",
        "Perro" => "🐶",
        "Búho" => "🦉",
        "Caracol" => "🐌",
        "Murciélago" => "🦇",
        "Jirafa" => "🦒",
        "Ballena" => "🐳",
        "Tortuga" => "🐢",
        "Medusa" => "🪼",
        "Nube" => "☁",
        "Estrella" => "⭐",
        "Mariposa" => "🦋",
        "Rana" => "🐸",
        "Abeja" => "🐝",
        "Luna" => "🌙",
        "Pizza" => "🍕",
        "Galleta" => "🍪",
        "Ardilla" => "🐿",
        "Cebra" => "🦓",
        _ => "🎵",
    }
}

/// Colores vivos para las tarjetas de las canciones.
pub const COLORS: [[u8; 3]; 8] = [
    [0xFF, 0x3B, 0x3B],
    [0xFF, 0x8A, 0x00],
    [0xFF, 0xD4, 0x00],
    [0x2E, 0xD5, 0x73],
    [0x1E, 0xC8, 0xFF],
    [0x3D, 0x7B, 0xFF],
    [0xB4, 0x5C, 0xFF],
    [0xFF, 0x5C, 0xE1],
];

/// Nombre y color a partir de una semilla (por ejemplo, la hora).
pub fn funny_name(seed: u64) -> (String, [u8; 3]) {
    // Mezcla de bits (splitmix64) para que semillas parecidas den nombres distintos.
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let (noun, fem) = NOUNS[(z % NOUNS.len() as u64) as usize];
    let (m, f) = ADJECTIVES[((z >> 16) % ADJECTIVES.len() as u64) as usize];
    let color = COLORS[((z >> 32) % COLORS.len() as u64) as usize];
    (format!("{noun} {}", if fem { f } else { m }), color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concuerdan_en_genero() {
        for seed in 0..500 {
            let (name, _) = funny_name(seed);
            let mut parts = name.split(' ');
            let noun = parts.next().unwrap();
            let adj = parts.next().unwrap();
            let fem = NOUNS.iter().find(|n| n.0 == noun).unwrap().1;
            let pair = ADJECTIVES
                .iter()
                .find(|a| a.0 == adj || a.1 == adj)
                .unwrap();
            assert_eq!(adj, if fem { pair.1 } else { pair.0 }, "{name}");
        }
    }

    #[test]
    fn todos_tienen_icono() {
        for (n, _) in NOUNS {
            assert_ne!(icon_for(n), "🎵", "{n}");
        }
        assert_eq!(icon_for("Lo que sea"), "🎵");
    }

    #[test]
    fn variados() {
        let mut names: Vec<String> = (0..50).map(|s| funny_name(s).0).collect();
        names.sort();
        names.dedup();
        assert!(names.len() > 40);
    }
}
