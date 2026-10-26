use realfft::RealFftPlanner;
use realfft::num_complex::Complex;

/// Detecta o BPM de um buffer de áudio.
/// `samples`: Buffer de áudio mono.
/// `sample_rate`: Taxa de amostragem (ex: 44100.0).
pub fn detect_bpm(samples: &[f32], sample_rate: f32) -> f32 {
    if samples.len() < (sample_rate as usize) {
        return 0.0; // Áudio muito curto para análise confiável
    }

    // 1. Calcula a função de energia (onset detection function) simplificada
    let frame_size = 1024;
    let hop_size = 512;
    let mut energy_diffs = Vec::new();

    for i in (0..samples.len() - frame_size).step_by(hop_size) {
        let current_energy: f32 = samples[i..i + frame_size].iter().map(|s| s * s).sum();
        let prev_energy: f32 = if i >= hop_size {
            samples[i - hop_size..i - hop_size + frame_size].iter().map(|s| s * s).sum()
        } else {
            0.0
        };
        // Half-wave rectification: só nos interessa se a energia aumentou (batida)
        let diff = (current_energy - prev_energy).max(0.0);
        energy_diffs.push(diff);
    }

    let n = energy_diffs.len();
    if n == 0 {
        return 0.0;
    }

    // 2. FFT para calcular a autocorrelação via Teorema de Wiener-Khinchin
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);
    
    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut energy_diffs, &mut spectrum).unwrap();

    // Multiplica pelo conjugado complexo para obter o espectro de potência
    let mut power_spectrum: Vec<Complex<f32>> = spectrum.iter().map(|c| {
        Complex::new(c.norm_sqr(), 0.0)
    }).collect();

    // 3. IFFT para voltar ao domínio do tempo (autocorrelação)
    let c2r = planner.plan_fft_inverse(n);
    let mut autocorr = c2r.make_output_vec();
    c2r.process(&mut power_spectrum, &mut autocorr).unwrap();

    // 4. Achar o pico da autocorrelação dentro da faixa de BPM humano (60 a 200 BPM)
    // 60 BPM -> ~86 frames de lag (44100 / 512)
    // 200 BPM -> ~26 frames de lag
    let min_lag_frames = 20;
    let max_lag_frames = 100;

    let mut best_lag = 0;
    let mut max_corr = 0.0f32;

    for lag in min_lag_frames..max_lag_frames.min(n) {
        let corr = autocorr[lag];
        if corr > max_corr {
            max_corr = corr;
            best_lag = lag;
        }
    }

    if best_lag == 0 {
        return 0.0;
    }

    // Converte lag de volta para BPM
    let seconds_per_frame = hop_size as f32 / sample_rate;
    let seconds_per_beat = best_lag as f32 * seconds_per_frame;
    let bpm = 60.0 / seconds_per_beat;

    bpm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bpm_120() {
        // Gera um "kick" simulado a cada 0.5s (120 BPM)
        let sample_rate = 44100.0;
        let duration_secs = 5.0;
        let total_samples = (sample_rate * duration_secs) as usize;
        let mut samples = vec![0.0f32; total_samples];
        
        let beat_interval = (sample_rate * 0.5) as usize; // 120 BPM
        
        for i in (0..total_samples).step_by(beat_interval) {
            if i + 100 < total_samples {
                // Simula um kick (onda senoidal de baixa frequência com decay)
                for j in 0..100 {
                    samples[i + j] = (j as f32 * 0.5).sin() * (1.0 - j as f32 / 100.0);
                }
            }
        }

        let bpm = detect_bpm(&samples, sample_rate);
        
        // Deve detectar algo próximo de 120 (margem de erro de ±10 BPM é aceitável para algoritmos simplificados)
        assert!((bpm - 120.0).abs() < 10.0, "BPM detectado: {}. Esperado ~120", bpm);
    }

    #[test]
    fn test_detect_bpm_silence() {
        let samples = vec![0.0f32; 44100 * 5];
        let bpm = detect_bpm(&samples, 44100.0);
        assert_eq!(bpm, 0.0);
    }
}