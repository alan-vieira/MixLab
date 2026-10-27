use std::fs::File;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::FromSample;

pub struct AudioDecoder;

impl AudioDecoder {
    /// Decodifica o arquivo inteiro para um vetor de samples mono (f32)
    /// OTIMIZADO: Pré-aloca vetor e usa downmix mais eficiente
    pub fn load_and_decode(path: &str) -> Result<(Vec<f32>, u32, u16, f64), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let hint = Hint::new();
        let format_opts = FormatOptions::default();
        let meta_opts = MetadataOptions::default();
        let probe = get_probe();
        let probe_result = probe.format(&hint, mss, &format_opts, &meta_opts)?;
        let mut format = probe_result.format;

        let track = format.default_track().ok_or("Nenhuma trilha de áudio encontrada")?;
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let num_frames = codec_params.n_frames.unwrap_or(0);

        let duration_secs = codec_params.time_base.map(|tb| {
            (num_frames as f64 * tb.numer as f64) / tb.denom as f64
        }).unwrap_or(0.0);

        let mut decoder = symphonia::default::get_codecs().make(codec_params, &DecoderOptions::default())?;

        // OTIMIZAÇÃO: Pré-aloca o vetor com tamanho estimado
        let estimated_samples = if num_frames > 0 {
            (num_frames as usize) * (channels as usize)
        } else {
            44100 * 60 * 5 // Estimativa para 5 minutos
        };
        let mut all_samples = Vec::with_capacity(estimated_samples);

        while let Ok(packet) = format.next_packet() {
            let decoded = decoder.decode(&packet)?;
            let spec = decoded.spec();
            let num_channels = spec.channels.count();
            let frames = decoded.frames();

            // OTIMIZAÇÃO: Processa por frames em vez de sample por sample
            match &decoded {
                AudioBufferRef::U8(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::U16(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::U24(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::U32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S8(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S16(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S24(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::F32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::F64(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels).map(|ch| f32::from_sample(b.chan(ch)[i])).sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
            }
        }

        Ok((all_samples, sample_rate, channels, duration_secs))
    }
}