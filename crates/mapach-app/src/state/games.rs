//! Juegos *Cópiame* (memoria, tipo Simón) y *Sigue la canción*, y las estrellas.
//!
//! Ideas de Rainbow mode (ver `docs/investigacion.md` §6.4, punto 6). Equivocarse nunca castiga:
//! se vuelve a intentar.

use super::*;

/// Canciones para *Sigue la canción*: grados de la escala mayor (0 = tónica; 7–13, la fila de
/// arriba, una octava más aguda).
pub const SONGS: [(&str, &[u8]); 4] = [
    (
        "Estrellita",
        &[
            0, 0, 4, 4, 5, 5, 4, 3, 3, 2, 2, 1, 1, 0, 4, 4, 3, 3, 2, 2, 1, 4, 4, 3, 3, 2, 2, 1, 0,
            0, 4, 4, 5, 5, 4, 3, 3, 2, 2, 1, 1, 0,
        ],
    ),
    (
        "Himno de la alegría",
        &[
            2, 2, 3, 4, 4, 3, 2, 1, 0, 0, 1, 2, 2, 1, 1, 2, 2, 3, 4, 4, 3, 2, 1, 0, 0, 1, 2, 1, 0,
            0,
        ],
    ),
    (
        "Cumpleaños feliz",
        &[
            4, 4, 5, 4, 7, 6, 4, 4, 5, 4, 8, 7, 4, 4, 11, 9, 7, 6, 5, 10, 10, 9, 7, 8, 7,
        ],
    ),
    (
        "Martinillo",
        &[
            0, 1, 2, 0, 0, 1, 2, 0, 2, 3, 4, 2, 3, 4, 4, 5, 4, 3, 2, 0, 4, 5, 4, 3, 2, 0,
        ],
    ),
];

/// Estrellas que desbloquean cada sonido secreto (en el orden de [`SECRET_SOUNDS`]).
pub const UNLOCKS: [u32; 2] = [10, 25];

/// Cuántos sonidos del final de `SOUNDS` son secretos.
pub const SECRET_SOUNDS: usize = UNLOCKS.len();

/// *Cópiame*: suena una serie de acordes y hay que repetirla.
#[derive(Debug, Clone, PartialEq)]
pub struct Simon {
    pub seq: Vec<u8>,
    /// Teclas que entran en juego.
    pub keys: u8,
    /// Enseñando la serie: siguiente paso y cuándo.
    pub showing: Option<(usize, f64)>,
    /// Tecla que se ilumina ahora mientras se enseña.
    pub lit: Option<u8>,
    /// Por dónde va el niño al repetir.
    pub pos: usize,
    pub feedback: Option<(bool, f64)>,
    /// Serie más larga conseguida.
    pub best: usize,
}

/// *Sigue la canción*: se ilumina la siguiente tecla.
#[derive(Debug, Clone, PartialEq)]
pub struct Follow {
    pub song: usize,
    pub pos: usize,
    /// Canción terminada: se enseña la celebración hasta este momento.
    pub done_until: Option<f64>,
}

impl Follow {
    pub fn notes(&self) -> &'static [u8] {
        SONGS[self.song % SONGS.len()].1
    }

    pub fn title(&self) -> &'static str {
        SONGS[self.song % SONGS.len()].0
    }

    /// Siguiente grado a tocar.
    pub fn next(&self) -> Option<u8> {
        self.notes().get(self.pos).copied()
    }
}

impl State {
    /// ¿Hay algún juego en marcha?
    pub fn in_game(&self) -> bool {
        self.game.is_some() || self.simon.is_some() || self.follow.is_some()
    }

    /// Sale de cualquier juego.
    pub(super) fn quit_games(&mut self, out: &mut Vec<Command>) {
        self.game = None;
        self.simon = None;
        if self.follow.take().is_some() {
            for (_, n) in self.melody.drain(..) {
                out.push(Command::NoteOff {
                    note: n.clamp(0, 127) as u8,
                });
            }
        }
        out.push(Command::ChordOff);
    }

    /// Tecla que el juego ilumina (para la pantalla).
    pub fn highlight(&self) -> Option<(u8, bool)> {
        if let Some(s) = &self.simon {
            return s.lit.map(|b| (b, false));
        }
        let f = self.follow.as_ref()?;
        if f.done_until.is_some() {
            return None;
        }
        f.next().map(|d| (d % 7, d >= 7))
    }

    /// Una estrella más. Avisa si desbloquea un sonido secreto.
    pub(super) fn earn_star(&mut self) {
        self.stars += 1;
        if let Some(i) = UNLOCKS.iter().position(|&n| n == self.stars) {
            let snd = &SOUNDS[SOUNDS.len() - SECRET_SOUNDS + i];
            self.show_toast(format!("🎁 ¡Sonido secreto: {} {}!", snd.icon, snd.name));
        } else {
            self.show_toast(format!("⭐ ¡Estrella! Ya tienes {}", self.stars));
        }
    }

    /// Sonidos que se pueden elegir (los secretos, solo desbloqueados).
    pub fn available_sounds(&self) -> usize {
        let unlocked = UNLOCKS.iter().filter(|&&n| self.stars >= n).count();
        SOUNDS.len() - SECRET_SOUNDS + unlocked
    }

    // ---------- Cópiame ----------

    pub(super) fn start_simon(&mut self, out: &mut Vec<Command>) {
        self.quit_games(out);
        self.open_menu = None;
        self.set_play_mode(PlayMode::Tocar, out);
        self.simon = Some(Simon {
            seq: Vec::new(),
            keys: 3,
            showing: None,
            lit: None,
            pos: 0,
            feedback: None,
            best: 0,
        });
        self.simon_grow();
    }

    /// Añade un acorde a la serie y empieza a enseñarla.
    fn simon_grow(&mut self) {
        let keys = self.simon.as_ref().map_or(3, |s| s.keys);
        let b = self.next_rand(keys as usize) as u8;
        let now = self.now;
        if let Some(s) = &mut self.simon {
            s.seq.push(b);
            s.showing = Some((0, now + 0.6));
            s.pos = 0;
        }
    }

    pub(super) fn simon_tick(&mut self, now: f64, out: &mut Vec<Command>) {
        let Some(s) = &mut self.simon else { return };
        if let Some((ok, until)) = s.feedback
            && now >= until
        {
            s.feedback = None;
            if ok {
                self.simon_grow();
            } else {
                // Otra vez la misma serie.
                s.showing = Some((0, now + 0.3));
                s.pos = 0;
            }
            return;
        }
        let Some((i, at)) = s.showing else { return };
        if now < at {
            return;
        }
        if s.lit.take().is_some() {
            out.push(Command::ChordOff);
            s.showing = Some((i, now + 0.15));
        } else if let Some(&b) = s.seq.get(i) {
            s.lit = Some(b);
            s.showing = Some((i + 1, now + 0.55));
            self.knob_color = 0;
            self.play_chord(b, false, true, out);
            self.current = None;
        } else {
            // Te toca.
            s.showing = None;
        }
    }

    /// Respuesta en *Cópiame*. Devuelve `true` si hay que ignorar la tecla (se está enseñando).
    pub(super) fn simon_press(&mut self, b: u8) -> bool {
        let now = self.now;
        let Some(s) = &mut self.simon else {
            return false;
        };
        if s.showing.is_some() || s.feedback.is_some() {
            return true;
        }
        if s.seq.get(s.pos) == Some(&b) {
            s.pos += 1;
            if s.pos == s.seq.len() {
                s.best = s.best.max(s.seq.len());
                s.feedback = Some((true, now + 1.0));
                let len = s.seq.len();
                // Más teclas cuando la serie crece.
                if len.is_multiple_of(3) && s.keys < 7 {
                    s.keys += 1;
                }
                if len.is_multiple_of(4) {
                    self.earn_star();
                }
            }
        } else {
            s.feedback = Some((false, now + 1.3));
        }
        false
    }

    // ---------- Sigue la canción ----------

    pub(super) fn start_follow(&mut self, out: &mut Vec<Command>) {
        let next_song = self.follow.as_ref().map_or(0, |f| f.song + 1);
        self.quit_games(out);
        self.open_menu = None;
        self.set_play_mode(PlayMode::Tocar, out);
        self.follow = Some(Follow {
            song: next_song % SONGS.len(),
            pos: 0,
            done_until: None,
        });
    }

    /// Nota de un grado en *Sigue la canción* (siempre escala mayor de la tonalidad actual).
    fn follow_note(&self, degree: u8) -> i32 {
        Key::new(self.key.tonic, Scale::Major).scale_note(degree as i32) + 12 * self.key.octave
    }

    pub(super) fn follow_press(&mut self, c: Control, out: &mut Vec<Command>) {
        let now = self.now;
        let degree = match c {
            Control::Chord(b) if (b as usize) < BUTTONS => b,
            Control::ChordHigh(b) if (b as usize) < BUTTONS => b + 7,
            _ => return,
        };
        let note = self.follow_note(degree);
        self.melody.retain(|(x, _)| *x != c);
        self.melody.push((c, note));
        out.push(Command::NoteOn {
            note: note.clamp(0, 127) as u8,
            velocity: 0.9,
        });
        let Some(f) = &mut self.follow else { return };
        if f.done_until.is_none() && f.next() == Some(degree) {
            f.pos += 1;
            if f.next().is_none() {
                f.done_until = Some(now + 2.5);
                self.earn_star();
            }
        }
    }

    pub(super) fn follow_tick(&mut self, now: f64) {
        let Some(f) = &mut self.follow else { return };
        if let Some(until) = f.done_until
            && now >= until
        {
            // Siguiente canción.
            f.song = (f.song + 1) % SONGS.len();
            f.pos = 0;
            f.done_until = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(s: &mut State, c: Control) -> Vec<Command> {
        s.handle(Event::Press(c), s.now)
    }
    fn release(s: &mut State, c: Control) -> Vec<Command> {
        s.handle(Event::Release(c), s.now)
    }
    fn tick(s: &mut State, dt: f64) -> Vec<Command> {
        let now = s.now + dt;
        s.sync(LooperStatus::default(), now)
    }

    #[test]
    fn canciones_validas() {
        for (name, notes) in SONGS {
            assert!(!notes.is_empty(), "{name}");
            assert!(notes.iter().all(|&d| d < 14), "{name}");
        }
    }

    #[test]
    fn copiame() {
        let mut s = State::default();
        tick(&mut s, 0.0);
        s.change(Item::Copiame, 1, &mut Vec::new());
        assert!(s.in_game());
        // Enseña la serie (1 acorde) y se ilumina.
        let mut lit = None;
        for _ in 0..20 {
            tick(&mut s, 0.2);
            if let Some((b, _)) = s.highlight() {
                lit = Some(b);
            }
        }
        let first = lit.unwrap();
        assert!(
            s.simon.as_ref().unwrap().showing.is_none(),
            "ya le toca al niño"
        );
        // Repetirla bien: crece.
        press(&mut s, Control::Chord(first));
        release(&mut s, Control::Chord(first));
        tick(&mut s, 1.1);
        assert_eq!(s.simon.as_ref().unwrap().seq.len(), 2);
        // Mientras enseña, las teclas no cuentan.
        let pos = s.simon.as_ref().unwrap().pos;
        press(&mut s, Control::Chord(6));
        assert_eq!(s.simon.as_ref().unwrap().pos, pos);
        release(&mut s, Control::Chord(6));
        for _ in 0..20 {
            tick(&mut s, 0.2);
        }
        // Equivocarse: se repite la misma serie.
        let seq = s.simon.as_ref().unwrap().seq.clone();
        let wrong = (seq[0] + 1) % 3;
        press(&mut s, Control::Chord(wrong));
        release(&mut s, Control::Chord(wrong));
        tick(&mut s, 1.5);
        assert_eq!(s.simon.as_ref().unwrap().seq, seq);
        // Salir.
        press(&mut s, Control::KnobPress(Knob::Cursor));
        assert!(!s.in_game());
    }

    #[test]
    fn sigue_la_cancion_y_estrella() {
        let mut s = State::default();
        tick(&mut s, 0.0);
        s.change(Item::Sigue, 1, &mut Vec::new());
        let notes = s.follow.as_ref().unwrap().notes();
        assert_eq!(s.follow.as_ref().unwrap().title(), "Estrellita");
        // Una tecla equivocada suena pero no avanza.
        let wrong = (notes[0] + 1) % 7;
        let c = press(&mut s, Control::Chord(wrong));
        assert!(matches!(c[0], Command::NoteOn { .. }));
        release(&mut s, Control::Chord(wrong));
        assert_eq!(s.follow.as_ref().unwrap().pos, 0);
        for &d in notes {
            assert_eq!(s.highlight(), Some((d % 7, d >= 7)));
            let c = if d >= 7 {
                Control::ChordHigh(d - 7)
            } else {
                Control::Chord(d)
            };
            press(&mut s, c);
            release(&mut s, c);
        }
        assert_eq!(s.stars, 1);
        assert!(s.highlight().is_none(), "celebración");
        tick(&mut s, 3.0);
        assert_eq!(s.follow.as_ref().unwrap().title(), "Himno de la alegría");
    }

    #[test]
    fn sonidos_secretos() {
        let mut s = State::default();
        let normal = SOUNDS.len() - SECRET_SOUNDS;
        assert_eq!(s.available_sounds(), normal);
        // Dar la vuelta a los sonidos no llega a los secretos.
        s.change(Item::Sonido, -1, &mut Vec::new());
        assert_eq!(s.sound, normal - 1);
        s.stars = 9;
        s.earn_star();
        assert_eq!(s.available_sounds(), normal + 1);
        assert!(s.toast_text().unwrap().contains("secreto"));
    }
}
