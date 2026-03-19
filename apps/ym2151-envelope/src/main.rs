//! `ym2151-envelope` — visualise a YM2151 (OPM) ADSR envelope as a line chart
//! rendered inline in the terminal using sixel graphics.
//!
//! The YM2151 OPM chip uses a logarithmic envelope generator with four phases:
//!
//! | Phase  | Register | Description                              |
//! |--------|----------|------------------------------------------|
//! | Attack | AR (0-31)| Rate at which the envelope rises         |
//! | Decay  | D1R(0-31)| Rate at which it falls to the sustain lv |
//! | Sustain| D1L(0-15)| Level at which decay gives way to D2     |
//! | Sustain 2 (Decay during key-on) | D2R(0-31) | Slow decay during sustain |
//! | Release| RR (0-15)| Rate at which it falls on key-off        |
//!
//! # Usage
//! ```text
//! ym2151-envelope [AR] [D1R] [D1L] [D2R] [RR]
//! ```
//! All parameters are optional and default to a representative example.
//!
//! # Example
//! ```text
//! ym2151-envelope 31 10 8 4 8
//! ```

use sixel_encoder::encode_rgba_to_sixel;
use wave_chart::render_line_chart;
use ym2151_envelope_core::{parse_args, simulate_envelope};

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 300;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let params = parse_args(&args);

    // Simulate the envelope and collect (time, level) data points
    let points = simulate_envelope(&params);

    // Render line chart → RGBA image
    let rgba = render_line_chart(&points, DEFAULT_WIDTH, DEFAULT_HEIGHT);

    // Encode RGBA → sixel string
    let sixel = encode_rgba_to_sixel(&rgba, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap_or_else(|e| {
        eprintln!("Sixel encoding error: {e}");
        std::process::exit(1);
    });

    // Print the sixel escape sequence
    print!("{sixel}");
}
