/// YM2151 envelope parameters (all values clamped to valid ranges).
#[derive(Debug, Clone, Copy)]
pub struct EnvParams {
    /// Attack rate (AR): 0-31, higher = faster attack
    pub ar: u32,
    /// Decay 1 rate (D1R): 0-31, fall rate to sustain level
    pub d1r: u32,
    /// Decay 1 level / sustain level (D1L): 0-15 (0 = full level, 15 = -45 dB)
    pub d1l: u32,
    /// Decay 2 rate (D2R): 0-31, slow fall during key-on sustain
    pub d2r: u32,
    /// Release rate (RR): 0-15, fall rate after key-off
    pub rr: u32,
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

/// Parse envelope parameters from command-line style argument list.
///
/// `args[0]` is expected to be the executable name; missing/invalid values use
/// defaults and values are clamped to YM2151 register ranges.
pub fn parse_args(args: &[String]) -> EnvParams {
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

/// Simulate a simplified YM2151 envelope and return `(time, level)` points.
///
/// The envelope level is expressed on a linear 0.0–1.0 scale for display
/// purposes (internally the YM2151 uses a 10-bit attenuation value).
pub fn simulate_envelope(p: &EnvParams) -> Vec<(f64, f64)> {
    // Convert register values to per-sample rates (heuristic approximation).
    const TOTAL_SAMPLES: usize = 2000;
    const KEY_OFF_SAMPLE: usize = 1400;
    const MAX_ATT: f64 = 1023.0;

    let attack_rate = if p.ar == 0 {
        0.0
    } else {
        MAX_ATT / (512.0 / (p.ar as f64 + 1.0)).max(1.0)
    };
    let d1_rate = if p.d1r == 0 {
        0.0
    } else {
        MAX_ATT / (2048.0 / (p.d1r as f64 + 1.0)).max(1.0)
    };
    let sustain_att = p.d1l as f64 / 15.0 * MAX_ATT;
    let d2_rate = if p.d2r == 0 {
        0.0
    } else {
        MAX_ATT / (16384.0 / (p.d2r as f64 + 1.0)).max(1.0)
    };
    let release_rate = if p.rr == 0 {
        0.0
    } else {
        MAX_ATT / (1024.0 / (p.rr as f64 + 1.0)).max(1.0)
    };

    let mut points = Vec::with_capacity(TOTAL_SAMPLES);
    let mut att: f64 = MAX_ATT;

    for t in 0..TOTAL_SAMPLES {
        let key_on = t < KEY_OFF_SAMPLE;
        if key_on {
            if att > 0.0 {
                att = (att - attack_rate).max(0.0);
            } else if att < sustain_att {
                att = (att + d1_rate).min(sustain_att);
            } else {
                att = (att + d2_rate).min(MAX_ATT);
            }
        } else {
            att = (att + release_rate).min(MAX_ATT);
        }
        let level = 1.0 - att / MAX_ATT;
        points.push((t as f64, level));
    }
    points
}
