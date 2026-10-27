use realfft::num_complex::Complex;
use realfft::RealFftPlanner;

pub fn detect_bpm(samples: &[f32], sample_rate: f32) -> f32 {
    // Analisa os primeiros 60 segundos
    let max_samples = (sample_rate * 60.0) as usize;
    let analysis_samples = if samples.len() > max_samples {
        &samples[..max_samples]
    } else {
        samples
    };

    if analysis_samples.len() < (sample_rate as usize) {
        return 0.0;
    }

    // ✅ ETAPA 1: FFT com foco EXCLUSIVO na banda de FREQUÊNCIAS GRAVES (Kick Drum)
    // Isso imita a "Wavelet Decomposition" do bpm-analyzer, ignorando ruídos de médios/agudos.
    let frame_size = 2048;
    let hop_size = 512;
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(frame_size);

    let mut low_freq_flux = Vec::new();
    let mut prev_low_energy = 0.0f32;

    // Limite de frequência para isolar o "bumbo" (ex: 250 Hz)
    let max_bin = ((250.0 * frame_size as f32) / sample_rate) as usize;

    for i in (0..analysis_samples.len() - frame_size).step_by(hop_size) {
        let block = &analysis_samples[i..i + frame_size];
        let mut fft_input = block.to_vec();

        let mut spectrum = r2c.make_output_vec();
        r2c.process(&mut fft_input, &mut spectrum).unwrap();

        // Soma APENAS a energia das frequências graves (< 250 Hz)
        let mut current_low_energy = 0.0f32;
        for spectrum_chunk in spectrum.iter().take(max_bin + 1) {
            current_low_energy += spectrum_chunk.norm_sqr();
        }

        // Calcula o "flux" (diferença positiva) apenas na banda grave
        let diff = (current_low_energy - prev_low_energy).max(0.0);
        low_freq_flux.push(diff);

        prev_low_energy = current_low_energy;
    }

    let n = low_freq_flux.len();
    if n == 0 {
        return 0.0;
    }

    // ✅ ETAPA 2: Autocorrelação do fluxo de graves
    let mut planner2 = RealFftPlanner::<f32>::new();
    let r2c2 = planner2.plan_fft_forward(n);
    let mut spectrum = r2c2.make_output_vec();
    r2c2.process(&mut low_freq_flux, &mut spectrum).unwrap();

    let mut power_spectrum: Vec<Complex<f32>> = spectrum
        .iter()
        .map(|c| Complex::new(c.norm_sqr(), 0.0))
        .collect();

    let c2r = planner2.plan_fft_inverse(n);
    let mut autocorr = c2r.make_output_vec();
    c2r.process(&mut power_spectrum, &mut autocorr).unwrap();

    // ✅ ETAPA 3: Normalização
    let lag0 = autocorr[0].abs();
    if lag0 > 0.0 {
        for val in autocorr.iter_mut() {
            *val /= lag0;
        }
    }

    // Faixa para 60-200 BPM
    let min_lag = 26; // ~200 BPM
    let max_lag = 86; // ~60 BPM

    // ✅ ETAPA 4: Coletar picos locais
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for lag in min_lag..max_lag.min(n) {
        let corr = autocorr[lag];
        if corr > 0.1 {
            let is_local_max = if lag > min_lag && lag < max_lag.min(n) - 1 {
                corr >= autocorr[lag - 1] && corr >= autocorr[lag + 1]
            } else {
                true
            };
            if is_local_max {
                candidates.push((lag, corr));
            }
        }
    }

    if candidates.is_empty() {
        return 0.0;
    }

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // ✅ ETAPA 5: Scoring Musical (Half-time / Double-time)
    let seconds_per_frame = hop_size as f32 / sample_rate;
    let mut best_bpm = 0.0f32;
    let mut best_score = -1.0f32;

    for &(lag, corr) in candidates.iter().take(5) {
        let bpm_raw = 60.0 / (lag as f32 * seconds_per_frame);

        for &multiplier in &[0.5, 1.0, 2.0] {
            let bpm = bpm_raw * multiplier;
            let mut score = corr;

            // Bônus para faixa eletrônica
            if (90.0..=150.0).contains(&bpm) {
                score += 0.6;
            }

            // Bônus para BPMs comuns
            let common_bpms = [
                95.0, 96.0, 100.0, 120.0, 124.0, 126.0, 128.0, 130.0, 132.0, 135.0, 138.0, 140.0,
            ];
            for &common in &common_bpms {
                if (bpm - common).abs() < 1.5 {
                    score += 0.3;
                }
            }

            if bpm < 80.0 || bpm > 180.0 {
                score *= 0.4;
            }

            if score > best_score {
                best_score = score;
                best_bpm = bpm;
            }
        }
    }

    (best_bpm * 10.0).round() / 10.0
}
