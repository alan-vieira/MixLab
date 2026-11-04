// Probe temporário: verifica se detect_bpm entra em panic quando o número de
// frames de flux não é potência de 2 (exigência do realfft).
use mixlab_lib::detect_bpm;

#[test]
fn probe_non_power_of_two_flux_count() {
    // 44100 Hz, 60 s -> ~5180 frames de flux com hop 512 e frame 2048.
    // 5180 não é potência de 2. Se o realfft exigir POT, o plano ou o panic estouram.
    let sr = 44100usize;
    let samples: Vec<f32> = (0..sr * 10)
        .map(|i| (i as f32 * 0.01).sin() * 0.5)
        .collect();
    let bpm = detect_bpm(&samples, sr as f32);
    println!("BPM detectado (10s): {}", bpm);
}

#[test]
fn probe_short_file_under_frame_size() {
    // 0.5 s -> passa o guard de 1 s? 44100*0.5 = 22050 < 44100 -> retorna 0.0
    let sr = 44100usize;
    let samples: Vec<f32> = vec![0.1; sr / 2];
    let bpm = detect_bpm(&samples, sr as f32);
    println!("BPM arquivo curto: {}", bpm);
}
