use std::f32::consts::PI;

/// Filtro combinado Low-Pass / High-Pass (estilo CDJ)
/// position: 0.0 = LPF máximo, 0.5 = neutro, 1.0 = HPF máximo
pub struct Filter {
    position: f32,
    sample_rate: f32,
    lp_state: [f32; 2],
    hp_state: [f32; 2],
    lp_b0: f32,
    lp_b1: f32,
    lp_b2: f32,
    lp_a1: f32,
    lp_a2: f32,
    hp_b0: f32,
    hp_b1: f32,
    hp_b2: f32,
    hp_a1: f32,
    hp_a2: f32,
}

impl Filter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            position: 0.5,
            sample_rate,
            lp_state: [0.0; 2],
            hp_state: [0.0; 2],
            lp_b0: 1.0,
            lp_b1: 0.0,
            lp_b2: 0.0,
            lp_a1: 0.0,
            lp_a2: 0.0,
            hp_b0: 1.0,
            hp_b1: 0.0,
            hp_b2: 0.0,
            hp_a1: 0.0,
            hp_a2: 0.0,
        };
        filter.update_coefficients(0.5);
        filter
    }

    pub fn set_position(&mut self, position: f32) {
        self.position = position.clamp(0.0, 1.0);
        self.update_coefficients(self.position);
    }

    pub fn get_position(&self) -> f32 {
        self.position
    }

    pub fn reset(&mut self) {
        self.lp_state = [0.0; 2];
        self.hp_state = [0.0; 2];
    }

    fn update_coefficients(&mut self, position: f32) {
        // 👈 CORREÇÃO: Adicionado _f32 aos literais para evitar ambiguidade de tipo
        let cutoff = if position < 0.5 {
            let t = position / 0.5_f32;
            200.0_f32 * (20000.0_f32 / 200.0_f32).powf(t)
        } else {
            let t = (position - 0.5_f32) / 0.5_f32;
            20000.0_f32 * (200.0_f32 / 20000.0_f32).powf(t)
        };

        let wc = 2.0_f32 * PI * cutoff;
        let t = 1.0_f32 / self.sample_rate;
        let wc_t = wc * t;
        let wc_t_sq = wc_t * wc_t;
        let sqrt2_wc_t = std::f32::consts::SQRT_2 * wc_t;
        let denom = 1.0_f32 + sqrt2_wc_t + wc_t_sq;

        self.lp_b0 = wc_t_sq / denom;
        self.lp_b1 = 2.0_f32 * self.lp_b0;
        self.lp_b2 = self.lp_b0;
        self.lp_a1 = (2.0_f32 * wc_t_sq - 2.0_f32) / denom;
        self.lp_a2 = (1.0_f32 - sqrt2_wc_t + wc_t_sq) / denom;

        self.hp_b0 = 1.0_f32 / denom;
        self.hp_b1 = -2.0_f32 * self.hp_b0;
        self.hp_b2 = self.hp_b0;
        self.hp_a1 = self.lp_a1;
        self.hp_a2 = self.lp_a2;
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        if (self.position - 0.5_f32).abs() < 0.01_f32 {
            return;
        }

        for sample in buffer.iter_mut() {
            if self.position < 0.5 {
                let input = *sample;
                let output = self.lp_b0 * input
                    + self.lp_b1 * self.lp_state[0]
                    + self.lp_b2 * self.lp_state[1]
                    - self.lp_a1 * self.lp_state[0]
                    - self.lp_a2 * self.lp_state[1];

                self.lp_state[1] = self.lp_state[0];
                self.lp_state[0] = input;
                *sample = output;
            } else {
                let input = *sample;
                let output = self.hp_b0 * input
                    + self.hp_b1 * self.hp_state[0]
                    + self.hp_b2 * self.hp_state[1]
                    - self.hp_a1 * self.hp_state[0]
                    - self.hp_a2 * self.hp_state[1];

                self.hp_state[1] = self.hp_state[0];
                self.hp_state[0] = input;
                *sample = output;
            }
        }
    }
}
