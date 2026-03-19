//! `wave-chart` — generate RGBA image data from waveforms and line charts.
//!
//! Uses [plotters](https://crates.io/crates/plotters) with `BitMapBackend` to
//! draw into an in-memory RGB buffer, then converts it to RGBA so it is ready
//! for downstream sixel encoding.
//!
//! A bundled subset of the DejaVu Sans font (Bitstream Vera licence, public
//! domain changes) is registered on first use so that axis labels render
//! correctly without requiring system fonts.

use plotters::prelude::*;
use plotters::style::register_font;
use std::sync::OnceLock;

/// Bytes of the bundled DejaVu Sans font, included at compile time.
static FONT_BYTES: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

/// Ensure the bundled font is registered with plotters exactly once.
fn ensure_font() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        register_font("sans-serif", FontStyle::Normal, FONT_BYTES)
            .unwrap_or_else(|_| panic!("bundled DejaVuSans.ttf should be valid"));
    });
}

/// Render a waveform from a slice of normalised samples (`-1.0 ..= 1.0`) into
/// an RGBA image buffer.
///
/// # Arguments
/// * `samples`  – audio samples (normalised to `[-1.0, 1.0]`)
/// * `width`    – output image width in pixels
/// * `height`   – output image height in pixels
///
/// # Returns
/// A `Vec<u8>` containing RGBA pixels (4 bytes per pixel, row-major).
pub fn render_waveform(samples: &[f32], width: u32, height: u32) -> Vec<u8> {
    ensure_font();
    let mut rgb_buf = vec![0u8; (width * height * 3) as usize];

    {
        let root =
            BitMapBackend::with_buffer(&mut rgb_buf, (width, height)).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let x_max = samples.len().max(1) as f32;
        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .x_label_area_size(20)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..x_max, -1.0f32..1.0f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                samples
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| (i as f32, s.clamp(-1.0, 1.0))),
                &BLUE,
            ))
            .unwrap();

        root.present().unwrap();
    }

    rgb_to_rgba(&rgb_buf)
}

/// Render a line chart from a slice of `(x, y)` data points into an RGBA
/// image buffer.
///
/// # Arguments
/// * `points` – slice of `(x, y)` pairs
/// * `width`  – output image width in pixels
/// * `height` – output image height in pixels
/// * `title`  – chart title shown at the top
///
/// # Returns
/// A `Vec<u8>` containing RGBA pixels (4 bytes per pixel, row-major).
pub fn render_line_chart(points: &[(f64, f64)], width: u32, height: u32, title: &str) -> Vec<u8> {
    ensure_font();
    let mut rgb_buf = vec![0u8; (width * height * 3) as usize];

    {
        let root =
            BitMapBackend::with_buffer(&mut rgb_buf, (width, height)).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let (x_min, x_max, y_min, y_max) = data_range(points);

        let mut chart = ChartBuilder::on(&root)
            .caption(title, ("sans-serif", 16))
            .margin(10)
            .x_label_area_size(20)
            .y_label_area_size(40)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(points.iter().copied(), &RED))
            .unwrap();

        root.present().unwrap();
    }

    rgb_to_rgba(&rgb_buf)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a flat RGB buffer (3 bytes/pixel) to a flat RGBA buffer (4 bytes/pixel,
/// alpha = 255).
fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    rgb.chunks_exact(3)
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect()
}

/// Compute axis ranges with a small margin so the series is not clipped.
fn data_range(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    if points.is_empty() {
        return (0.0, 1.0, 0.0, 1.0);
    }
    let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let y_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let y_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

    let x_margin = (x_max - x_min).max(1.0) * 0.05;
    let y_margin = (y_max - y_min).max(1.0) * 0.05;

    (
        x_min - x_margin,
        x_max + x_margin,
        y_min - y_margin,
        y_max + y_margin,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waveform_output_size() {
        let samples: Vec<f32> = (0..100).map(|i| (i as f32 / 50.0).sin()).collect();
        let rgba = render_waveform(&samples, 320, 200);
        assert_eq!(rgba.len(), (320 * 200 * 4) as usize);
    }

    #[test]
    fn line_chart_output_size() {
        let points: Vec<(f64, f64)> = (0..50).map(|i| (i as f64, (i as f64).sin())).collect();
        let rgba = render_line_chart(&points, 320, 200, "Test");
        assert_eq!(rgba.len(), (320 * 200 * 4) as usize);
    }

    #[test]
    fn empty_waveform_does_not_panic() {
        let rgba = render_waveform(&[], 64, 64);
        assert_eq!(rgba.len(), (64 * 64 * 4) as usize);
    }

    #[test]
    fn empty_line_chart_does_not_panic() {
        let rgba = render_line_chart(&[], 64, 64, "Empty");
        assert_eq!(rgba.len(), (64 * 64 * 4) as usize);
    }
}
