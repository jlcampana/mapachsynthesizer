//! Una voz del sintetizador analógico virtual: 2 osciladores, filtro y ADSR.

use mapach_dsp::{Adsr, AdsrParams, Oscillator, Svf, pan_gains};

use crate::Patch;
use crate::midi_to_freq;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Voice {
    pub note: u8,
    pub started: u64,
    velocity: f32,
    pan_l: f32,
    pan_r: f32,
    osc1: Oscillator,
    osc2: Oscillator,
    filter: Svf,
    env: Adsr,
    /// La tecla sigue pulsada.
    held: bool,
}

impl Voice {
    pub fn start(&mut self, note: u8, velocity: f32, pan: f32, clock: u64, patch: &Patch) {
        let restart = !self.env.is_active() || self.note != note;
        self.note = note;
        self.started = clock;
        self.velocity = velocity;
        let (l, r) = pan_gains(pan * patch.stereo);
        self.pan_l = l;
        self.pan_r = r;
        if restart {
            // Fases distintas por nota para que los acordes no empiecen todos con el mismo golpe,
            // pero deterministas.
            let p = (note as f32 * 0.137).fract();
            self.osc1.reset(p);
            self.osc2.reset((p + 0.5).fract());
            self.filter.reset();
        }
        self.held = true;
        self.env.gate_on();
    }

    pub fn release(&mut self) {
        self.held = false;
        self.env.gate_off();
    }

    pub fn is_held(&self, note: u8) -> bool {
        self.held && self.note == note && self.env.is_active()
    }

    pub fn is_active(&self) -> bool {
        self.env.is_active()
    }

    pub fn is_released(&self) -> bool {
        self.env.is_released()
    }

    pub fn level(&self) -> f32 {
        self.env.level()
    }

    /// Suma la voz a un bloque estéreo intercalado.
    pub fn render(
        &mut self,
        out: &mut [f32],
        patch: &Patch,
        env: &AdsrParams,
        cutoff: f32,
        sample_rate: f32,
    ) {
        let f1 = midi_to_freq(self.note as f32 + patch.osc1_semitones);
        let f2 = midi_to_freq(self.note as f32 + patch.osc2_semitones + patch.detune_cents / 100.0);
        let dt1 = f1 / sample_rate;
        let dt2 = f2 / sample_rate;
        // El corte sigue un poco a la nota, para que las agudas no se apaguen.
        let key_track = 2f32.powf((self.note as f32 - 60.0) / 24.0);
        let gain = 0.22 * (0.4 + 0.6 * self.velocity);
        let m2 = patch.osc2_mix;
        let m1 = 1.0 - m2 * 0.5;
        for frame in out.chunks_exact_mut(2) {
            let e = self.env.next(env, sample_rate);
            let fc = cutoff * key_track * (1.0 + patch.env_to_filter * e);
            self.filter.set(fc, patch.resonance, sample_rate);
            let raw = self.osc1.next(patch.osc1, dt1) * m1 + self.osc2.next(patch.osc2, dt2) * m2;
            let s = self.filter.lowpass(raw) * e * gain;
            frame[0] += s * self.pan_l;
            frame[1] += s * self.pan_r;
            if !self.env.is_active() {
                break;
            }
        }
    }
}
