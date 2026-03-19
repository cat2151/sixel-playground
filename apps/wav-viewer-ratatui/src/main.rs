//! `wav-viewer-ratatui` — display the waveform of a WAV file in a TUI using
//! ratatui-image.
//!
//! # Usage
//! ```text
//! wav-viewer-ratatui <path/to/file.wav>
//! ```
//! Press `q` to quit.

use std::{env, io, process};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Margin,
    widgets::{Block, Borders},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use wav_ratatui_image::render_wav_as_image;

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
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path/to/file.wav>", args[0]);
        process::exit(1);
    }

    let path = &args[1];
    let image = render_wav_as_image(path).unwrap_or_else(|e| {
        eprintln!("Error reading {path}: {e}");
        process::exit(1);
    });

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
                .title(format!("WAV Viewer ({path}) - press q to quit"));
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

    if let Some(result) = protocol.last_encoding_result() {
        result?;
    }

    Ok(())
}
