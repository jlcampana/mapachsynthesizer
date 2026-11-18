//! Conexión del [`Engine`] con la tarjeta de sonido mediante `cpal`.
//!
//! Reglas del hilo de audio: nada de bloqueos, reservas de memoria ni E/S en el callback.
//! Las órdenes llegan por un ring buffer `rtrb` y las medidas salen por atómicos.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig};

use crate::looper::LooperStatus;
use crate::{Command, Engine, Reply};

/// Tamaño de bloque pedido a la tarjeta: 256 muestras a 48 kHz ≈ 5,3 ms.
pub const PREFERRED_BUFFER: u32 = 256;

/// Tamaño máximo de bloque que procesamos de una vez (en frames).
const MAX_BLOCK: usize = 4096;

/// Medidas que el hilo de audio publica para la interfaz.
#[derive(Debug, Default)]
pub struct AudioStats {
    /// Frames del último callback.
    pub block_frames: AtomicU32,
    /// Latencia de salida estimada por `cpal`, en microsegundos.
    pub output_latency_us: AtomicU32,
    /// Pico de salida del último bloque (bits de un `f32`).
    pub peak_bits: AtomicU32,
    /// Voces sonando.
    pub voices: AtomicU32,
    /// Paso de la caja de ritmos + 1 (0 = parada).
    pub drum_step: AtomicU32,
    /// Estado del looper empaquetado ([`LooperStatus::pack`]).
    pub looper: AtomicU64,
}

/// Una salida de audio en marcha. Mientras exista, suena.
pub struct AudioOutput {
    _stream: cpal::Stream,
    pub commands: rtrb::Producer<Command>,
    /// Respuestas del motor (pistas copiadas, búferes para liberar).
    pub replies: rtrb::Consumer<Reply>,
    /// Frames que caben en una pista del looper.
    pub looper_capacity: usize,
    pub stats: Arc<AudioStats>,
    pub sample_rate: u32,
    pub channels: u16,
    pub requested_buffer: Option<u32>,
    pub device_name: String,
}

impl AudioOutput {
    /// Manda una orden al motor. Si la cola está llena, la orden se pierde (no bloquea nunca).
    pub fn send(&mut self, cmd: Command) {
        let _ = self.commands.push(cmd);
    }

    /// Saca una respuesta del motor, si hay.
    pub fn reply(&mut self) -> Option<Reply> {
        self.replies.pop().ok()
    }

    /// Latencia de salida estimada en milisegundos.
    pub fn latency_ms(&self) -> f32 {
        self.stats.output_latency_us.load(Ordering::Relaxed) as f32 / 1000.0
    }

    /// Pico de salida del último bloque (lineal).
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.stats.peak_bits.load(Ordering::Relaxed))
    }

    /// Paso de la caja de ritmos que suena.
    pub fn drum_step(&self) -> Option<usize> {
        match self.stats.drum_step.load(Ordering::Relaxed) {
            0 => None,
            n => Some(n as usize - 1),
        }
    }

    pub fn looper(&self) -> LooperStatus {
        LooperStatus::unpack(self.stats.looper.load(Ordering::Relaxed))
    }

    /// Voces sonando.
    pub fn voices(&self) -> u32 {
        self.stats.voices.load(Ordering::Relaxed)
    }

    /// Frames del último bloque procesado.
    pub fn block_frames(&self) -> u32 {
        self.stats.block_frames.load(Ordering::Relaxed)
    }
}

/// Abre la salida por defecto y pone el motor a sonar.
pub fn start() -> Result<AudioOutput> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no hay salida de audio"))?;
    let device_name = device
        .id()
        .map(|id| id.to_string())
        .unwrap_or_else(|_| "desconocido".into());
    let default = device
        .default_output_config()
        .context("no se pudo leer la configuración de audio")?;
    let format = default.sample_format();
    let mut config: StreamConfig = default.into();

    // Primero probamos con un bloque pequeño; si la tarjeta no quiere, usamos el de serie.
    config.buffer_size = BufferSize::Fixed(PREFERRED_BUFFER);
    match build(&device, &config, format) {
        Ok(built) => finish(built, &config, Some(PREFERRED_BUFFER), device_name),
        Err(e) => {
            eprintln!("Bloque de {PREFERRED_BUFFER} rechazado ({e}); uso el de serie");
            config.buffer_size = BufferSize::Default;
            let built = build(&device, &config, format)?;
            finish(built, &config, None, device_name)
        }
    }
}

struct Built {
    stream: cpal::Stream,
    commands: rtrb::Producer<Command>,
    replies: rtrb::Consumer<Reply>,
    looper_capacity: usize,
    stats: Arc<AudioStats>,
}

fn finish(
    b: Built,
    config: &StreamConfig,
    requested_buffer: Option<u32>,
    device_name: String,
) -> Result<AudioOutput> {
    b.stream.play().context("no se pudo arrancar el audio")?;
    Ok(AudioOutput {
        _stream: b.stream,
        commands: b.commands,
        replies: b.replies,
        looper_capacity: b.looper_capacity,
        stats: b.stats,
        sample_rate: config.sample_rate,
        channels: config.channels,
        requested_buffer,
        device_name,
    })
}

fn build(device: &cpal::Device, config: &StreamConfig, format: SampleFormat) -> Result<Built> {
    match format {
        SampleFormat::F32 => build_typed::<f32>(device, config),
        SampleFormat::I16 => build_typed::<i16>(device, config),
        SampleFormat::I32 => build_typed::<i32>(device, config),
        SampleFormat::U16 => build_typed::<u16>(device, config),
        other => Err(anyhow!("formato de audio no soportado: {other}")),
    }
}

fn build_typed<T>(device: &cpal::Device, config: &StreamConfig) -> Result<Built>
where
    T: SizedSample + FromSample<f32>,
{
    let (producer, mut consumer) = rtrb::RingBuffer::<Command>::new(1024);
    let (mut reply_tx, reply_rx) = rtrb::RingBuffer::<Reply>::new(16);
    let stats = Arc::new(AudioStats::default());
    let stats_cb = stats.clone();
    let channels = config.channels as usize;
    let mut engine = Engine::new(config.sample_rate as f32);
    let looper_capacity = engine.looper_capacity();
    // Buffer estéreo reservado antes de arrancar: el callback no reserva memoria.
    let mut scratch = vec![0.0f32; MAX_BLOCK * 2];

    let stream = device.build_output_stream(
        *config,
        move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
            while let Ok(cmd) = consumer.pop() {
                engine.handle(cmd);
            }
            let frames = data.len() / channels;
            stats_cb
                .block_frames
                .store(frames as u32, Ordering::Relaxed);
            let ts = info.timestamp();
            let lat = ts.playback.duration_since(ts.callback);
            stats_cb
                .output_latency_us
                .store(lat.as_micros() as u32, Ordering::Relaxed);

            let mut peak = 0.0f32;
            for chunk_out in data.chunks_mut(MAX_BLOCK * channels) {
                let n = chunk_out.len() / channels;
                let stereo = &mut scratch[..n * 2];
                engine.process(stereo);
                peak = peak.max(engine.peak());
                for (frame, s) in chunk_out
                    .chunks_exact_mut(channels)
                    .zip(stereo.chunks_exact(2))
                {
                    for (c, out) in frame.iter_mut().enumerate() {
                        let v = if c % 2 == 0 { s[0] } else { s[1] };
                        *out = T::from_sample(v.clamp(-1.0, 1.0));
                    }
                }
            }
            // Respuestas: si la cola está llena, esperan al siguiente bloque.
            while !reply_tx.is_full() {
                match engine.take_reply() {
                    Some(r) => {
                        let _ = reply_tx.push(r);
                    }
                    None => break,
                }
            }
            stats_cb.peak_bits.store(peak.to_bits(), Ordering::Relaxed);
            stats_cb
                .looper
                .store(engine.looper_status().pack(), Ordering::Relaxed);
            stats_cb.drum_step.store(
                engine.drum_step().map_or(0, |s| s as u32 + 1),
                Ordering::Relaxed,
            );
            stats_cb
                .voices
                .store(engine.active_voices() as u32, Ordering::Relaxed);
        },
        |err| eprintln!("Aviso de audio: {err}"),
        None,
    )?;
    Ok(Built {
        stream,
        commands: producer,
        replies: reply_rx,
        looper_capacity,
        stats,
    })
}
