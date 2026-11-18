//! Mezcla del looper y exportación a WAV, OGG y MP3.

use std::io::Write;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use mapach_dsp::{Limiter, db_to_gain};

use crate::song::{TRACKS, TrackAudio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp3,
    Ogg,
    Wav,
}

impl Format {
    pub const ALL: [Format; 3] = [Format::Mp3, Format::Ogg, Format::Wav];

    pub fn ext(self) -> &'static str {
        match self {
            Format::Mp3 => "mp3",
            Format::Ogg => "ogg",
            Format::Wav => "wav",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Mp3 => "MP3",
            Format::Ogg => "OGG",
            Format::Wav => "WAV",
        }
    }
}

/// Mezcla las pistas del looper: repite la vuelta hasta llegar a unos 20 s (entre 2 y 8
/// vueltas), aplica volúmenes y silencios, un fundido final y el limitador.
pub fn mixdown(
    tracks: &[Option<TrackAudio>; TRACKS],
    loop_frames: usize,
    volumes: &[f32; TRACKS],
    muted: &[bool; TRACKS],
    sample_rate: u32,
    ceiling_db: f32,
) -> Vec<f32> {
    if loop_frames == 0 {
        return Vec::new();
    }
    let secs = loop_frames as f32 / sample_rate as f32;
    let loops = ((20.0 / secs).ceil() as usize).clamp(2, 8);
    let total = loop_frames * loops;
    let mut out = vec![0.0f32; total * 2];
    for (t, track) in tracks.iter().enumerate() {
        let Some(track) = track else { continue };
        if muted[t] {
            continue;
        }
        let g = volumes[t];
        let n = track.frames().min(loop_frames);
        for (f, frame) in out.chunks_exact_mut(2).enumerate() {
            let i = f % loop_frames;
            if i < n {
                frame[0] += track.samples[i * 2] * g;
                frame[1] += track.samples[i * 2 + 1] * g;
            }
        }
    }
    // Fundido de salida en el último segundo (o la última vuelta, si es más corta).
    let fade = (sample_rate as usize).min(loop_frames).min(total);
    for (k, frame) in out.chunks_exact_mut(2).skip(total - fade).enumerate() {
        let g = 1.0 - k as f32 / fade as f32;
        frame[0] *= g;
        frame[1] *= g;
    }
    // El mismo limitador que en directo: nada pasa del techo.
    let mut lim = Limiter::new(db_to_gain(ceiling_db), 0.15, sample_rate as f32);
    // Compensamos el retardo del limitador procesando una cola de silencio.
    let mut limited = Vec::with_capacity(out.len());
    let delay = 64;
    for i in 0..total + delay {
        let (l, r) = if i < total {
            (out[i * 2], out[i * 2 + 1])
        } else {
            (0.0, 0.0)
        };
        let (a, b) = lim.process(l, r);
        if i >= delay {
            limited.push(a);
            limited.push(b);
        }
    }
    limited
}

/// Escribe el audio (estéreo intercalado) en el formato pedido.
pub fn write(
    path: &Path,
    samples: &[f32],
    sample_rate: u32,
    format: Format,
    title: &str,
) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Primero a un temporal: así nunca queda un fichero a medias con el nombre bueno.
    let tmp = path.with_extension(format!("{}.tmp", format.ext()));
    match format {
        Format::Wav => write_wav(&tmp, samples, sample_rate)?,
        Format::Ogg => write_ogg(&tmp, samples, sample_rate, title)?,
        Format::Mp3 => write_mp3(&tmp, samples, sample_rate, title)?,
    }
    std::fs::rename(tmp, path)?;
    Ok(())
}

/// Ruta libre en `dir` para `name.ext` (añade " 2", " 3"… si ya existe).
pub fn free_path(dir: &Path, name: &str, format: Format) -> PathBuf {
    let base = crate::paths::safe_file_name(name);
    let mut p = dir.join(format!("{base}.{}", format.ext()));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base} {n}.{}", format.ext()));
        n += 1;
    }
    p
}

fn write_wav(path: &Path, samples: &[f32], rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16)?;
    }
    w.finalize()?;
    Ok(())
}

fn write_ogg(path: &Path, samples: &[f32], rate: u32, title: &str) -> Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut builder = vorbis_rs::VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(rate).ok_or_else(|| anyhow!("frecuencia 0"))?,
        NonZeroU8::new(2).expect("2 no es 0"),
        file,
        0x4D41_5041, // serie fija: el mismo audio da el mismo fichero
    );
    builder.comment_tags([("TITLE", title), ("ARTIST", "Mapachsynthesizer")])?;
    let mut enc = builder.build()?;
    const BLOCK: usize = 4096;
    let mut left = Vec::with_capacity(BLOCK);
    let mut right = Vec::with_capacity(BLOCK);
    for chunk in samples.chunks(BLOCK * 2) {
        left.clear();
        right.clear();
        for f in chunk.chunks_exact(2) {
            left.push(f[0]);
            right.push(f[1]);
        }
        enc.encode_audio_block([&left, &right])?;
    }
    let mut file = enc.finish()?;
    file.flush()?;
    Ok(())
}

fn write_mp3(path: &Path, samples: &[f32], rate: u32, title: &str) -> Result<()> {
    use mp3lame_encoder::{Bitrate, Builder, FlushNoGap, Id3Tag, InterleavedPcm, Quality};
    let mut b = Builder::new().ok_or_else(|| anyhow!("no se pudo iniciar LAME"))?;
    let err = |e| anyhow!("LAME: {e:?}");
    b.set_num_channels(2).map_err(err)?;
    b.set_sample_rate(rate).map_err(err)?;
    b.set_brate(Bitrate::Kbps192).map_err(err)?;
    b.set_quality(Quality::Best).map_err(err)?;
    let title = title.as_bytes();
    let _ = b.set_id3_tag(Id3Tag {
        title: &title[..title.len().min(30)],
        artist: b"Mapachsynthesizer",
        album: &[],
        year: &[],
        comment: &[],
        album_art: &[],
    });
    let mut enc = b.build().map_err(err)?;
    let mut out = Vec::new();
    const BLOCK: usize = 4096;
    for chunk in samples.chunks(BLOCK * 2) {
        // Tamaño máximo que puede ocupar un bloque codificado (fórmula de LAME).
        out.reserve(BLOCK * 5 / 4 + 7200);
        enc.encode_to_vec(InterleavedPcm(chunk), &mut out)
            .map_err(|e| anyhow!("LAME: {e:?}"))?;
    }
    out.reserve(7200);
    enc.flush_to_vec::<FlushNoGap>(&mut out)
        .map_err(|e| anyhow!("LAME: {e:?}"))?;
    std::fs::write(path, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frames: usize, amp: f32) -> TrackAudio {
        let samples = (0..frames)
            .flat_map(|i| {
                let s = (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * amp;
                [s, s]
            })
            .collect();
        TrackAudio { samples }
    }

    fn tracks() -> [Option<TrackAudio>; TRACKS] {
        let mut t: [Option<TrackAudio>; TRACKS] = Default::default();
        t[0] = Some(tone(48_000, 0.8));
        t[3] = Some(tone(48_000, 0.8));
        t
    }

    #[test]
    fn mezcla_repite_y_no_pasa_del_techo() {
        let mix = mixdown(&tracks(), 48_000, &[1.0; 6], &[false; 6], 48_000, -12.0);
        // 1 s de vuelta → 20 vueltas, pero como mucho 8.
        assert_eq!(mix.len(), 8 * 48_000 * 2);
        let c = db_to_gain(-12.0);
        assert!(mix.iter().all(|s| s.abs() <= c + 1e-6));
        assert!(mix[10_000..20_000].iter().any(|s| s.abs() > 0.1));
        // Termina en silencio (fundido).
        assert!(mix[mix.len() - 20..].iter().all(|s| s.abs() < 0.01));
    }

    #[test]
    fn mezcla_vacia_y_silenciadas() {
        assert!(
            mixdown(
                &Default::default(),
                0,
                &[1.0; 6],
                &[false; 6],
                48_000,
                -12.0
            )
            .is_empty()
        );
        let mut muted = [false; 6];
        muted[0] = true;
        muted[3] = true;
        let mix = mixdown(&tracks(), 48_000, &[1.0; 6], &muted, 48_000, -12.0);
        assert!(mix.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn exportar_los_tres_formatos() {
        let dir = std::env::temp_dir().join(format!("mapach-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mix = mixdown(&tracks(), 48_000, &[1.0; 6], &[false; 6], 48_000, -12.0);
        for f in Format::ALL {
            let p = free_path(&dir, "Dragón Naranja", f);
            write(&p, &mix, 48_000, f, "Dragón Naranja").unwrap();
            let bytes = std::fs::read(&p).unwrap();
            assert!(bytes.len() > 1000, "{f:?}");
            let magic: &[u8] = match f {
                Format::Wav => b"RIFF",
                Format::Ogg => b"OggS",
                Format::Mp3 => b"ID3",
            };
            assert_eq!(&bytes[..magic.len()], magic, "{f:?}");
            // Un segundo fichero con el mismo nombre no pisa el primero.
            assert_ne!(free_path(&dir, "Dragón Naranja", f), p);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
