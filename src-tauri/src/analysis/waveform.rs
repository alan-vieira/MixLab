use realfft::RealFftPlanner;

/// Estrutura que contém as 3 camadas de frequência da waveform
#[derive(Clone)]
pub struct WaveformLayers {
    pub low: Vec<f32>,
    pub mid: Vec<f32>,
    pub high: Vec<f32>,
}

/// Estrutura para a waveform completa (min/max envelope)
#[derive(Clone)]
pub struct FullWaveform {
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

/// Gera as 3 camadas de waveform usando análise espectral (FFT)
pub fn generate_waveform_layers(samples: &[f32], sample_rate: f32, num_peaks: usize) -> WaveformLayers {
    if samples.is_empty() || num_peaks == 0 {
        return WaveformLayers { low: vec![], mid: vec![], high: vec![] };
    }

    let num_peaks = num_peaks.min(1000);
    let samples_per_peak = samples.len() / num_peaks;
    if samples_per_peak == 0 {
        return WaveformLayers { low: vec![0.0; num_peaks], mid: vec![0.0; num_peaks], high: vec![0.0; num_peaks] };
    }

    let mut low = Vec::with_capacity(num_peaks);
    let mut mid = Vec::with_capacity(num_peaks);
    let mut high = Vec::with_capacity(num_peaks);

    let fft_size = 2048usize;
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(fft_size);

    for i in 0..num_peaks {
        let start = i * samples_per_peak;
        let end = start + samples_per_peak;
        let block = &samples[start..end];

        let mut fft_input = vec![0.0f32; fft_size];
        let stride = (block.len() / fft_size).max(1);
        for (j, &s) in block.iter().step_by(stride).enumerate() {
            if j < fft_size { fft_input[j] = s; }
        }

        let mut spectrum = r2c.make_output_vec();
        r2c.process(&mut fft_input, &mut spectrum).unwrap();

        let bin_hz = sample_rate / fft_size as f32;
        let mut low_energy = 0.0f32;
        let mut mid_energy = 0.0f32;
        let mut high_energy = 0.0f32;

        for (bin, c) in spectrum.iter().enumerate() {
            let freq = bin as f32 * bin_hz;
            let energy = c.norm().powi(2);
            if freq < 300.0 { low_energy += energy; }
            else if freq < 4000.0 { mid_energy += energy; }
            else { high_energy += energy; }
        }

        low.push(low_energy.sqrt());
        mid.push(mid_energy.sqrt());
        high.push(high_energy.sqrt());
    }

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
    
    // ✅ CORREÇÃO: Percentil 99 em vez de 95
    // Ignora picos extremos isolados e usa o "pico real" sustentado como referência
    let percentile_99 = sorted[(sorted.len() as f32 * 0.99) as usize].max(0.001);
    
    layer.iter_mut().for_each(|p| {
        let normalized = (*p / percentile_99).min(1.0);
        // ✅ CORREÇÃO: Gamma 0.7 em vez de 0.5
        // Comprime menos os valores baixos, preservando a diferença visual
        *p = normalized.powf(0.7);
    });
}

pub fn generate_waveform_peaks(samples: &[f32], num_peaks: usize) -> Vec<f32> {
    if samples.is_empty() || num_peaks == 0 { return vec![]; }
    let samples_per_peak = samples.len() / num_peaks;
    if samples_per_peak == 0 { return samples.to_vec(); }
    
    let mut peaks = Vec::with_capacity(num_peaks);
    for i in 0..num_peaks {
        let start = i * samples_per_peak;
        let end = start + samples_per_peak;
        let block = &samples[start..end];
        let sum_squares: f32 = block.iter().map(|s| s * s).sum();
        peaks.push((sum_squares / block.len() as f32).sqrt());
    }
    
    let mut sorted = peaks.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_99 = sorted[(sorted.len() as f32 * 0.99) as usize].max(0.001);
    
    // ✅ Gamma 0.7 para preservar dinâmica
    peaks.iter_mut().for_each(|p| {
        *p = ((*p / percentile_99).min(1.0)).powf(0.7);
    });
    peaks
}

/// ✅ Gera waveform completa com picos positivos e negativos (min/max envelope)
pub fn generate_full_waveform(samples: &[f32], num_points: usize) -> FullWaveform {
    if samples.is_empty() || num_points == 0 {
        return FullWaveform { min: vec![], max: vec![] };
    }

    let samples_per_point = samples.len() / num_points;
    if samples_per_point == 0 {
        return FullWaveform { min: vec![0.0; num_points], max: vec![0.0; num_points] };
    }

    let mut min_vals = Vec::with_capacity(num_points);
    let mut max_vals = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let start = i * samples_per_point;
        let end = start + samples_per_point;
        let block = &samples[start..end];

        let mut min_val = 0.0f32;
        let mut max_val = 0.0f32;

        for &s in block {
            if s < min_val { min_val = s; }
            if s > max_val { max_val = s; }
        }

        min_vals.push(min_val);
        max_vals.push(max_val);
    }

    // Normalização
    let mut all_values: Vec<f32> = min_vals.iter().chain(max_vals.iter()).map(|v| v.abs()).collect();
    all_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_95 = all_values[(all_values.len() as f32 * 0.95) as usize].max(0.001);

    min_vals.iter_mut().for_each(|v| {
        let normalized = (*v / percentile_95).clamp(-1.0, 1.0);
        *v = normalized.signum() * normalized.abs().powf(0.5);
    });

    max_vals.iter_mut().for_each(|v| {
        let normalized = (*v / percentile_95).clamp(-1.0, 1.0);
        *v = normalized.signum() * normalized.abs().powf(0.5);
    });

    FullWaveform { min: min_vals, max: max_vals }
}