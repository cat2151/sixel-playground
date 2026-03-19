//! `ym2151-envelope-ratatui` — visualise a YM2151 (OPM) ADSR envelope as a line
//! chart rendered in a TUI using ratatui-image.
//!
//! # Usage
//! ```text
//! ym2151-envelope-ratatui [AR] [D1R] [D1L] [D2R] [RR]
//! ```
//! Press `q` to quit.

use std::io;

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
use ym2151_envelope_core::{parse_args, simulate_envelope};

struct RawModeGuard;

impl RawModeGuard {
    fn acquire() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let params = parse_args(&args);
    let points = simulate_envelope(&params);
    let rgba = render_line_chart(&points, 800, 300);
    let image = rgba_to_dynamic_image(rgba, 800, 300)?;

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let mut protocol: StatefulProtocol = picker.new_resize_protocol(image);

    let _raw_mode_guard = RawModeGuard::acquire()?;
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

    // Surface any image encoding error if it happened.
    if let Some(result) = protocol.last_encoding_result() {
        result?;
    }
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
