//! Sonidos (patches) del sintetizador analógico virtual.

use mapach_dsp::{AdsrParams, Waveform};

/// Ajustes de un sonido. Es `Copy` para poder mandarlo por el ring buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
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
    }
}

use Waveform::*;

/// Sonidos de serie del motor analógico.
pub const SOUNDS: [Sound; 8] = [
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
];
