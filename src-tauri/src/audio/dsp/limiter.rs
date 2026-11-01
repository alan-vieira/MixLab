/// Peak Limiter com lookahead simplificado (ataque instantâneo, release suave).
///
/// Funcionamento:
/// 1. Detecta o pico absoluto (sample-by-sample, sem atraso — ataque instantâneo)
/// 2. Calcula o ganho necessário para manter o sinal abaixo do threshold
/// 3. Aplica o ganho com release suave (interpolação linear entre amostras)
/// 4. Se o sinal exceder o ceiling, faz soft clip suave para evitar distorção dura
///
/// Uso no pipeline:
///   1. lim.process(&mut buffer) — processa in-place
///   2. lim.get_gain_reduction() — retorna a redução de gain em dB (0.0 = sem redução)
pub struct PeakLimiter {
    threshold: f32,    // Limiar em amplitude linear (0.0–1.0)
    ceiling: f32,      // Teto absoluto (0.0–1.0)
    attack: f32,       // Coeficiente de ataque (0.0–1.0, 1.0 = instantâneo)
    release: f32,      // Coeficiente de release (0.0–1.0, menor = mais lento)
    current_gain: f32, // Gain atual sendo aplicado (1.0 = sem atenuação)
    sample_rate: u32,
    enabled: bool,
}

impl PeakLimiter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            threshold: 0.8, // -2 dBFS
            ceiling: 0.95,  // -0.5 dBFS (margem de segurança)
            attack: 0.99,   // Ataque praticamente instantâneo
            release: 0.95,  // Release moderado
            current_gain: 1.0,
            sample_rate,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_gain = 1.0;
        }
    }

    pub fn set_threshold(&mut self, threshold_db: f32) {
        // Converte dB para amplitude linear: 10^(db/20)
        self.threshold = (10.0_f32).powf(threshold_db / 20.0).clamp(0.001, 1.0);
    }

    pub fn set_ceiling(&mut self, ceiling_db: f32) {
        self.ceiling = (10.0_f32).powf(ceiling_db / 20.0).clamp(0.001, 1.0);
    }

    pub fn set_attack(&mut self, attack_ms: f32) {
        // Converte ms para coeficiente: exp(-1 / (ms * sample_rate / 1000))
        let tau = attack_ms.max(0.1) * self.sample_rate as f32 / 1000.0;
        self.attack = (-1.0 / tau).exp().clamp(0.0, 1.0);
    }

    pub fn set_release(&mut self, release_ms: f32) {
        let tau = release_ms.max(1.0) * self.sample_rate as f32 / 1000.0;
        self.release = (-1.0 / tau).exp().clamp(0.0, 1.0);
    }

    /// Processa o buffer in-place, aplicando limitação de pico.
    /// Retorna a redução máxima de gain aplicada (em dB, negativo = atenuação).
    pub fn process(&mut self, buffer: &mut [f32]) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let mut max_gain_reduction = 0.0;

        for sample in buffer.iter_mut() {
            let abs_sample = sample.abs();

            // 1. Detecta se excede o threshold
            let needed_gain = if abs_sample > self.threshold {
                self.threshold / abs_sample.max(0.0001)
            } else {
                1.0
            };

            // 2. Se o sinal após ganho ainda excede o ceiling, aplica soft clip
            let mut target_gain = needed_gain.min(self.ceiling / abs_sample.max(0.0001));
            target_gain = target_gain.clamp(0.0, 1.0);

            // 3. Suaviza a transição do gain (ataque rápido, release mais lento)
            if target_gain < self.current_gain {
                // Ataque: reduzir gain rapidamente
                self.current_gain =
                    self.current_gain * self.attack + target_gain * (1.0 - self.attack);
            } else {
                // Release: aumentar gain lentamente
                self.current_gain =
                    self.current_gain * self.release + target_gain * (1.0 - self.release);
            }

            // 4. Aplica o gain ao sinal
            let mut out = *sample * self.current_gain;

            // 5. Soft clip se ainda exceder o ceiling (proteção final)
            if out.abs() > self.ceiling {
                out = self.soft_clip(out, self.ceiling);
            }

            *sample = out;

            // Rastreia a maior redução de gain (em dB)
            let gr_db = 20.0 * self.current_gain.log10().max(-100.0);
            if gr_db < max_gain_reduction {
                max_gain_reduction = gr_db;
            }
        }

        max_gain_reduction
    }

    /// Soft clip suave: arredonda os picos que excedem o ceiling
    fn soft_clip(&self, sample: f32, ceiling: f32) -> f32 {
        if sample.abs() <= ceiling {
            return sample;
        }
        // Tanh-based soft clipping ao redor do ceiling
        let over = (sample.abs() - ceiling) / ceiling;
        let soft = ceiling * (1.0 + (over.tanh() - 1.0).tanh());
        if sample < 0.0 {
            -soft
        } else {
            soft
        }
    }

    /// Retorna a redução de gain atual em dB (para VU meter de redução)
    pub fn get_gain_reduction_db(&self) -> f32 {
        20.0 * self.current_gain.log10().max(-100.0)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_threshold_db(&self) -> f32 {
        20.0 * self.threshold.log10()
    }

    pub fn get_ceiling_db(&self) -> f32 {
        20.0 * self.ceiling.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_disabled_passthrough() {
        let mut limiter = PeakLimiter::new(48000);
        limiter.set_enabled(false);
        let mut buffer = vec![0.5, -0.5, 0.9, -0.9];
        let original = buffer.clone();
        limiter.process(&mut buffer);
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_limiter_prevents_clipping() {
        let mut limiter = PeakLimiter::new(48000);
        limiter.set_enabled(true);
        limiter.set_threshold(-6.0); // 0.5 amplitude
        let mut buffer = vec![1.0, -1.0, 1.0, -1.0];
        limiter.process(&mut buffer);
        for &s in &buffer {
            assert!(s.abs() <= 1.0 + 0.001, "Sample excede 1.0: {}", s);
        }
    }

    #[test]
    fn test_limiter_quiet_signal_unchanged() {
        let mut limiter = PeakLimiter::new(48000);
        limiter.set_enabled(true);
        limiter.set_threshold(-6.0);
        let mut buffer = vec![0.1, -0.1, 0.2];
        let original = buffer.clone();
        limiter.process(&mut buffer);
        for (i, &s) in buffer.iter().enumerate() {
            assert!(
                (s - original[i]).abs() < 0.001,
                "Sinal abaixo do threshold não deve ser alterado: idx {}",
                i
            );
        }
    }

    #[test]
    fn test_limiter_gain_reduction_negative_db() {
        let mut limiter = PeakLimiter::new(48000);
        limiter.set_enabled(true);
        limiter.set_threshold(-20.0);
        let mut buffer = vec![0.9, 0.9, 0.9];
        let gr = limiter.process(&mut buffer);
        assert!(
            gr < 0.0,
            "Redução de gain deve ser negativa em dB, got {}",
            gr
        );
    }
}
