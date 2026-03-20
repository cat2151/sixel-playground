use std::io;
use std::path::Path;

use hound::WavReader;
use image::{DynamicImage, ImageBuffer, Rgba};
use wave_chart::render_waveform;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 300;

/// Reads samples from the first channel of a WAV file and normalizes them to
/// [-1.0, 1.0].
pub fn read_wav<P: AsRef<Path>>(path: P) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .step_by(spec.channels as usize)
            .map(|s| {
                s.map(|v| v.clamp(-1.0, 1.0))
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .step_by(spec.channels as usize)
                .map(|s| {
                    s.map(|v| v as f32 / max)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };

    Ok(samples)
}

/// Converts rendered RGBA bytes into a [`DynamicImage`].
pub fn rgba_to_dynamic_image(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "failed to build image buffer from rendered RGBA bytes",
        )
    })?;
    Ok(DynamicImage::ImageRgba8(image))
}

/// Builds a waveform image from WAV file path using default dimensions.
pub fn render_wav_as_image<P: AsRef<Path>>(
    path: P,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    render_wav_as_image_with_size(path, DEFAULT_WIDTH, DEFAULT_HEIGHT)
}

/// Builds a waveform image from WAV file path with explicit dimensions.
pub fn render_wav_as_image_with_size<P: AsRef<Path>>(
    path: P,
    width: u32,
    height: u32,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let samples = read_wav(path)?;
    let rgba = render_waveform(&samples, width, height);
    rgba_to_dynamic_image(rgba, width, height)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};

    fn temp_wav_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed to get current time for generating unique test file name")
            .as_nanos();
        path.push(format!("wav-ratatui-image-{name}-{nanos}.wav"));
        path
    }

    fn write_wav_i16(path: &Path, channels: u16, frames: &[i16]) {
        let spec = WavSpec {
            channels,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).expect("failed to create test WAV file");
        for sample in frames {
            writer
                .write_sample(*sample)
                .expect("failed to write sample to test WAV file");
        }
        writer.finalize().expect("failed to finalize test WAV file");
    }

    fn write_wav_f32(path: &Path, channels: u16, frames: &[f32]) {
        let spec = WavSpec {
            channels,
            sample_rate: 8000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(path, spec).expect("failed to create test WAV file");
        for sample in frames {
            writer
                .write_sample(*sample)
                .expect("failed to write sample to test WAV file");
        }
        writer.finalize().expect("failed to finalize test WAV file");
    }

    #[test]
    fn rgba_to_dynamic_image_size_matches() {
        let width = 3;
        let height = 2;
        let rgba = vec![255; (width * height * 4) as usize];
        let image = rgba_to_dynamic_image(rgba, width, height).expect("valid rgba data");
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
    }

    #[test]
    fn rgba_to_dynamic_image_rejects_invalid_length() {
        let result = rgba_to_dynamic_image(vec![0; 3], 1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn read_wav_reads_first_channel_for_stereo_int_pcm() {
        let path = temp_wav_path("stereo-int");
        // interleaved stereo: (L0, R0, L1, R1)
        write_wav_i16(&path, 2, &[1000, -1000, 2000, -2000]);

        let samples = read_wav(&path).expect("failed to read test WAV file");
        std::fs::remove_file(&path).expect("failed to remove temporary test WAV file");

        assert_eq!(samples.len(), 2);
        let max = 32768.0_f32;
        assert!((samples[0] - (1000.0 / max)).abs() < 1e-6);
        assert!((samples[1] - (2000.0 / max)).abs() < 1e-6);
    }

    #[test]
    fn read_wav_clamps_float_samples_to_documented_range() {
        let path = temp_wav_path("float-clamp");
        write_wav_f32(&path, 1, &[-1.5, -0.5, 0.5, 1.5]);

        let samples = read_wav(&path).expect("failed to read test WAV file");
        std::fs::remove_file(&path).expect("failed to remove temporary test WAV file");

        assert_eq!(samples, vec![-1.0, -0.5, 0.5, 1.0]);
    }

    #[test]
    fn render_wav_as_image_with_size_returns_requested_dimensions() {
        let path = temp_wav_path("render-size");
        write_wav_i16(&path, 1, &[0, 1000, -1000, 0]);

        let image = render_wav_as_image_with_size(&path, 123, 45)
            .expect("failed to render test waveform image");
        std::fs::remove_file(&path).expect("failed to remove temporary test WAV file");

        assert_eq!(image.width(), 123);
        assert_eq!(image.height(), 45);
    }
}
