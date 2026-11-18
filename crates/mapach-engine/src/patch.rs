//! Sonidos (patches): analógico virtual, FM, aditivo, ruido y formantes.

use mapach_dsp::{AdsrParams, Waveform};

/// Motor de sonido de una voz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// Dos osciladores (los campos `osc1`, `osc2`…).
    Analog,
    /// FM de 2 operadores: un modulador (a `ratio` veces la frecuencia) mueve la fase de la
    /// portadora. `index` es la profundidad, que cae con `index_decay` segundos (golpe brillante
    /// que se apaga, como un piano eléctrico o una campana).
    Fm {
        ratio: f32,
        index: f32,
        index_decay: f32,
        /// Profundidad que queda al final (0–1 de `index`).
        index_sustain: f32,
    },
    /// Suma de senos en los armónicos 1, 2, 3, 4, 6 y 8 (como los tiradores de un órgano).
    Additive { drawbars: [f32; 6] },
    /// Ruido filtrado en la nota: viento afinado, olas, naves espaciales.
    /// `focus`: resonancia (0 = ruido ancho, 1 = casi silbido). `lfo`: Hz de ondulación.
    Noise { focus: f32, lfo: f32 },
    /// Sierra con dos formantes de vocal (coro). `vowel`: 0 = a, 1 = e, 2 = i, 3 = o, 4 = u.
    Formant { vowel: u8 },
    /// Un sample del taller, afinado según su nota raíz.
    Sampler { slot: u8 },
}

/// Ajustes de un sonido. Es `Copy` para poder mandarlo por el ring buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub kind: Kind,
    pub osc1: Waveform,
    pub osc2: Waveform,
    /// Transposición de cada oscilador en semitonos.
    pub osc1_semitones: f32,
    pub osc2_semitones: f32,
    /// Desafinación del oscilador 2 en cents (da anchura).
    pub detune_cents: f32,
    /// Cantidad del oscilador 2 (0–1).
    pub osc2_mix: f32,
    pub env: AdsrParams,
    /// Brillo por defecto (corte del filtro), 0–1.
    pub brightness: f32,
    pub resonance: f32,
    /// Cuánto abre la envolvente el filtro (multiplicador del corte en el pico).
    pub env_to_filter: f32,
    /// Apertura estéreo (0–1).
    pub stereo: f32,
    /// Vibrato: profundidad en semitonos (a 5,5 Hz).
    pub vibrato: f32,
}

impl Default for Patch {
    fn default() -> Self {
        SOUNDS[0].patch
    }
}

/// Un sonido con nombre, color e icono para la pantalla.
#[derive(Debug, Clone, Copy)]
pub struct Sound {
    pub name: &'static str,
    pub icon: &'static str,
    pub color: [u8; 3],
    pub patch: Patch,
}

const fn patch(
    osc1: Waveform,
    osc2: Waveform,
    osc2_semitones: f32,
    detune_cents: f32,
    osc2_mix: f32,
    env: AdsrParams,
    brightness: f32,
    resonance: f32,
    env_to_filter: f32,
) -> Patch {
    Patch {
        kind: Kind::Analog,
        osc1,
        osc2,
        osc1_semitones: 0.0,
        osc2_semitones,
        detune_cents,
        osc2_mix,
        env,
        brightness,
        resonance,
        env_to_filter,
        stereo: 1.0,
        vibrato: 0.0,
    }
}

/// Pedorreta: pulso y sierra muy graves, con filtro resonante y mucho meneo.
const fn fart() -> Patch {
    let mut p = patch(
        Pulse,
        Saw,
        -12.0,
        35.0,
        0.7,
        AdsrParams::new(0.01, 0.25, 0.6, 0.12),
        0.22,
        0.65,
        1.8,
    );
    p.osc1_semitones = -24.0;
    p.osc2_semitones = -24.0;
    p.vibrato = 0.9;
    p
}

/// Patch de otro motor, con su envolvente y brillo.
const fn other(kind: Kind, env: AdsrParams, brightness: f32, vibrato: f32) -> Patch {
    let mut p = patch(Sine, Sine, 0.0, 0.0, 0.0, env, brightness, 0.0, 0.0);
    p.kind = kind;
    p.vibrato = vibrato;
    p
}

use Waveform::*;

/// Sonidos de serie del motor analógico.
pub const SOUNDS: [Sound; 23] = [
    Sound {
        name: "Nube",
        icon: "☁",
        color: [0x4D, 0xF0, 0xFF],
        patch: patch(Saw, Saw, 0.0, 9.0, 0.8, AdsrParams::TOUCH, 0.55, 0.15, 0.6),
    },
    Sound {
        name: "Piano de juguete",
        icon: "🎹",
        color: [0xFF, 0xD4, 0x00],
        patch: patch(
            Triangle,
            Square,
            12.0,
            3.0,
            0.25,
            AdsrParams::KEYS,
            0.6,
            0.1,
            1.5,
        ),
    },
    Sound {
        name: "Órgano",
        icon: "⛪",
        color: [0xB4, 0x5C, 0xFF],
        patch: patch(
            Square,
            Sine,
            12.0,
            2.0,
            0.6,
            AdsrParams::SHORT,
            0.5,
            0.0,
            0.0,
        ),
    },
    Sound {
        name: "Gotas",
        icon: "💧",
        color: [0x1E, 0xC8, 0xFF],
        patch: patch(
            Sine,
            Triangle,
            12.0,
            0.0,
            0.4,
            AdsrParams::PLUCK,
            0.7,
            0.3,
            2.0,
        ),
    },
    Sound {
        name: "Cuerdas",
        icon: "🎻",
        color: [0xFF, 0x8A, 0x00],
        patch: patch(Saw, Saw, 0.0, 14.0, 1.0, AdsrParams::SWELL, 0.45, 0.05, 0.3),
    },
    Sound {
        name: "Robot",
        icon: "🤖",
        color: [0x2E, 0xD5, 0x73],
        patch: patch(
            Pulse,
            Square,
            -12.0,
            5.0,
            0.5,
            AdsrParams::SHORT,
            0.4,
            0.75,
            1.0,
        ),
    },
    Sound {
        name: "Campanita",
        icon: "🔔",
        color: [0xFF, 0xE4, 0x5C],
        patch: patch(Sine, Sine, 19.0, 0.0, 0.5, AdsrParams::KEYS, 0.9, 0.0, 0.0),
    },
    Sound {
        name: "Espacio",
        icon: "🚀",
        color: [0x3D, 0x7B, 0xFF],
        patch: patch(Saw, Pulse, 7.0, 20.0, 0.7, AdsrParams::LONG, 0.35, 0.5, 1.2),
    },
    // --- FM ---
    Sound {
        name: "Piano eléctrico",
        icon: "🎹",
        color: [0xFF, 0x8A, 0x00],
        patch: other(
            Kind::Fm {
                ratio: 1.0,
                index: 2.2,
                index_decay: 0.6,
                index_sustain: 0.15,
            },
            AdsrParams::new(0.003, 1.6, 0.35, 0.6),
            0.85,
            0.0,
        ),
    },
    Sound {
        name: "Campanas",
        icon: "🔔",
        color: [0xFF, 0xE4, 0x5C],
        patch: other(
            Kind::Fm {
                ratio: 3.5,
                index: 3.0,
                index_decay: 1.2,
                index_sustain: 0.1,
            },
            AdsrParams::new(0.002, 2.5, 0.0, 2.5),
            1.0,
            0.0,
        ),
    },
    Sound {
        name: "Marimba",
        icon: "🪵",
        color: [0xC8, 0x6B, 0x2E],
        patch: other(
            Kind::Fm {
                ratio: 4.0,
                index: 1.6,
                index_decay: 0.05,
                index_sustain: 0.0,
            },
            AdsrParams::new(0.002, 0.45, 0.0, 0.4),
            1.0,
            0.0,
        ),
    },
    Sound {
        name: "Bajo FM",
        icon: "🎸",
        color: [0x3D, 0x7B, 0xFF],
        patch: other(
            Kind::Fm {
                ratio: 1.0,
                index: 3.5,
                index_decay: 0.15,
                index_sustain: 0.3,
            },
            AdsrParams::new(0.002, 0.4, 0.6, 0.15),
            0.6,
            0.0,
        ),
    },
    Sound {
        name: "Cristal",
        icon: "💎",
        color: [0x4D, 0xF0, 0xFF],
        patch: other(
            Kind::Fm {
                ratio: 7.0,
                index: 1.2,
                index_decay: 0.8,
                index_sustain: 0.2,
            },
            AdsrParams::new(0.01, 1.5, 0.3, 1.8),
            1.0,
            0.0,
        ),
    },
    // --- Aditivo ---
    Sound {
        name: "Órgano de iglesia",
        icon: "⛪",
        color: [0x8E, 0x8A, 0x99],
        patch: other(
            Kind::Additive {
                drawbars: [1.0, 0.8, 0.6, 0.5, 0.3, 0.4],
            },
            AdsrParams::new(0.08, 0.1, 1.0, 0.5),
            1.0,
            0.08,
        ),
    },
    Sound {
        name: "Flauta",
        icon: "🪈",
        color: [0x2E, 0xD5, 0x73],
        patch: other(
            Kind::Additive {
                drawbars: [1.0, 0.15, 0.08, 0.0, 0.0, 0.0],
            },
            AdsrParams::new(0.06, 0.2, 0.9, 0.25),
            1.0,
            0.15,
        ),
    },
    // --- Ruido ---
    Sound {
        name: "Viento",
        icon: "🌬",
        color: [0xB4, 0xE8, 0xFF],
        patch: other(
            Kind::Noise {
                focus: 0.92,
                lfo: 0.3,
            },
            AdsrParams::new(0.4, 0.5, 0.9, 1.5),
            1.0,
            0.0,
        ),
    },
    Sound {
        name: "Olas",
        icon: "🌊",
        color: [0x1E, 0xC8, 0xFF],
        patch: other(
            Kind::Noise {
                focus: 0.2,
                lfo: 0.15,
            },
            AdsrParams::new(1.0, 1.0, 0.9, 2.5),
            0.7,
            0.0,
        ),
    },
    Sound {
        name: "Nave espacial",
        icon: "🛸",
        color: [0xB4, 0x5C, 0xFF],
        patch: other(
            Kind::Noise {
                focus: 0.98,
                lfo: 6.0,
            },
            AdsrParams::new(0.2, 0.5, 0.8, 1.0),
            1.0,
            0.6,
        ),
    },
    // --- Formantes ---
    Sound {
        name: "Coro",
        icon: "👄",
        color: [0xFF, 0x5C, 0xE1],
        patch: other(
            Kind::Formant { vowel: 0 },
            AdsrParams::new(0.25, 0.3, 0.9, 0.8),
            1.0,
            0.12,
        ),
    },
    Sound {
        name: "Fantasma",
        icon: "👻",
        color: [0xE0, 0xE0, 0xFF],
        patch: other(
            Kind::Formant { vowel: 4 },
            AdsrParams::new(0.5, 0.5, 0.8, 1.5),
            1.0,
            0.35,
        ),
    },
    // --- Sampler ---
    Sound {
        name: "Mi sample",
        icon: "🎤",
        color: [0xFF, 0x3B, 0x3B],
        patch: other(
            Kind::Sampler { slot: 0 },
            AdsrParams::new(0.002, 0.1, 1.0, 0.3),
            1.0,
            0.0,
        ),
    },
    // --- Secretos (se desbloquean con estrellas; siempre al final) ---
    Sound {
        name: "Pedorreta",
        icon: "💨",
        color: [0x8B, 0x5A, 0x2B],
        patch: fart(),
    },
    Sound {
        name: "Gato",
        icon: "🐱",
        color: [0xFF, 0xB3, 0x47],
        patch: other(
            Kind::Formant { vowel: 2 },
            AdsrParams::new(0.12, 0.4, 0.6, 0.5),
            1.0,
            0.6,
        ),
    },
];
