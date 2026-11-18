//! Taller de samples: biblioteca, micro, destrozar, deshacer y usar como instrumento.

use std::path::PathBuf;
use std::sync::Arc;

use mapach_engine::Command;
use mapach_engine::mic::Mic;
use mapach_samples::destroy::Destroy;
use mapach_samples::library::{self, Entry};
use mapach_samples::resample::resample;
use mapach_samples::{MAX_SECONDS, RATE, Sample, pitch};

use crate::canciones::Link;

/// Columnas del dibujo de la forma de onda.
pub const OVERVIEW: usize = 120;

pub struct Taller {
    pub list: Vec<Entry>,
    /// Sample elegido: posición en la lista y audio.
    pub current: Option<(usize, Sample)>,
    pub overview: Vec<f32>,
    /// Nota raíz detectada del sample elegido.
    pub root: Option<f32>,
    undo: Vec<Sample>,
    mic: Option<Mic>,
    rec: Vec<f32>,
    rec_started: f64,
    /// Nivel del micro (para la barrita).
    pub rec_level: f32,
    /// Segundos grabados.
    pub rec_seconds: f32,
    dir: PathBuf,
    import: PathBuf,
}

impl Taller {
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            current: None,
            overview: vec![0.0; OVERVIEW],
            root: None,
            undo: Vec::new(),
            mic: None,
            rec: Vec::new(),
            rec_started: 0.0,
            rec_level: 0.0,
            rec_seconds: 0.0,
            dir: library::samples_dir(),
            import: library::import_dir(),
        }
    }

    /// Usa otras carpetas (para las capturas de demostración).
    pub fn set_dirs(&mut self, dir: PathBuf, import: PathBuf) {
        self.dir = dir;
        self.import = import;
    }

    pub fn is_recording(&self) -> bool {
        self.mic.is_some()
    }

    /// Al abrir: importa lo que haya en la carpeta Importar y lee la biblioteca.
    /// Devuelve un mensaje si se ha importado algo.
    pub fn open(&mut self) -> Option<String> {
        let _ = std::fs::create_dir_all(&self.import);
        let imported = library::import_pending(&self.import, &self.dir);
        self.refresh();
        let ok = imported.iter().filter(|i| i.result.is_ok()).count();
        let bad = imported.len() - ok;
        for i in &imported {
            if let Err(e) = &i.result {
                eprintln!("No se pudo importar {}: {e:#}", i.file);
            }
        }
        match (ok, bad) {
            (0, 0) => None,
            (n, 0) => Some(format!("📥 {n} sonido(s) importado(s)")),
            (n, b) => Some(format!("📥 {n} importado(s), {b} no se pudo")),
        }
    }

    fn refresh(&mut self) {
        self.list = library::list_in(&self.dir);
    }

    /// Elige y carga el sample `i`.
    pub fn select(&mut self, i: usize) {
        self.undo.clear();
        self.current = None;
        if let Some(e) = self.list.get(i) {
            match library::load(&e.path) {
                Ok(s) => {
                    self.current = Some((i, s));
                }
                Err(e) => eprintln!("No se pudo leer el sample: {e:#}"),
            }
        }
        self.update_view();
    }

    fn update_view(&mut self) {
        match &self.current {
            Some((_, s)) => {
                self.overview = s.overview(OVERVIEW);
                self.root = pitch::detect(&s.data).map(pitch::freq_to_midi);
            }
            None => {
                self.overview = vec![0.0; OVERVIEW];
                self.root = None;
            }
        }
    }

    pub fn preview(&self, link: &mut dyn Link) {
        if let Some((_, s)) = &self.current {
            link.send(Command::Preview(Some(Arc::from(s.data.as_slice()))));
        }
    }

    pub fn stop_preview(&self, link: &mut dyn Link) {
        link.send(Command::Preview(None));
    }

    /// Destroza el sample elegido y lo guarda. Devuelve el nombre del botón.
    pub fn destroy(&mut self, b: usize, link: &mut dyn Link) -> Option<&'static str> {
        let d = Destroy::ALL[b % Destroy::ALL.len()];
        let (i, s) = self.current.as_mut()?;
        let new = d.apply(s);
        if new.is_empty() {
            return None;
        }
        self.undo.push(std::mem::replace(s, new));
        // Como mucho 10 pasos de deshacer.
        if self.undo.len() > 10 {
            self.undo.remove(0);
        }
        let i = *i;
        self.save_current(i);
        self.update_view();
        self.preview(link);
        Some(d.name())
    }

    pub fn undo(&mut self, link: &mut dyn Link) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        let Some((i, s)) = self.current.as_mut() else {
            return false;
        };
        *s = prev;
        let i = *i;
        self.save_current(i);
        self.update_view();
        self.preview(link);
        true
    }

    /// Guarda el sample elegido (si era un MP3, OGG o FLAC, como WAV nuevo).
    fn save_current(&mut self, i: usize) {
        let Some((_, s)) = &self.current else { return };
        let Some(entry) = self.list.get_mut(i) else {
            return;
        };
        let res = library::editable_path(&entry.path).and_then(|p| {
            library::overwrite(&p, s)?;
            Ok(p)
        });
        match res {
            Ok(p) => entry.path = p,
            Err(e) => eprintln!("No se pudo guardar el sample: {e:#}"),
        }
    }

    /// Datos y raíz del sample elegido, para usarlo como instrumento.
    pub fn for_instrument(&self) -> Option<(Arc<[f32]>, f32, String, PathBuf)> {
        let (i, s) = self.current.as_ref()?;
        let e = self.list.get(*i)?;
        Some((
            Arc::from(s.data.as_slice()),
            pitch::root_note(&s.data),
            e.name.clone(),
            e.path.clone(),
        ))
    }

    /// Empieza o termina de grabar. Al terminar, guarda "Mi voz" y lo elige.
    /// Devuelve el índice del sample nuevo.
    pub fn mic_toggle(&mut self, now: f64) -> Result<Option<usize>, String> {
        if self.mic.is_some() {
            return Ok(self.finish_recording());
        }
        match Mic::start() {
            Ok(m) => {
                self.mic = Some(m);
                self.rec.clear();
                self.rec_started = now;
                self.rec_seconds = 0.0;
                Ok(None)
            }
            Err(e) => Err(format!("{e:#}")),
        }
    }

    /// Llamar en cada fotograma: recoge lo grabado y para solo a los 10 s.
    pub fn tick(&mut self, now: f64) -> Option<usize> {
        let mic = self.mic.as_mut()?;
        self.rec_level = mic.drain(&mut self.rec).max(self.rec_level * 0.9);
        self.rec_seconds = self.rec.len() as f32 / mic.sample_rate as f32;
        if now - self.rec_started >= MAX_SECONDS as f64 {
            return self.finish_recording();
        }
        None
    }

    fn finish_recording(&mut self) -> Option<usize> {
        let mut mic = self.mic.take()?;
        mic.drain(&mut self.rec);
        let rate = mic.sample_rate;
        drop(mic);
        let mut s = Sample::new(resample(&self.rec, rate, RATE));
        self.rec.clear();
        self.rec_level = 0.0;
        s.tidy();
        if s.is_empty() {
            return None;
        }
        match library::save_in(&self.dir, "Mi voz", &s) {
            Ok(path) => {
                self.refresh();
                let i = self.list.iter().position(|e| e.path == path)?;
                self.select(i);
                Some(i)
            }
            Err(e) => {
                eprintln!("No se pudo guardar la grabación: {e:#}");
                None
            }
        }
    }
}

/// Carga el sample guardado como instrumento (al arrancar).
pub fn load_instrument(path: &std::path::Path) -> Option<(Arc<[f32]>, f32)> {
    let s = library::load(path).ok()?;
    Some((Arc::from(s.data.as_slice()), pitch::root_note(&s.data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapach_engine::Reply;

    /// Apunta las órdenes que se mandan al motor.
    #[derive(Default)]
    struct Spy(Vec<Command>);

    impl Link for Spy {
        fn send(&mut self, c: Command) {
            self.0.push(c);
        }
        fn reply(&mut self) -> Option<Reply> {
            None
        }
        fn capacity(&self) -> usize {
            0
        }
    }

    #[test]
    fn importar_destrozar_deshacer_y_usar() {
        let base = std::env::temp_dir().join(format!("mapach-taller-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (dir, import) = (base.join("Samples"), base.join("Importar"));
        std::fs::create_dir_all(&import).unwrap();
        // Un WAV en Importar.
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(import.join("silbido.wav"), spec).unwrap();
        for i in 0..24_000 {
            let v = (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin();
            w.write_sample((v * 12_000.0) as i16).unwrap();
        }
        w.finalize().unwrap();

        let mut t = Taller::new();
        t.set_dirs(dir.clone(), import);
        assert!(t.open().unwrap().contains('1'));
        assert_eq!(t.list.len(), 1);
        t.select(0);
        let root = t.root.unwrap();
        assert!((root - 69.0).abs() < 0.3, "{root}");

        let mut spy = Spy::default();
        let before = t.current.as_ref().unwrap().1.data.len();
        assert_eq!(t.destroy(8, &mut spy), Some("Lento"));
        let after = t.current.as_ref().unwrap().1.data.len();
        assert!(after > before);
        // Se ha guardado en el fichero y se escucha.
        assert!(matches!(spy.0.last(), Some(Command::Preview(Some(_)))));
        assert_eq!(library::load(&t.list[0].path).unwrap().data.len(), after);

        assert!(t.undo(&mut spy));
        assert_eq!(t.current.as_ref().unwrap().1.data.len(), before);
        assert!(!t.undo(&mut spy));

        let (data, root, name, _) = t.for_instrument().unwrap();
        assert_eq!(name, "silbido");
        assert_eq!(data.len(), before);
        assert!((root - 69.0).abs() < 0.3);
        std::fs::remove_dir_all(base).unwrap();
    }
}
