/// Resample linear simples (suficiente para pitch correto no load).
///
/// Faz downsample ou upsample de um buffer de samples para o sample rate de destino.
/// É executado uma única vez no load_track (offline), não no thread de áudio real-time.
pub fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        if idx + 1 < input.len() {
            let s0 = input[idx];
            let s1 = input[idx + 1];
            output.push(s0 * (1.0 - frac as f32) + s1 * frac as f32);
        } else if idx < input.len() {
            output.push(input[idx]);
        } else {
            output.push(0.0);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate_returns_copy() {
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let output = resample_linear(&input, 44100, 44100);
        assert_eq!(input, output);
    }

    #[test]
    fn test_resample_halves_length() {
        let input = vec![0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0];
        let output = resample_linear(&input, 44100, 22050);
        // 8 samples @ 44100 → ~4 samples @ 22050
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_resample_doubles_length() {
        let input = vec![0.0, 1.0, 0.0, -1.0];
        let output = resample_linear(&input, 22050, 44100);
        // 4 samples @ 22050 → ~8 samples @ 44100
        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_resample_preserves_dc_component() {
        let input = vec![1.0; 1000];
        let output = resample_linear(&input, 44100, 22050);
        for &sample in &output {
            assert!((sample - 1.0).abs() < 0.001, "DC component not preserved");
        }
    }
}
