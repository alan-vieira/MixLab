use realfft::RealFftPlanner;

/// Estrutura que contém as 3 camadas de frequência da waveform
#[derive(Clone)]
pub struct WaveformLayers {
    pub low: Vec<f32>,  // Graves (0-300Hz)
    pub mid: Vec<f32>,  // Médios (300-4000Hz)
    pub high: Vec<f32>, // Agudos (4000Hz+)
}

/// Gera as 3 camadas de waveform usando análise espectral (FFT)
pub fn generate_waveform_layers(samples: &[f32], sample_rate: f32, num_peaks: usize) -> WaveformLayers {
    if samples.is_empty() || num_peaks == 0 {
        return WaveformLayers {
            low: vec![],
            mid: vec![],
            high: vec![],
        };
    }

    let samples_per_peak = samples.len() / num_peaks;
    if samples_per_peak == 0 {
        return WaveformLayers {
            low: vec![0.0; num_peaks],
            mid: vec![0.0; num_peaks],
            high: vec![0.0; num_peaks],
        };
    }

    let mut low = Vec::with_capacity(num_peaks);
    let mut mid = Vec::with_capacity(num_peaks);
    let mut high = Vec::with_capacity(num_peaks);

    let mut planner = RealFftPlanner::<f32>::new();
    let fft_size = samples_per_peak.next_power_of_two();
    let r2c = planner.plan_fft_forward(fft_size);

    for i in 0..num_peaks {
        let start = i * samples_per_peak;
        let end = start + samples_per_peak;
        let block = &samples[start..end];

        // CORREÇÃO: Input deve ser Vec<f32>, não Complex
        let mut fft_input = vec![0.0f32; fft_size];
        for (j, &s) in block.iter().enumerate() {
            if j < fft_size {
                fft_input[j] = s;
            }
        }

        let mut spectrum = r2c.make_output_vec();
        r2c.process(&mut fft_input, &mut spectrum).unwrap();

        // Calcula energia em cada faixa de frequência
        let bin_hz = sample_rate / fft_size as f32;
        let mut low_energy = 0.0f32;
        let mut mid_energy = 0.0f32;
        let mut high_energy = 0.0f32;

        for (bin, c) in spectrum.iter().enumerate() {
            let freq = bin as f32 * bin_hz;
            let magnitude = c.norm();
            let energy = magnitude * magnitude;

            if freq < 300.0 {
                low_energy += energy;
            } else if freq < 4000.0 {
                mid_energy += energy;
            } else {
                high_energy += energy;
            }
        }

        low.push(low_energy.sqrt());
        mid.push(mid_energy.sqrt());
        high.push(high_energy.sqrt());
    }

    // Normaliza cada camada separadamente
    normalize_layer(&mut low);
    normalize_layer(&mut mid);
    normalize_layer(&mut high);

    WaveformLayers { low, mid, high }
}

fn normalize_layer(layer: &mut [f32]) {
    if layer.is_empty() {
        return;
    }
    let mut sorted = layer.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_95 = sorted[(sorted.len() as f32 * 0.95) as usize].max(0.001);

    layer.iter_mut().for_each(|p| {
        let normalized = (*p / percentile_95).min(1.0);
        *p = normalized.powf(0.5); // Curva gamma para suavizar
    });
}

/// Versão simples (RMS geral) para a mini waveform
pub fn generate_waveform_peaks(samples: &[f32], num_peaks: usize) -> Vec<f32> {
    if samples.is_empty() || num_peaks == 0 {
        return vec![];
    }

    let samples_per_peak = samples.len() / num_peaks;
    if samples_per_peak == 0 {
        return samples.to_vec();
    }

    let mut peaks = Vec::with_capacity(num_peaks);

    for i in 0..num_peaks {
        let start = i * samples_per_peak;
        let end = start + samples_per_peak;
        let block = &samples[start..end];
        
        let sum_squares: f32 = block.iter().map(|s| s * s).sum();
        let rms = (sum_squares / block.len() as f32).sqrt();
        peaks.push(rms);
    }

    let mut sorted = peaks.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_95 = sorted[(sorted.len() as f32 * 0.95) as usize];
    
    let gamma = 0.5f32;
    peaks.iter_mut().for_each(|p| {
        let normalized = (*p / percentile_95).min(1.0);
        *p = normalized.powf(gamma);
    });

    peaks
}