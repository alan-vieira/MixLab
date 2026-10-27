use realfft::RealFftPlanner;
use realfft::num_complex::Complex;

pub fn detect_bpm(samples: &[f32], sample_rate: f32) -> f32 {
    let max_samples = (sample_rate * 60.0) as usize;
    let analysis_samples = if samples.len() > max_samples {
        &samples[..max_samples]
    } else {
        samples
    };

    if analysis_samples.len() < (sample_rate as usize) {
        return 0.0;
    }

    let frame_size = 1024;
    let hop_size = 512;
    let mut energy_diffs = Vec::new();
    
    for i in (0..analysis_samples.len() - frame_size).step_by(hop_size) {
        let current_energy: f32 = analysis_samples[i..i + frame_size].iter().map(|s| s * s).sum();
        let prev_energy: f32 = if i >= hop_size {
            analysis_samples[i - hop_size..i - hop_size + frame_size].iter().map(|s| s * s).sum()
        } else {
            0.0
        };
        let diff = (current_energy - prev_energy).max(0.0);
        energy_diffs.push(diff);
    }

    let n = energy_diffs.len();
    if n == 0 {
        return 0.0;
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);
    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut energy_diffs, &mut spectrum).unwrap();
    
    let mut power_spectrum: Vec<Complex<f32>> = spectrum.iter().map(|c| {
        Complex::new(c.norm_sqr(), 0.0)
    }).collect();

    let c2r = planner.plan_fft_inverse(n);
    let mut autocorr = c2r.make_output_vec();
    c2r.process(&mut power_spectrum, &mut autocorr).unwrap();

    // ✅ CORREÇÃO CRÍTICA: Faixa correta para 60-200 BPM
    let min_lag_frames = 26;  // ~200 BPM
    let max_lag_frames = 86;  // ~60 BPM
    
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

    let seconds_per_frame = hop_size as f32 / sample_rate;
    let seconds_per_beat = best_lag as f32 * seconds_per_frame;
    let bpm = 60.0 / seconds_per_beat;
    
    bpm
}