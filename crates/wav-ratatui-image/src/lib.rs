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
            .map(|s| s.map_err(|e| Box::new(e) as Box<dyn std::error::Error>))
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
    use super::*;

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
}
