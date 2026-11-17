//! Voicing (qué notas exactas suenan), inversiones y voice leading.

use crate::chord::Chord;

/// Opciones de voicing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoicingOptions {
    /// Añadir el bajo (raíz una octava por debajo).
    pub bass: bool,
    /// 0 = fundamental, 1 = primera inversión, 2 = segunda.
    pub inversion: u8,
}

impl Default for VoicingOptions {
    fn default() -> Self {
        Self {
            bass: true,
            inversion: 0,
        }
    }
}

/// Notas que suenan: la parte de arriba (tríada + extensiones) y el bajo aparte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voicing {
    pub upper: Vec<i32>,
    pub bass: Option<i32>,
}

impl Voicing {
    /// Construye el voicing de un acorde.
    pub fn build(chord: Chord, opts: VoicingOptions) -> Voicing {
        let mut upper = upper_structure(chord);
        invert(&mut upper, opts.inversion);
        Voicing {
            upper,
            bass: opts.bass.then_some(chord.root - 12),
        }
    }

    /// Todas las notas, el bajo primero.
    pub fn notes(&self) -> Vec<i32> {
        self.bass.iter().copied().chain(self.upper.iter().copied()).collect()
    }
}

/// Raíz, tercera, quinta y hasta 2 extensiones, ordenadas.
fn upper_structure(chord: Chord) -> Vec<i32> {
    let k = chord.kind;
    let mut v = vec![chord.root];
    if let Some(t) = k.third() {
        v.push(chord.root + t);
    }
    v.push(chord.root + k.fifth());
    v.extend(k.extensions().iter().map(|e| chord.root + e));
    v.sort_unstable();
    v.dedup();
    v
}

/// Sube a la octava de arriba las `n` notas más graves.
fn invert(notes: &mut [i32], n: u8) {
    for _ in 0..n.min(notes.len().saturating_sub(1) as u8) {
        notes[0] += 12;
        notes.sort_unstable();
    }
}

/// Elige la inversión y la octava que menos mueven las notas respecto a `prev`.
///
/// Prueba 3 inversiones × 3 octavas. El coste es la suma de saltos entre notas emparejadas
/// (de grave a agudo, repitiendo la más aguda si una lista es más corta), más 6 si la nota
/// más grave salta más de 7 semitonos. En los empates gana la más cercana a la fundamental.
pub fn voice_lead(prev: &[i32], chord: Chord, opts: VoicingOptions) -> Voicing {
    let base = Voicing::build(chord, VoicingOptions { inversion: 0, ..opts });
    if prev.is_empty() {
        return Voicing::build(chord, opts);
    }
    let mut prev_sorted = prev.to_vec();
    prev_sorted.sort_unstable();

    let mut best: Option<(i32, i32, Vec<i32>)> = None;
    for inv in 0..3u8 {
        for oct in [0, -12, 12] {
            let mut cand = base.upper.clone();
            invert(&mut cand, inv);
            cand.iter_mut().for_each(|n| *n += oct);
            let cost = movement_cost(&prev_sorted, &cand);
            // Preferencia en empates: fundamental y sin cambiar de octava.
            let pref = inv as i32 + (oct != 0) as i32;
            let better = match &best {
                None => true,
                Some((c, p, _)) => cost < *c || (cost == *c && pref < *p),
            };
            if better {
                best = Some((cost, pref, cand));
            }
        }
    }
    Voicing {
        upper: best.map(|b| b.2).unwrap_or(base.upper),
        bass: base.bass,
    }
}

fn movement_cost(prev: &[i32], cand: &[i32]) -> i32 {
    let n = prev.len().max(cand.len());
    let at = |v: &[i32], i: usize| v[i.min(v.len() - 1)];
    let mut cost: i32 = (0..n).map(|i| (at(cand, i) - at(prev, i)).abs()).sum();
    if (cand[0] - prev[0]).abs() > 7 {
        cost += 6;
    }
    cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord::ChordType::*;

    #[test]
    fn triada_con_bajo() {
        let v = Voicing::build(Chord::new(48, Major), VoicingOptions::default());
        assert_eq!(v.upper, [48, 52, 55]);
        assert_eq!(v.bass, Some(36));
        assert_eq!(v.notes(), [36, 48, 52, 55]);
    }

    #[test]
    fn inversiones() {
        let c = Chord::new(48, Major);
        let v = Voicing::build(c, VoicingOptions { bass: false, inversion: 1 });
        assert_eq!(v.upper, [52, 55, 60]);
        let v = Voicing::build(c, VoicingOptions { bass: false, inversion: 2 });
        assert_eq!(v.upper, [55, 60, 64]);
    }

    #[test]
    fn novena_tiene_cinco_notas() {
        let v = Voicing::build(Chord::new(48, Maj9), VoicingOptions::default());
        assert_eq!(v.upper, [48, 52, 55, 59, 62]);
    }

    #[test]
    fn voice_leading_de_do_a_fa() {
        let opts = VoicingOptions { bass: false, inversion: 0 };
        let prev = [48, 52, 55]; // Do
        let v = voice_lead(&prev, Chord::new(53, Major), opts);
        // Fa en segunda inversión (Do-Fa-La) es lo que menos se mueve.
        assert_eq!(v.upper, [48, 53, 57]);
    }

    #[test]
    fn voice_leading_sin_acorde_previo() {
        let opts = VoicingOptions::default();
        let c = Chord::new(55, Major);
        assert_eq!(voice_lead(&[], c, opts), Voicing::build(c, opts));
    }

    #[test]
    fn voice_leading_no_mueve_el_bajo() {
        let opts = VoicingOptions::default();
        let v = voice_lead(&[60, 64, 67], Chord::new(53, Major), opts);
        assert_eq!(v.bass, Some(41));
    }
}
