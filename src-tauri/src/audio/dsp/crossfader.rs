use std::f32::consts::PI;

pub struct Crossfader {
    position: f32, // 0.0 (totalmente A) a 1.0 (totalmente B)
}

impl Crossfader {
    pub fn new() -> Self {
        Self { position: 0.5 } // Começa no centro
    }

    pub fn set_position(&mut self, position: f32) {
            self.position = position.clamp(0.0, 1.0);
        }

        pub fn position(&self) -> f32 {
            self.position
        }

        /// Calcula os ganhos para o Deck A e Deck B usando a curva "Equal Power"
    /// Isso garante que a soma das potências (volume percebido) permaneça constante
    pub fn calculate_gains(&self) -> (f32, f32) {
        // Fórmula Equal Power: cos e sin de um ângulo de 0 a PI/2
        let angle = self.position * (PI / 2.0);
        let gain_a = angle.cos();
        let gain_b = angle.sin();
        
        (gain_a, gain_b)
    }

    /// Mistura dois buffers de áudio aplicando a curva do crossfader
    pub fn mix(&self, buffer_a: &[f32], buffer_b: &[f32], output: &mut [f32]) {
        let (gain_a, gain_b) = self.calculate_gains();
        
        // Garante que os buffers tenham o mesmo tamanho
        let len = output.len().min(buffer_a.len()).min(buffer_b.len());
        
        for i in 0..len {
            let mixed = (buffer_a[i] * gain_a) + (buffer_b[i] * gain_b);
            output[i] = mixed.clamp(-1.0, 1.0); // Previne clipping na soma
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfader_center_gives_equal_power() {
        let crossfader = Crossfader::new();
        // No centro (0.5), os ganhos devem ser iguais e somar ~1.0 em potência
        let (gain_a, gain_b) = crossfader.calculate_gains();
        
        // cos(PI/4) = sin(PI/4) ≈ 0.707
        assert!((gain_a - gain_b).abs() < 0.01, "Ganhos no centro devem ser iguais");
        assert!((gain_a - 0.707).abs() < 0.01, "Ganho no centro deve ser ~0.707 (-3dB)");
    }

    #[test]
    fn test_crossfader_left_is_full_deck_a() {
        let mut crossfader = Crossfader::new();
        crossfader.set_position(0.0); // Totalmente à esquerda
        
        let (gain_a, gain_b) = crossfader.calculate_gains();
        
        assert!((gain_a - 1.0).abs() < 0.01, "Deck A deve estar em 100%");
        assert!(gain_b.abs() < 0.01, "Deck B deve estar em 0% (mute)");
    }

    #[test]
    fn test_crossfader_right_is_full_deck_b() {
        let mut crossfader = Crossfader::new();
        crossfader.set_position(1.0); // Totalmente à direita
        
        let (gain_a, gain_b) = crossfader.calculate_gains();
        
        assert!(gain_a.abs() < 0.01, "Deck A deve estar em 0% (mute)");
        assert!((gain_b - 1.0).abs() < 0.01, "Deck B deve estar em 100%");
    }

    #[test]
    fn test_crossfader_mixes_buffers_without_clipping() {
        let mut crossfader = Crossfader::new();
        crossfader.set_position(0.5); // Centro
        
        let buffer_a = vec![1.0, 1.0, 1.0]; // Sinal máximo
        let buffer_b = vec![1.0, 1.0, 1.0]; // Sinal máximo
        let mut output = vec![0.0; 3];
        
        crossfader.mix(&buffer_a, &buffer_b, &mut output);
        
        // 1.0 * 0.707 + 1.0 * 0.707 = 1.414, mas o clamp deve limitar a 1.0
        assert!(output.iter().all(|&x| x <= 1.0 && x >= -1.0), "Output não pode exceder 1.0 (clipping)");
        assert!((output[0] - 1.0).abs() < 0.01, "Output deve ser limitado a 1.0");
    }
}