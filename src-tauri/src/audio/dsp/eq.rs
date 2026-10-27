use std::f32::consts::PI;

/// Filtro Biquad padrão da indústria (Audio EQ Cookbook)
#[derive(Clone)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub fn new(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

pub struct Equalizer {
    sample_rate: f32,
    low_filter: Biquad,
    mid_filter: Biquad,
    high_filter: Biquad,
    low_gain_db: f32,
    mid_gain_db: f32,
    high_gain_db: f32,
}

// Fórmula Low-Shelf (Graves)
fn make_low_shelf(freq: f32, sample_rate: f32, q: f32, db: f32) -> Biquad {
    let w0 = 2.0 * PI * freq / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let a_lin = 10.0f32.powf(db / 20.0);
    let a_sqrt = a_lin.sqrt();
    let two_sqrt_a = 2.0 * a_sqrt;
    let alpha = sin_w0 / (2.0 * q);

    let b0 = a_lin * ((a_lin + 1.0) - (a_lin - 1.0) * cos_w0 + two_sqrt_a * alpha);
    let b1 = 2.0 * a_lin * ((a_lin - 1.0) - (a_lin + 1.0) * cos_w0);
    let b2 = a_lin * ((a_lin + 1.0) - (a_lin - 1.0) * cos_w0 - two_sqrt_a * alpha);
    let a0 = (a_lin + 1.0) + (a_lin - 1.0) * cos_w0 + two_sqrt_a * alpha;
    let a1 = -2.0 * ((a_lin - 1.0) + (a_lin + 1.0) * cos_w0);
    let a2 = (a_lin + 1.0) + (a_lin - 1.0) * cos_w0 - two_sqrt_a * alpha;

    let inv_a0 = 1.0 / a0;
    Biquad::new(
        b0 * inv_a0,
        b1 * inv_a0,
        b2 * inv_a0,
        a1 * inv_a0,
        a2 * inv_a0,
    )
}

// Fórmula Peaking (Médios)
fn make_peaking(freq: f32, sample_rate: f32, q: f32, db: f32) -> Biquad {
    let w0 = 2.0 * PI * freq / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let a_lin = 10.0f32.powf(db / 20.0);
    let alpha = sin_w0 / (2.0 * q);

    let b0 = 1.0 + alpha * a_lin;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a_lin;
    let a0 = 1.0 + alpha / a_lin;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a_lin;

    let inv_a0 = 1.0 / a0;
    Biquad::new(
        b0 * inv_a0,
        b1 * inv_a0,
        b2 * inv_a0,
        a1 * inv_a0,
        a2 * inv_a0,
    )
}

// Fórmula High-Shelf (Agudos)
fn make_high_shelf(freq: f32, sample_rate: f32, q: f32, db: f32) -> Biquad {
    let w0 = 2.0 * PI * freq / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let a_lin = 10.0f32.powf(db / 20.0);
    let a_sqrt = a_lin.sqrt();
    let two_sqrt_a = 2.0 * a_sqrt;
    let alpha = sin_w0 / (2.0 * q);

    let b0 = a_lin * ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 + two_sqrt_a * alpha);
    let b1 = -2.0 * a_lin * ((a_lin - 1.0) + (a_lin + 1.0) * cos_w0);
    let b2 = a_lin * ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 - two_sqrt_a * alpha);
    let a0 = (a_lin + 1.0) - (a_lin - 1.0) * cos_w0 + two_sqrt_a * alpha;
    let a1 = 2.0 * ((a_lin - 1.0) - (a_lin + 1.0) * cos_w0);
    let a2 = (a_lin + 1.0) - (a_lin - 1.0) * cos_w0 - two_sqrt_a * alpha;

    let inv_a0 = 1.0 / a0;
    Biquad::new(
        b0 * inv_a0,
        b1 * inv_a0,
        b2 * inv_a0,
        a1 * inv_a0,
        a2 * inv_a0,
    )
}

impl Equalizer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            low_filter: make_low_shelf(320.0, sample_rate, 1.0, 0.0),
            mid_filter: make_peaking(1000.0, sample_rate, 1.0, 0.0),
            high_filter: make_high_shelf(3200.0, sample_rate, 1.0, 0.0),
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
        }
    }

    pub fn set_low_gain(&mut self, db: f32) {
        self.low_filter = make_low_shelf(320.0, self.sample_rate, 1.0, db);
        self.low_gain_db = db;
    }

    pub fn set_mid_gain(&mut self, db: f32) {
        self.mid_filter = make_peaking(1000.0, self.sample_rate, 1.0, db);
        self.mid_gain_db = db;
    }

    pub fn set_high_gain(&mut self, db: f32) {
        self.high_filter = make_high_shelf(3200.0, self.sample_rate, 1.0, db);
        self.high_gain_db = db;
    }

    pub fn low_gain(&self) -> f32 {
        10.0f32.powf(self.low_gain_db / 20.0)
    }

    pub fn mid_gain(&self) -> f32 {
        10.0f32.powf(self.mid_gain_db / 20.0)
    }

    pub fn high_gain(&self) -> f32 {
        10.0f32.powf(self.high_gain_db / 20.0)
    }

    pub fn low_gain_db(&self) -> f32 {
        self.low_gain_db
    }

    pub fn mid_gain_db(&self) -> f32 {
        self.mid_gain_db
    }

    pub fn high_gain_db(&self) -> f32 {
        self.high_gain_db
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            let s1 = self.low_filter.process(*sample);
            let s2 = self.mid_filter.process(s1);
            *sample = self.high_filter.process(s2).clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_sine_wave(freq: f32, sample_rate: f32, duration_secs: f32) -> Vec<f32> {
        let num_samples = (sample_rate * duration_secs) as usize;
        (0..num_samples)
            .map(|i| (i as f32 * 2.0 * PI * freq / sample_rate).sin())
            .collect()
    }

    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    #[test]
    fn test_eq_low_shelf_cuts_bass_frequencies() {
        let mut eq = Equalizer::new(44100.0);
        eq.set_low_gain(-24.0);

        let mut buffer = generate_sine_wave(60.0, 44100.0, 1.0);
        let original_rms = calculate_rms(&buffer);

        eq.process(&mut buffer);
        let filtered_rms = calculate_rms(&buffer);

        assert!(
            filtered_rms < original_rms * 0.1,
            "O corte de graves falhou. Original: {:.4}, Filtrado: {:.4}",
            original_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_eq_high_shelf_cuts_treble_frequencies() {
        let mut eq = Equalizer::new(44100.0);
        eq.set_high_gain(-24.0);

        let mut buffer = generate_sine_wave(10000.0, 44100.0, 1.0);
        let original_rms = calculate_rms(&buffer);

        eq.process(&mut buffer);
        let filtered_rms = calculate_rms(&buffer);

        assert!(
            filtered_rms < original_rms * 0.1,
            "O corte de agudos falhou. Original: {:.4}, Filtrado: {:.4}",
            original_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_eq_neutral_does_not_alter_signal() {
        let mut eq = Equalizer::new(44100.0);

        let mut buffer = generate_sine_wave(1000.0, 44100.0, 1.0);
        let original_rms = calculate_rms(&buffer);

        eq.process(&mut buffer);
        let filtered_rms = calculate_rms(&buffer);

        assert!(
            (filtered_rms - original_rms).abs() < 0.01,
            "EQ neutro alterou o sinal. Original: {:.4}, Filtrado: {:.4}",
            original_rms,
            filtered_rms
        );
    }
}
