//! Grabar del micrófono (para el taller de samples).
//!
//! El hilo de audio solo copia las muestras a un ring buffer. La interfaz las saca, las mezcla
//! a mono y luego las remuestrea.

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, StreamConfig};

/// Segundos que caben en la cola (de sobra para 10 s de grabación).
const QUEUE_SECONDS: usize = 14;

/// Un micrófono abierto y grabando. Al soltarlo, deja de grabar.
pub struct Mic {
    _stream: cpal::Stream,
    rx: rtrb::Consumer<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

impl Mic {
    /// Abre el micrófono por defecto y empieza a grabar.
    pub fn start() -> Result<Mic> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("no hay micrófono"))?;
        let default = device
            .default_input_config()
            .context("no se pudo leer la configuración del micrófono")?;
        let format = default.sample_format();
        let config: StreamConfig = default.into();
        let rate = config.sample_rate;
        let channels = config.channels as usize;
        let (tx, rx) = rtrb::RingBuffer::<f32>::new(rate as usize * channels * QUEUE_SECONDS);
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, &config, tx)?,
            SampleFormat::I16 => build::<i16>(&device, &config, tx)?,
            SampleFormat::I32 => build::<i32>(&device, &config, tx)?,
            SampleFormat::U16 => build::<u16>(&device, &config, tx)?,
            other => return Err(anyhow!("formato de micrófono no soportado: {other}")),
        };
        stream.play().context("no se pudo arrancar el micrófono")?;
        Ok(Mic {
            _stream: stream,
            rx,
            sample_rate: rate,
            channels,
        })
    }

    /// Saca lo grabado hasta ahora, en mono, y lo añade a `out`. Devuelve el pico del trozo.
    pub fn drain(&mut self, out: &mut Vec<f32>) -> f32 {
        let n = self.rx.slots() / self.channels * self.channels;
        let mut peak = 0.0f32;
        if let Ok(chunk) = self.rx.read_chunk(n) {
            let (a, b) = chunk.as_slices();
            let mut frame = Vec::with_capacity(self.channels);
            for &s in a.iter().chain(b) {
                frame.push(s);
                if frame.len() == self.channels {
                    let m = frame.iter().sum::<f32>() / self.channels as f32;
                    peak = peak.max(m.abs());
                    out.push(m);
                    frame.clear();
                }
            }
            chunk.commit_all();
        }
        peak
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut tx: rtrb::Producer<f32>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    Ok(device.build_input_stream(
        *config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            // Si la cola se llena, lo que sobra se pierde (nunca se bloquea).
            for &s in data {
                if tx.push(f32::from_sample(s)).is_err() {
                    break;
                }
            }
        },
        |err| eprintln!("Aviso del micrófono: {err}"),
        None,
    )?)
}
