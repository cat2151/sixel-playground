use std::io;

use image::{DynamicImage, ImageBuffer, Rgba};
use wave_chart::render_line_chart;
use ym2151_envelope_core::{parse_args, simulate_envelope};

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 300;

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

/// Builds a YM2151 envelope image from command-line-style arguments, using
/// default dimensions.
pub fn render_ym2151_as_image_from_args(
    args: &[String],
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    render_ym2151_as_image_from_args_with_size(args, DEFAULT_WIDTH, DEFAULT_HEIGHT)
}

/// Builds a YM2151 envelope image from command-line-style arguments with
/// explicit dimensions.
pub fn render_ym2151_as_image_from_args_with_size(
    args: &[String],
    width: u32,
    height: u32,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let params = parse_args(args);
    let points = simulate_envelope(&params);
    let rgba = render_line_chart(&points, width, height);
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

    #[test]
    fn render_ym2151_as_image_from_args_with_size_returns_requested_dimensions() {
        let args = vec![
            "prog".to_string(),
            "28".to_string(),
            "12".to_string(),
            "4".to_string(),
            "2".to_string(),
            "8".to_string(),
        ];

        let image = render_ym2151_as_image_from_args_with_size(&args, 123, 45)
            .expect("failed to render YM2151 envelope image");

        assert_eq!(image.width(), 123);
        assert_eq!(image.height(), 45);
    }
}
