//! El intérprete: decide cuándo suena cada nota del acorde según el modo de toque
//! (normal, rasgueo, arpegio, repetir, zumbido), sincronizado con el tempo.
//!
//! Trabaja en muestras, así que el tiempo es exacto y el resultado determinista.

/// Máximo de notas en un acorde.
pub const MAX_CHORD: usize = 8;

/// Patrones de arpegio (índices de nota del acorde de grave a agudo; si el acorde tiene menos
/// notas, se sigue en la octava de arriba). Vienen de fm1-chord.
pub const ARP_PATTERNS: [(&str, &[u8]); 6] = [
    ("Sube", &[0, 1, 2, 3]),
    ("Baja", &[3, 2, 1, 0]),
    ("Sube y baja", &[0, 1, 2, 3, 2, 1]),
    ("Baja y sube", &[3, 2, 1, 0, 1, 2]),
    ("Al azar", &[0, 1, 2, 3]),
    ("Guitarra", &[0, 2, 1, 2, 3, 2, 1, 2]),
];

/// Índice del patrón "al azar".
const ARP_RANDOM: u8 = 4;

/// Figuras rítmicas: pasos por negra.
pub const RATES: [(&str, f32); 5] = [
    ("Negras", 1.0),
    ("Corcheas", 2.0),
    ("Tresillos", 3.0),
    ("Semicorcheas", 4.0),
    ("Fusas", 8.0),
];

/// Velocidades del rasgueo, en ms entre nota y nota.
pub const STRUM_MS: [(&str, f32); 3] = [("Lento", 200.0), ("Medio", 80.0), ("Rápido", 40.0)];

/// Modo de toque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Todas las notas a la vez.
    Normal,
    /// Las notas una detrás de otra, de grave a agudo.
    Strum { speed: u8 },
    /// Arpegio con patrón y figura.
    Arp { pattern: u8, rate: u8 },
    /// Repite el acorde entero al ritmo.
    Repeat { rate: u8 },
    /// Zumbido: el acorde sigue sonando al soltar, hasta el siguiente.
    Drone,
}

/// Lo que el intérprete pide al motor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Out {
    On(u8, f32),
    Off(u8),
}

#[derive(Debug, Clone)]
pub struct Performer {
    sample_rate: f32,
    style: Style,
    bpm: f32,
    /// Acorde actual (bajo aparte), ordenado de grave a agudo.
    notes: [u8; MAX_CHORD],
    len: usize,
    bass: Option<u8>,
    velocity: f32,
    held: bool,
    /// Notas encendidas por el intérprete (bit por nota MIDI).
    on: u128,
    /// Muestras hasta el siguiente paso.
    countdown: f64,
    /// Muestras hasta apagar la nota del paso (puerta).
    gate: f64,
    step: usize,
    /// Notas que faltan por rasguear.
    strum_next: usize,
    rng: u32,
}

impl Performer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            style: Style::Normal,
            bpm: 120.0,
            notes: [0; MAX_CHORD],
            len: 0,
            bass: None,
            velocity: 0.85,
            held: false,
            on: 0,
            countdown: 0.0,
            gate: f64::INFINITY,
            step: 0,
            strum_next: 0,
            rng: 0x9E37_79B9,
        }
    }

    pub fn style(&self) -> Style {
        self.style
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 400.0);
    }

    /// Cambia el modo de toque. Si cambia de tipo, se para lo que sonaba.
    pub fn set_style(&mut self, style: Style, out: &mut impl FnMut(Out)) {
        if std::mem::discriminant(&style) != std::mem::discriminant(&self.style) {
            self.all_off(out);
            self.held = false;
            self.len = 0;
        }
        self.style = style;
    }

    /// Muestras por paso de la figura `rate`.
    fn step_samples(&self, rate: u8) -> f64 {
        let per_beat = RATES[rate as usize % RATES.len()].1 as f64;
        60.0 / self.bpm as f64 / per_beat * self.sample_rate as f64
    }

    fn note_on(&mut self, n: u8, v: f32, out: &mut impl FnMut(Out)) {
        self.on |= 1u128 << (n & 127);
        out(Out::On(n, v));
    }

    fn note_off(&mut self, n: u8, out: &mut impl FnMut(Out)) {
        if self.on & (1u128 << (n & 127)) != 0 {
            self.on &= !(1u128 << (n & 127));
            out(Out::Off(n));
        }
    }

    fn all_off(&mut self, out: &mut impl FnMut(Out)) {
        for n in 0..128u8 {
            self.note_off(n, out);
        }
    }

    /// Nota del paso `i` del arpegio.
    fn arp_note(&self, idx: u8) -> u8 {
        let i = idx as usize;
        let n = self.notes[i % self.len] as usize + 12 * (i / self.len);
        n.min(127) as u8
    }

    /// Empieza (o cambia) el acorde. `notes` sin el bajo. `retrigger`: tecla nueva (vuelve a
    /// atacar todo); si no, solo cambia lo que haga falta (al mover una flecha).
    pub fn chord_on(
        &mut self,
        notes: &[u8],
        bass: Option<u8>,
        velocity: f32,
        retrigger: bool,
        out: &mut impl FnMut(Out),
    ) {
        let mut sorted = [0u8; MAX_CHORD];
        let len = notes.len().min(MAX_CHORD);
        sorted[..len].copy_from_slice(&notes[..len]);
        sorted[..len].sort_unstable();
        let old_on = self.on;
        let old_bass = self.bass;
        self.notes = sorted;
        self.len = len;
        self.bass = bass;
        self.velocity = velocity;
        self.held = true;
        if len == 0 {
            self.all_off(out);
            return;
        }
        match self.style {
            Style::Normal | Style::Drone => {
                let mut want: u128 = 0;
                for &n in self.notes[..len].iter().chain(bass.iter()) {
                    want |= 1u128 << (n & 127);
                }
                for n in 0..128u8 {
                    let bit = 1u128 << n;
                    let was = old_on & bit != 0;
                    let wanted = want & bit != 0;
                    if was && (!wanted || retrigger) {
                        self.note_off(n, out);
                    }
                    if wanted && (!was || retrigger) {
                        self.note_on(n, velocity, out);
                    }
                }
            }
            Style::Strum { .. } => {
                if retrigger {
                    self.all_off(out);
                    if let Some(b) = bass {
                        self.note_on(b, velocity, out);
                    }
                    self.strum_next = 0;
                    self.countdown = 0.0;
                } else {
                    // Mover una flecha: cambiamos a la vez lo que ya había sonado.
                    self.retarget_sounding(old_on, out);
                }
            }
            Style::Arp { .. } => {
                if retrigger {
                    self.all_off(out);
                    if let Some(b) = bass {
                        self.note_on(b, velocity, out);
                    }
                    self.step = 0;
                    self.countdown = 0.0;
                } else if old_bass != bass {
                    // Solo el bajo cambia ya; el resto, en el siguiente paso.
                    if let Some(ob) = old_bass {
                        self.note_off(ob, out);
                    }
                    if let Some(b) = bass {
                        self.note_on(b, velocity, out);
                    }
                }
            }
            Style::Repeat { .. } => {
                if retrigger {
                    self.countdown = 0.0;
                    self.all_off(out);
                } else {
                    self.retarget_sounding(old_on, out);
                }
            }
        }
    }

    /// Cambia las notas que suenan por las del acorde nuevo (sin volver a atacar las comunes).
    fn retarget_sounding(&mut self, old_on: u128, out: &mut impl FnMut(Out)) {
        if old_on == 0 {
            return;
        }
        let mut want: u128 = 0;
        for &n in self.notes[..self.len].iter().chain(self.bass.iter()) {
            want |= 1u128 << (n & 127);
        }
        for n in 0..128u8 {
            let bit = 1u128 << n;
            if old_on & bit != 0 && want & bit == 0 {
                self.note_off(n, out);
            }
            if want & bit != 0 && old_on & bit == 0 {
                self.note_on(n, self.velocity, out);
            }
        }
    }

    /// Se suelta la tecla.
    pub fn chord_off(&mut self, out: &mut impl FnMut(Out)) {
        self.held = false;
        if self.style != Style::Drone {
            self.all_off(out);
            self.len = 0;
        }
    }

    /// Para todo (también el zumbido).
    pub fn stop(&mut self, out: &mut impl FnMut(Out)) {
        self.held = false;
        self.len = 0;
        self.all_off(out);
    }

    /// Avanza `n` muestras y lanza las notas que toquen.
    pub fn advance(&mut self, n: usize, out: &mut impl FnMut(Out)) {
        let n = n as f64;
        match self.style {
            Style::Normal | Style::Drone => {}
            Style::Strum { speed } => {
                if !self.held || self.strum_next >= self.len {
                    return;
                }
                while self.countdown <= 0.0 && self.strum_next < self.len {
                    let note = self.notes[self.strum_next];
                    self.note_on(note, self.velocity, out);
                    self.strum_next += 1;
                    let ms = STRUM_MS[speed as usize % STRUM_MS.len()].1;
                    self.countdown += ms as f64 / 1000.0 * self.sample_rate as f64;
                }
                self.countdown -= n;
            }
            Style::Arp { pattern, rate } => {
                if !self.held || self.len == 0 {
                    return;
                }
                if self.gate <= 0.0 {
                    self.arp_gate_off(out);
                    self.gate = f64::INFINITY;
                }
                while self.countdown <= 0.0 {
                    let step_len = self.step_samples(rate);
                    self.arp_gate_off(out);
                    let pat = ARP_PATTERNS[pattern as usize % ARP_PATTERNS.len()].1;
                    let idx = if pattern == ARP_RANDOM {
                        self.rng ^= self.rng << 13;
                        self.rng ^= self.rng >> 17;
                        self.rng ^= self.rng << 5;
                        (self.rng % self.len.max(3) as u32) as u8
                    } else {
                        pat[self.step % pat.len()]
                    };
                    let note = self.arp_note(idx);
                    // El primer paso de cada vuelta, un poco más fuerte.
                    let accent = if self.step.is_multiple_of(pat.len()) {
                        1.0
                    } else {
                        0.8
                    };
                    self.note_on(note, self.velocity * accent, out);
                    self.step += 1;
                    self.countdown += step_len;
                    self.gate = step_len * 0.6 + self.countdown.min(0.0);
                }
                self.countdown -= n;
                self.gate -= n;
            }
            Style::Repeat { rate } => {
                if !self.held || self.len == 0 {
                    return;
                }
                if self.gate <= 0.0 {
                    self.repeat_off(out);
                    self.gate = f64::INFINITY;
                }
                while self.countdown <= 0.0 {
                    let step_len = self.step_samples(rate);
                    self.repeat_off(out);
                    for i in 0..self.len {
                        let note = self.notes[i];
                        self.note_on(note, self.velocity, out);
                    }
                    if let Some(b) = self.bass {
                        self.note_on(b, self.velocity, out);
                    }
                    self.countdown += step_len;
                    self.gate = step_len * 0.5 + self.countdown.min(0.0);
                }
                self.countdown -= n;
                self.gate -= n;
            }
        }
    }

    /// Apaga las notas del arpegio (no el bajo).
    fn arp_gate_off(&mut self, out: &mut impl FnMut(Out)) {
        for n in 0..128u8 {
            if Some(n) != self.bass {
                self.note_off(n, out);
            }
        }
    }

    fn repeat_off(&mut self, out: &mut impl FnMut(Out)) {
        self.all_off(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn collect(p: &mut Performer, secs: f32) -> Vec<(usize, Out)> {
        let mut v = Vec::new();
        let blocks = (secs * SR / 32.0) as usize;
        for b in 0..blocks {
            p.advance(32, &mut |o| v.push((b * 32, o)));
        }
        v
    }

    fn ons(v: &[(usize, Out)]) -> Vec<(usize, u8)> {
        v.iter()
            .filter_map(|&(t, o)| match o {
                Out::On(n, _) => Some((t, n)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn normal_enciende_y_apaga_todo() {
        let mut p = Performer::new(SR);
        let mut v = Vec::new();
        p.chord_on(&[60, 64, 67], Some(48), 1.0, true, &mut |o| v.push(o));
        assert_eq!(v.len(), 4);
        v.clear();
        p.chord_off(&mut |o| v.push(o));
        assert_eq!(v.len(), 4);
        assert!(v.iter().all(|o| matches!(o, Out::Off(_))));
    }

    #[test]
    fn normal_sin_retrigger_solo_cambia_lo_necesario() {
        let mut p = Performer::new(SR);
        p.chord_on(&[60, 64, 67], None, 1.0, true, &mut |_| {});
        let mut v = Vec::new();
        p.chord_on(&[60, 63, 67], None, 1.0, false, &mut |o| v.push(o));
        assert_eq!(v.len(), 2);
        assert!(v.contains(&Out::Off(64)) && v.contains(&Out::On(63, 1.0)));
    }

    #[test]
    fn rasgueo_espaciado() {
        let mut p = Performer::new(SR);
        p.set_style(Style::Strum { speed: 1 }, &mut |_| {});
        let mut first = Vec::new();
        p.chord_on(&[60, 64, 67], Some(48), 1.0, true, &mut |o| first.push(o));
        assert_eq!(first, [Out::On(48, 1.0)]); // el bajo, enseguida
        let v = ons(&collect(&mut p, 0.5));
        assert_eq!(v.len(), 3);
        // 80 ms = 3840 muestras entre notas (con resolución de bloque de 32).
        assert!((v[1].0 - v[0].0) as i64 - 3840 < 64);
        assert_eq!(v.iter().map(|x| x.1).collect::<Vec<_>>(), [60, 64, 67]);
    }

    #[test]
    fn arpegio_a_tempo() {
        let mut p = Performer::new(SR);
        p.set_bpm(120.0);
        p.set_style(
            Style::Arp {
                pattern: 0,
                rate: 1,
            },
            &mut |_| {},
        );
        p.chord_on(&[60, 64, 67], None, 1.0, true, &mut |_| {});
        let v = collect(&mut p, 1.0);
        let on = ons(&v);
        // 120 BPM en corcheas: 4 notas por segundo.
        assert_eq!(on.len(), 4);
        assert_eq!(on.iter().map(|x| x.1).collect::<Vec<_>>(), [60, 64, 67, 72]);
        // Cada nota se apaga antes de la siguiente.
        let offs = v.iter().filter(|(_, o)| matches!(o, Out::Off(_))).count();
        assert!(offs >= 3);
    }

    #[test]
    fn arpegio_al_azar_es_determinista() {
        let run = || {
            let mut p = Performer::new(SR);
            p.set_style(
                Style::Arp {
                    pattern: ARP_RANDOM,
                    rate: 3,
                },
                &mut |_| {},
            );
            p.chord_on(&[60, 64, 67], None, 1.0, true, &mut |_| {});
            ons(&collect(&mut p, 1.0))
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn repetir() {
        let mut p = Performer::new(SR);
        p.set_bpm(60.0);
        p.set_style(Style::Repeat { rate: 0 }, &mut |_| {});
        p.chord_on(&[60, 64, 67], None, 1.0, true, &mut |_| {});
        // 60 BPM en negras: 1 por segundo, 3 notas cada vez.
        assert_eq!(ons(&collect(&mut p, 2.5)).len(), 9);
    }

    #[test]
    fn zumbido_sigue_al_soltar() {
        let mut p = Performer::new(SR);
        p.set_style(Style::Drone, &mut |_| {});
        p.chord_on(&[60, 64, 67], None, 1.0, true, &mut |_| {});
        let mut v = Vec::new();
        p.chord_off(&mut |o| v.push(o));
        assert!(v.is_empty());
        p.stop(&mut |o| v.push(o));
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn cambiar_de_modo_apaga_lo_que_sonaba() {
        let mut p = Performer::new(SR);
        p.set_style(Style::Drone, &mut |_| {});
        p.chord_on(&[60, 64, 67], None, 1.0, true, &mut |_| {});
        let mut v = Vec::new();
        p.set_style(Style::Normal, &mut |o| v.push(o));
        assert_eq!(v.len(), 3);
    }
}
