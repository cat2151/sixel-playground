//! `wave-chart` — generate RGBA image data from waveforms and line charts.
//!
//! Uses [plotters](https://crates.io/crates/plotters) with `BitMapBackend` to
//! draw into an in-memory RGB buffer, then converts it to RGBA so it is ready
//! for downstream sixel encoding.
//!
//! No text or labels are rendered inside the chart, so no font infrastructure
//! is required.

use plotters::prelude::*;

const MONOKAI_GREEN: RGBColor = RGBColor(166, 226, 46);
const ORANGE: RGBColor = RGBColor(255, 165, 0);

/// Render a waveform from a slice of normalised samples (`-1.0 ..= 1.0`) into
/// an RGBA image buffer.
///
/// # Arguments
/// * `samples` – audio samples (normalised to `[-1.0, 1.0]`)
/// * `width`   – output image width in pixels
/// * `height`  – output image height in pixels
///
/// # Returns
/// A `Vec<u8>` containing RGBA pixels (4 bytes per pixel, row-major).
pub fn render_waveform(samples: &[f32], width: u32, height: u32) -> Vec<u8> {
    let mut rgb_buf = vec![0u8; (width * height * 3) as usize];
    let aggregated = aggregate_waveform_columns(samples, width as usize);

    {
        let root = BitMapBackend::with_buffer(&mut rgb_buf, (width, height)).into_drawing_area();
        root.fill(&BLACK).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .margin(0)
            .build_cartesian_2d(0u32..width.max(1), -1.0f32..1.0f32)
            .unwrap();

        chart.configure_mesh().disable_mesh().draw().unwrap();

        chart
            .draw_series(
                aggregated
                    .into_iter()
                    .enumerate()
                    .filter_map(|(x, column)| {
                        column.map(|(min, max)| {
                            PathElement::new(vec![(x as u32, min), (x as u32, max)], MONOKAI_GREEN)
                        })
                    }),
            )
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
///
/// # Returns
/// A `Vec<u8>` containing RGBA pixels (4 bytes per pixel, row-major).
pub fn render_line_chart(points: &[(f64, f64)], width: u32, height: u32) -> Vec<u8> {
    let mut rgb_buf = vec![0u8; (width * height * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut rgb_buf, (width, height)).into_drawing_area();
        root.fill(&BLACK).unwrap();

        let (x_min, x_max, y_min, y_max) = data_range(points);

        let mut chart = ChartBuilder::on(&root)
            .margin(5)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .unwrap();

        chart.configure_mesh().disable_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(points.iter().copied(), &ORANGE))
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

fn aggregate_waveform_columns(samples: &[f32], columns: usize) -> Vec<Option<(f32, f32)>> {
    if columns == 0 {
        return Vec::new();
    }

    if samples.is_empty() {
        return vec![None; columns];
    }

    let sample_count = samples.len();
    let mut aggregated: Vec<Option<(f32, f32)>> = vec![None; columns];

    for (column, entry) in aggregated.iter_mut().enumerate() {
        let start = column * sample_count / columns;
        let upper_bound = (column + 1) * sample_count;
        let quotient = upper_bound / columns;
        let end = match quotient * columns == upper_bound {
            true => quotient,
            false => quotient + 1,
        };

        if start >= sample_count || start >= end {
            continue;
        }

        let (min, max) = samples[start..end]
            .iter()
            .map(|sample| sample.clamp(-1.0, 1.0))
            .fold((0.0f32, 0.0f32), |(min, max), sample| {
                if sample < 0.0 {
                    (min.min(sample), max)
                } else {
                    (min, max.max(sample))
                }
            });

        *entry = Some((min, max));
    }

    aggregated
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
        let rgba = render_line_chart(&points, 320, 200);
        assert_eq!(rgba.len(), (320 * 200 * 4) as usize);
    }

    #[test]
    fn empty_waveform_does_not_panic() {
        let rgba = render_waveform(&[], 64, 64);
        assert_eq!(rgba.len(), (64 * 64 * 4) as usize);
        assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
    }

    #[test]
    fn empty_line_chart_does_not_panic() {
        let rgba = render_line_chart(&[], 64, 64);
        assert_eq!(rgba.len(), (64 * 64 * 4) as usize);
        assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
    }

    #[test]
    fn line_chart_uses_orange_on_black() {
        assert_eq!(ORANGE, RGBColor(255, 165, 0));

        let points: Vec<(f64, f64)> = (0..100)
            .map(|i| (i as f64, ((i as f64) / 10.0).sin()))
            .collect();
        let rgba = render_line_chart(&points, 128, 64);

        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| (pixel[0], pixel[1], pixel[2]) == (255, 165, 0)),
            "rendered line chart did not contain orange pixels"
        );
        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| (pixel[0], pixel[1], pixel[2]) == (0, 0, 0)),
            "rendered line chart did not contain black background pixels"
        );
    }

    #[test]
    fn waveform_uses_monokai_green() {
        assert_eq!(MONOKAI_GREEN, RGBColor(166, 226, 46));

        let samples: Vec<f32> = (0..100).map(|i| (i as f32 / 50.0).sin()).collect();
        let rgba = render_waveform(&samples, 64, 64);

        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| (pixel[0], pixel[1], pixel[2]) == (166, 226, 46)),
            "rendered waveform did not contain Monokai green pixels"
        );
    }

    #[test]
    fn waveform_columns_aggregate_positive_and_negative_extremes() {
        let aggregated =
            aggregate_waveform_columns(&[0.1, 0.4, -0.3, -0.8, 0.2, 0.7, -0.1, -0.5], 2);

        assert_eq!(aggregated, vec![Some((-0.8, 0.4)), Some((-0.5, 0.7))]);
    }

    #[test]
    fn waveform_columns_preserve_flat_zero_signal() {
        let aggregated = aggregate_waveform_columns(&[0.0, 0.0, 0.0, 0.0], 2);

        assert_eq!(aggregated, vec![Some((0.0, 0.0)), Some((0.0, 0.0))]);
    }

    #[test]
    fn short_waveform_reaches_right_edge_column() {
        let aggregated = aggregate_waveform_columns(&[0.25, -0.5], 4);

        assert_eq!(aggregated.first(), Some(&Some((0.0, 0.25))));
        assert_eq!(aggregated.last(), Some(&Some((-0.5, 0.0))));
    }
}
