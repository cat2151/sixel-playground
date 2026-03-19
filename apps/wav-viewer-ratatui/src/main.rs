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
use hound::WavReader;
use image::{DynamicImage, ImageBuffer, Rgba};
use ratatui::{
    backend::CrosstermBackend,
    layout::Margin,
    widgets::{Block, Borders},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use wave_chart::render_waveform;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 300;

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
    let samples = read_wav(path).unwrap_or_else(|e| {
        eprintln!("Error reading {path}: {e}");
        process::exit(1);
    });

    let rgba = render_waveform(&samples, DEFAULT_WIDTH, DEFAULT_HEIGHT);
    let image = rgba_to_dynamic_image(rgba, DEFAULT_WIDTH, DEFAULT_HEIGHT)?;

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

fn read_wav(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
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
