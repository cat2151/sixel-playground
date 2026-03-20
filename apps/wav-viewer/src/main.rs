//! `wav-viewer` — display the waveform of a WAV file inline in the terminal
//! using sixel graphics.
//!
//! # Usage
//! ```text
//! wav-viewer <path/to/file.wav>
//! ```
//!
//! The program reads the first channel of the WAV file, normalises the samples
//! to `[-1.0, 1.0]`, renders them as a line chart with [wave-chart], encodes
//! the chart as a sixel escape sequence with [sixel-encoder], and prints it to
//! stdout.  The output is visible in any sixel-capable terminal emulator (e.g.
//! WezTerm, mlterm, xterm with `+sixel`, foot …).

use std::{env, process};

use hound::WavReader;
use sixel_encoder::encode_rgba_to_sixel;
use wave_chart::render_waveform;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 300;

fn main() {
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

    eprintln!(
        "Loaded {} samples from '{path}'",
        samples.len()
    );

    // Render waveform → RGBA image
    let rgba = render_waveform(&samples, DEFAULT_WIDTH, DEFAULT_HEIGHT);

    // Encode RGBA → sixel string
    let sixel = encode_rgba_to_sixel(&rgba, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap_or_else(|e| {
        eprintln!("Sixel encoding error: {e}");
        process::exit(1);
    });

    // Print the sixel escape sequence — the terminal will render it as an image
    print!("{sixel}");
}

/// Read the first channel of a WAV file and normalise samples to `[-1.0, 1.0]`.
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
