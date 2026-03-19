//! `ym2151-envelope-ratatui` — visualise a YM2151 (OPM) ADSR envelope as a line
//! chart rendered in a TUI using ratatui-image.
//!
//! # Usage
//! ```text
//! ym2151-envelope-ratatui [AR] [D1R] [D1L] [D2R] [RR]
//! ```
//! Press `q` to quit.

use std::{env, io};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use image::{DynamicImage, ImageBuffer, Rgba};
use ratatui::{
    backend::CrosstermBackend,
    layout::Margin,
    widgets::{Block, Borders},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use wave_chart::render_line_chart;

/// YM2151 envelope parameters (all values clamped to valid ranges).
struct EnvParams {
    ar: u32,
    d1r: u32,
    d1l: u32,
    d2r: u32,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let params = parse_args();
    let points = simulate_envelope(&params);
    let rgba = render_line_chart(&points, 800, 300);
    let image = rgba_to_dynamic_image(rgba, 800, 300)?;

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let mut protocol: StatefulProtocol = picker.new_resize_protocol(image);

    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("YM2151 Envelope (ratatui-image) - press q to quit");
            let inner = block.inner(area).inner(Margin {
                vertical: 1,
                horizontal: 1,
            });
            f.render_widget(block, area);
            f.render_stateful_widget(StatefulImage::default(), inner, &mut protocol);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
        }
    }

    disable_raw_mode()?;
    // Surface any image encoding error if it happened.
    protocol.last_encoding_result().unwrap()?;
    Ok(())
}

fn rgba_to_dynamic_image(
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

fn simulate_envelope(p: &EnvParams) -> Vec<(f64, f64)> {
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
