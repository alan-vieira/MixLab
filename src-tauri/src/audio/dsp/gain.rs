pub struct GainNode {
    gain: f32,
}

impl GainNode {
    pub fn new(gain: f32) -> Self {
        GainNode { gain }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    /// Processa o buffer de áudio aplicando o ganho
    pub fn process(&self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            let processed = *sample * self.gain;
            // Clamping: evita que o áudio "estale" (clipping) se passar de 1.0 ou -1.0
            *sample = processed.clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain_applies_volume_correctly() {
        let gain_node = GainNode::new(0.5); // 50% de volume (sem mut, pois process usa &self)
        let mut buffer = vec![1.0, -1.0, 0.8, -0.4];
        
        gain_node.process(&mut buffer);
        
        assert_eq!(buffer, vec![0.5, -0.5, 0.4, -0.2]);
    }

    #[test]
    fn test_gain_clamps_to_prevent_clipping() {
        let gain_node = GainNode::new(2.0); // 200% de volume (amplificação)
        let mut buffer = vec![0.8, -0.9];
        
        gain_node.process(&mut buffer);
        
        // 0.8 * 2.0 = 1.6, mas deve ser limitado a 1.0
        // -0.9 * 2.0 = -1.8, mas deve ser limitado a -1.0
        assert_eq!(buffer, vec![1.0, -1.0]);
    }

    #[test]
    fn test_gain_handles_silence() {
        let gain_node = GainNode::new(5.0); // Ganho alto
        let mut buffer = vec![0.0, -0.0]; // Silêncio
        
        gain_node.process(&mut buffer);
        
        assert_eq!(buffer, vec![0.0, 0.0]);
    }

    #[test]
    fn test_set_gain_changes_volume_dynamically() {
        let mut gain_node = GainNode::new(1.0); // Começa em 100%
        let mut buffer = vec![0.5, -0.5];
        
        gain_node.process(&mut buffer);
        assert_eq!(buffer, vec![0.5, -0.5]); // Sem alteração
        
        // Muda o ganho para 0.0 (Mute)
        gain_node.set_gain(0.0);
        gain_node.process(&mut buffer);
        assert_eq!(buffer, vec![0.0, 0.0]); // Agora está em mute
    }
}