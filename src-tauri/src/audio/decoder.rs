use std::fs::File;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::FromSample; // 👈 Trait correto da symphonia

pub struct AudioDecoder;

impl AudioDecoder {
    /// Decodifica o arquivo inteiro para um vetor de samples mono (f32)
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
        let duration_secs = codec_params.time_base.map(|tb| {
            (codec_params.n_frames.unwrap_or(0) as f64 * tb.numer as f64) / tb.denom as f64
        }).unwrap_or(0.0);

        let mut decoder = symphonia::default::get_codecs().make(codec_params, &DecoderOptions::default())?;
        let mut all_samples = Vec::new();
        
        while let Ok(packet) = format.next_packet() {
            let decoded = decoder.decode(&packet)?;
            let spec = decoded.spec();
            let num_channels = spec.channels.count();
            let frames = decoded.frames();
            
            for i in 0..frames {
                let mut sample_sum = 0.0f32;
                for ch in 0..num_channels {
                    // f32::from_sample converte QUALQUER tipo de sample da symphonia para f32 (-1.0 a 1.0)
                    let val: f32 = match &decoded {
                        AudioBufferRef::U8(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::U16(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::U24(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::U32(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::S8(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::S16(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::S24(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::S32(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::F32(b) => f32::from_sample(b.chan(ch)[i]),
                        AudioBufferRef::F64(b) => f32::from_sample(b.chan(ch)[i]),
                    };
                    sample_sum += val;
                }
                // Downmix para mono (média dos canais)
                all_samples.push(sample_sum / num_channels as f32);
            }
        }
        
        Ok((all_samples, sample_rate, channels, duration_secs))
    }
}