//! Mide cuánto más rápido que el tiempo real va el motor en el peor caso
//! (todas las voces, ritmo y bloques de 256 muestras).
//!
//! `cargo run --release -p mapach-engine --example rendimiento`

use mapach_engine::{Command, Engine, MAX_VOICES, SOUNDS, drums};

fn main() {
    let sr = 48_000.0;
    let secs = 20.0;
    for sound in SOUNDS {
        let mut e = Engine::new(sr);
        e.handle(Command::SetPatch(sound.patch));
        e.handle(Command::SetPattern(drums::rhythm(2, 6)));
        e.handle(Command::DrumsPlay(true));
        for n in 0..MAX_VOICES as u8 {
            e.handle(Command::NoteOn {
                note: 36 + n * 2,
                velocity: 1.0,
            });
        }
        let mut buf = vec![0.0f32; 256 * 2];
        let blocks = (secs * sr / 256.0) as usize;
        let t = std::time::Instant::now();
        for _ in 0..blocks {
            e.process(&mut buf);
        }
        let el = t.elapsed().as_secs_f32();
        println!(
            "{:<18} {:>6.1}× tiempo real ({:.0} % de un núcleo)",
            sound.name,
            secs / el,
            100.0 * el / secs
        );
    }
}
