//! Leer WAV, MP3, OGG y FLAC con symphonia y dejarlos en mono a [`RATE`].

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::resample::resample;
use crate::{MAX_SECONDS, RATE, Sample};

/// Extensiones que se pueden importar.
pub const EXTENSIONS: [&str; 4] = ["wav", "mp3", "ogg", "flac"];

/// Decodifica un fichero de audio a mono y [`RATE`] Hz. Lee como mucho un poco más de
/// [`MAX_SECONDS`] (luego se recorta el silencio).
pub fn decode_file(path: &Path) -> Result<Sample> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("no se pudo abrir {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| anyhow!("no tiene sonido"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| anyhow!("formato de sonido desconocido"))?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())?;

    let mut rate = params.sample_rate.unwrap_or(RATE);
    let mut mono: Vec<f32> = Vec::new();
    let mut interleaved: Vec<f32> = Vec::new();
    let limit = ((MAX_SECONDS + 5.0) * rate as f32) as usize;
    while let Some(packet) = format.next_packet()? {
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(b) => b,
            // Un paquete roto no estropea el resto.
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        rate = buf.spec().rate();
        let channels = buf.spec().channels().count().max(1);
        interleaved.clear();
        buf.copy_to_vec_interleaved(&mut interleaved);
        mono.extend(
            interleaved
                .chunks_exact(channels)
                .map(|f| f.iter().sum::<f32>() / channels as f32),
        );
        if mono.len() > limit {
            break;
        }
    }
    if mono.is_empty() {
        return Err(anyhow!("está vacío"));
    }
    Ok(Sample::new(resample(&mono, rate, RATE)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_un_wav_estereo_a_44100() {
        let dir = std::env::temp_dir().join(format!("mapach-decode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tono.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for i in 0..44_100 {
            let v = ((i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin() * 16_000.0) as i16;
            w.write_sample(v).unwrap();
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        let s = decode_file(&path).unwrap();
        assert!((s.data.len() as i32 - 48_000).abs() < 5);
        let f = crate::pitch::detect(&s.data).unwrap();
        assert!((f - 440.0).abs() < 3.0, "{f}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn un_fichero_que_no_es_audio_da_error() {
        let dir = std::env::temp_dir().join(format!("mapach-decode-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("falso.mp3");
        std::fs::write(&path, b"esto no es un mp3").unwrap();
        assert!(decode_file(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
