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
//! | Sustain 2 (Release during key-on) | D2R(0-31) | Slow decay during sustain |
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

use std::env;

use sixel_encoder::encode_rgba_to_sixel;
use wave_chart::render_line_chart;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 300;

/// YM2151 envelope parameters (all values clamped to valid ranges).
struct EnvParams {
    /// Attack rate (AR): 0-31, higher = faster attack
    ar: u32,
    /// Decay 1 rate (D1R): 0-31, fall rate to sustain level
    d1r: u32,
    /// Decay 1 level / sustain level (D1L): 0-15 (0 = full level, 15 = -45 dB)
    d1l: u32,
    /// Decay 2 rate (D2R): 0-31, slow fall during key-on sustain
    d2r: u32,
    /// Release rate (RR): 0-15, fall rate after key-off
    rr: u32,
}

impl Default for EnvParams {
    fn default() -> Self {
        Self {
            ar: 28,
            d1r: 12,
            d1l: 4,
            d2r: 2,
            rr: 8,
        }
    }
}

fn main() {
    let params = parse_args();

    // Simulate the envelope and collect (time, level) data points
    let points = simulate_envelope(&params);

    // Render line chart → RGBA image
    let title = format!(
        "YM2151 Envelope  AR={} D1R={} D1L={} D2R={} RR={}",
        params.ar, params.d1r, params.d1l, params.d2r, params.rr
    );
    let rgba = render_line_chart(&points, DEFAULT_WIDTH, DEFAULT_HEIGHT, &title);

    // Encode RGBA → sixel string
    let sixel =
        encode_rgba_to_sixel(&rgba, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap_or_else(|e| {
            eprintln!("Sixel encoding error: {e}");
            std::process::exit(1);
        });

    // Print the sixel escape sequence
    print!("{sixel}");
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

fn parse_args() -> EnvParams {
    let args: Vec<String> = env::args().collect();
    let parse = |i: usize, max: u32, default: u32| -> u32 {
        args.get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(default)
            .min(max)
    };

    let defaults = EnvParams::default();
    EnvParams {
        ar: parse(1, 31, defaults.ar),
        d1r: parse(2, 31, defaults.d1r),
        d1l: parse(3, 15, defaults.d1l),
        d2r: parse(4, 31, defaults.d2r),
        rr: parse(5, 15, defaults.rr),
    }
}

// ---------------------------------------------------------------------------
// Envelope simulation
// ---------------------------------------------------------------------------

/// Simulate a simplified YM2151 envelope and return `(time, level)` points.
///
/// The envelope level is expressed on a linear 0.0–1.0 scale for display
/// purposes (internally the YM2151 uses a 10-bit attenuation value, but we
/// map that to 0 = max attenuation / silence, 1 = zero attenuation / full
/// volume for an intuitive chart).
fn simulate_envelope(p: &EnvParams) -> Vec<(f64, f64)> {
    // Convert register values to per-sample rates (heuristic approximation).
    // Real hardware clocks at ~55.9 kHz envelope update rate; here we use
    // arbitrary time units so the chart looks proportional.
    const TOTAL_SAMPLES: usize = 2000;
    const KEY_OFF_SAMPLE: usize = 1400; // simulate key-off at 70 % of the time

    // Attenuation in the range 0 (full level) .. 1023 (silence).
    // We store as f64 for smooth simulation.
    const MAX_ATT: f64 = 1023.0;

    // Attack rate → attenuation decrease per sample (heuristic)
    let attack_rate = if p.ar == 0 {
        0.0
    } else {
        MAX_ATT / (512.0 / (p.ar as f64 + 1.0)).max(1.0)
    };

    // Decay 1 rate → attenuation increase per sample
    let d1_rate = if p.d1r == 0 {
        0.0
    } else {
        MAX_ATT / (2048.0 / (p.d1r as f64 + 1.0)).max(1.0)
    };

    // Sustain target level (D1L): 0 = full volume, 15 = silence (each step = ~3 dB)
    let sustain_att = p.d1l as f64 / 15.0 * MAX_ATT;

    // Decay 2 rate (very slow during key-on)
    let d2_rate = if p.d2r == 0 {
        0.0
    } else {
        MAX_ATT / (16384.0 / (p.d2r as f64 + 1.0)).max(1.0)
    };

    // Release rate
    let release_rate = if p.rr == 0 {
        0.0
    } else {
        MAX_ATT / (1024.0 / (p.rr as f64 + 1.0)).max(1.0)
    };

    let mut points = Vec::with_capacity(TOTAL_SAMPLES);
    let mut att: f64 = MAX_ATT; // start fully attenuated

    for t in 0..TOTAL_SAMPLES {
        let key_on = t < KEY_OFF_SAMPLE;

        if key_on {
            if att > 0.0 {
                // Attack phase: drive attenuation down to 0
                att = (att - attack_rate).max(0.0);
            } else if att < sustain_att {
                // Decay 1 phase: rise to sustain level
                att = (att + d1_rate).min(sustain_att);
            } else {
                // Decay 2 phase: slow decay during key-on sustain
                att = (att + d2_rate).min(MAX_ATT);
            }
        } else {
            // Key-off: release phase
            att = (att + release_rate).min(MAX_ATT);
        }

        // Convert attenuation to level (0 = silence, 1 = full volume)
        let level = 1.0 - att / MAX_ATT;
        points.push((t as f64, level));
    }

    points
}
