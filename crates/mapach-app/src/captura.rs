//! Capturas de pantalla para depurar sin mirar la pantalla:
//! `mapach-app --ventana --captura salida.ppm --demo 1` (sin sonido).

use std::sync::Arc;

use eframe::egui::{self, ColorImage};
use mapach_input::{Arrow as A, Control, Event, Knob, Menu as M};

pub struct Captura {
    pub path: String,
    pub demo: u32,
    frame: u32,
}

impl Captura {
    pub fn from_args() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let get = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1).cloned())
        };
        Some(Self {
            path: get("--captura")?,
            demo: get("--demo").and_then(|d| d.parse().ok()).unwrap_or(0),
            frame: 0,
        })
    }

    /// Canciones de mentira para la escena de canciones.
    pub fn fake_songs(&self, songs: &mut crate::canciones::Songs) {
        songs.current.nombre = "Dragón Naranja".into();
        songs.list = [
            "Dragón Naranja",
            "Pulpo Saltarín",
            "Estrella Mágica",
            "Ballena Cósmica",
            "Gato Veloz",
        ]
        .iter()
        .enumerate()
        .map(|(i, n)| mapach_project::SongInfo {
            nombre: n.to_string(),
            color: mapach_project::names::COLORS[i * 3 % 8],
            guardada: 0,
            carpeta: std::path::PathBuf::new(),
            pistas: i + 1,
        })
        .collect();
        songs.last_saved = Some(0.0);
    }

    /// Eventos de cada escena de demostración.
    pub fn events(&self) -> Vec<Event> {
        use Control::*;
        let p = Event::Press;
        let r = Event::Release;
        match self.demo {
            1 => vec![p(Chord(5)), p(Arrow(A::Right))],
            2 => vec![p(Menu(M::Sonido)), r(Menu(M::Sonido)), p(Arrow(A::Down))],
            3 => vec![p(Menu(M::Tono))],
            4 => vec![
                p(Menu(M::Tono)),
                r(Menu(M::Tono)),
                p(Arrow(A::Down)),
                p(Arrow(A::Down)),
                p(Arrow(A::Down)),
                p(Arrow(A::Down)),
                p(Arrow(A::Down)),
                p(Arrow(A::Right)),
                p(Menu(M::Tono)),
                r(Menu(M::Tono)),
                p(Chord(0)),
                Event::Turn(Knob::Magica, 4),
            ],
            5 => vec![
                p(Menu(M::Modo)),
                p(Arrow(A::Right)),
                r(Arrow(A::Right)),
                r(Menu(M::Modo)),
                p(Chord(2)),
            ],
            6 => vec![
                p(Menu(M::Modo)),
                p(Arrow(A::Right)),
                r(Arrow(A::Right)),
                p(Arrow(A::Right)),
                r(Arrow(A::Right)),
                r(Menu(M::Modo)),
                p(Menu(M::Modo)),
                r(Menu(M::Modo)),
                p(Arrow(A::Down)),
                p(Arrow(A::Down)),
                p(Chord(3)),
            ],
            // Batería: pasar a modo Batería con el menú rojo y tocar la caja.
            7 => {
                let mut v = vec![p(Menu(M::Modo))];
                for _ in 0..6 {
                    v.push(p(Arrow(A::Right)));
                    v.push(r(Arrow(A::Right)));
                }
                v.extend([r(Menu(M::Modo)), p(Chord(1))]);
                v
            }
            8 => {
                let mut v = vec![p(Menu(M::Modo))];
                for _ in 0..7 {
                    v.push(p(Arrow(A::Right)));
                    v.push(r(Arrow(A::Right)));
                }
                v.push(r(Menu(M::Modo)));
                v
            }
            9 => {
                let mut v = vec![p(Menu(M::Modo))];
                for _ in 0..8 {
                    v.push(p(Arrow(A::Right)));
                    v.push(r(Arrow(A::Right)));
                }
                v.push(r(Menu(M::Modo)));
                for (row, steps) in [
                    (0u8, &[1u8, 7, 11][..]),
                    (1, &[5, 13]),
                    (2, &[1, 3, 5, 7, 9, 11, 13, 15]),
                ] {
                    v.push(p(Chord(row)));
                    v.push(r(Chord(row)));
                    for &st in steps {
                        v.push(p(Pad(st)));
                        v.push(r(Pad(st)));
                    }
                }
                v
            }
            10 => vec![p(Record), r(Record), p(Chord(0))],
            11 => vec![p(Canciones), p(Arrow(A::Right))],
            12 => vec![p(Canciones), p(Arrow(A::Down)), p(Arrow(A::Right))],
            _ => vec![],
        }
    }

    /// Llamar en cada fotograma. Devuelve `true` el fotograma en que hay que aplicar la demo.
    pub fn tick(&mut self, ctx: &egui::Context) -> bool {
        self.frame += 1;
        ctx.request_repaint();
        if self.frame == 30 {
            let path = self.path.clone();
            let ctx2 = ctx.clone();
            ctx.request_screenshot(move |img| {
                if let Err(e) = save_ppm(&path, &img) {
                    eprintln!("No se pudo guardar la captura: {e}");
                }
                ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
        self.frame == 5
    }
}

fn save_ppm(path: &str, img: &Arc<ColorImage>) -> std::io::Result<()> {
    let [w, h] = img.size;
    let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in &img.pixels {
        data.extend_from_slice(&[px.r(), px.g(), px.b()]);
    }
    std::fs::write(path, data)
}
