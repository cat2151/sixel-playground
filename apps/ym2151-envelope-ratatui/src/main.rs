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
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use ym2151_envelope_core::{parse_args, EnvParams};
use ym2151_ratatui_image::render_ym2151_as_image_from_args;

const PARAMETER_COUNT: usize = 5;
const PAGE_STEP: i32 = 4;

struct ParamSpec {
    label: &'static str,
    max: u32,
}

const PARAM_SPECS: [ParamSpec; PARAMETER_COUNT] = [
    ParamSpec {
        label: "AR",
        max: 31,
    },
    ParamSpec {
        label: "D1R",
        max: 31,
    },
    ParamSpec {
        label: "D1L",
        max: 15,
    },
    ParamSpec {
        label: "D2R",
        max: 31,
    },
    ParamSpec {
        label: "RR",
        max: 15,
    },
];

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

enum AppCommand {
    Continue,
    RefreshImage,
    Quit,
}

fn build_render_args(params: &EnvParams) -> Vec<String> {
    vec![
        "ym2151-envelope-ratatui".to_string(),
        params.ar.to_string(),
        params.d1r.to_string(),
        params.d1l.to_string(),
        params.d2r.to_string(),
        params.rr.to_string(),
    ]
}

fn render_protocol(
    picker: &Picker,
    params: &EnvParams,
) -> Result<StatefulProtocol, Box<dyn std::error::Error>> {
    let image = render_ym2151_as_image_from_args(&build_render_args(params))?;
    Ok(picker.new_resize_protocol(image))
}

fn selected_value_mut(params: &mut EnvParams, selected: usize) -> (&mut u32, u32) {
    match selected {
        0 => (&mut params.ar, PARAM_SPECS[0].max),
        1 => (&mut params.d1r, PARAM_SPECS[1].max),
        2 => (&mut params.d1l, PARAM_SPECS[2].max),
        3 => (&mut params.d2r, PARAM_SPECS[3].max),
        4 => (&mut params.rr, PARAM_SPECS[4].max),
        _ => unreachable!("selected parameter index must be in range"),
    }
}

fn adjust_selected_value(params: &mut EnvParams, selected: usize, delta: i32) -> bool {
    let (value, max) = selected_value_mut(params, selected);
    let next_value = (*value as i32 + delta).clamp(0, max as i32) as u32;
    let changed = *value != next_value;
    *value = next_value;
    changed
}

fn handle_key(key: KeyCode, selected: &mut usize, params: &mut EnvParams) -> AppCommand {
    match key {
        KeyCode::Char('q') => AppCommand::Quit,
        KeyCode::Up => {
            *selected = selected.saturating_sub(1);
            AppCommand::Continue
        }
        KeyCode::Down => {
            *selected = (*selected + 1).min(PARAMETER_COUNT - 1);
            AppCommand::Continue
        }
        KeyCode::Left => {
            if adjust_selected_value(params, *selected, -1) {
                AppCommand::RefreshImage
            } else {
                AppCommand::Continue
            }
        }
        KeyCode::Right => {
            if adjust_selected_value(params, *selected, 1) {
                AppCommand::RefreshImage
            } else {
                AppCommand::Continue
            }
        }
        KeyCode::PageUp => {
            if adjust_selected_value(params, *selected, PAGE_STEP) {
                AppCommand::RefreshImage
            } else {
                AppCommand::Continue
            }
        }
        KeyCode::PageDown => {
            if adjust_selected_value(params, *selected, -PAGE_STEP) {
                AppCommand::RefreshImage
            } else {
                AppCommand::Continue
            }
        }
        _ => AppCommand::Continue,
    }
}

fn parameter_lines(params: &EnvParams, selected: usize) -> Vec<Line<'static>> {
    let values = [params.ar, params.d1r, params.d1l, params.d2r, params.rr];
    PARAM_SPECS
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let prefix = if index == selected { ">" } else { " " };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(
                format!("{prefix} {:<3} {:>2} / {:>2}", spec.label, values[index], spec.max),
                style,
            ))
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut params = parse_args(&args);

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let mut protocol = render_protocol(&picker, &params)?;
    let mut selected = 0usize;

    let _raw_mode_guard = RawModeGuard::acquire()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("YM2151 Envelope (ratatui-image) - ↑↓ select, ←→ ±1, PgUp/PgDn ±4, q quit");
            let inner = block.inner(area).inner(Margin {
                vertical: 1,
                horizontal: 1,
            });

            let [params_area, graph_area] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(20), Constraint::Min(10)])
                .areas(inner);

            let params_block = Block::default().borders(Borders::ALL).title("Parameters");
            let params_inner = params_block.inner(params_area).inner(Margin {
                vertical: 1,
                horizontal: 1,
            });
            let graph_block = Block::default().borders(Borders::ALL).title("Live Graph");
            let graph_inner = graph_block.inner(graph_area).inner(Margin {
                vertical: 1,
                horizontal: 1,
            });

            f.render_widget(block, area);
            f.render_widget(params_block, params_area);
            f.render_widget(Paragraph::new(parameter_lines(&params, selected)), params_inner);
            f.render_widget(graph_block, graph_area);
            f.render_stateful_widget(StatefulImage::default(), graph_inner, &mut protocol);
        })?;

        if let Event::Key(key) = event::read()? {
            match handle_key(key.code, &mut selected, &mut params) {
                AppCommand::Continue => {}
                AppCommand::RefreshImage => {
                    protocol = render_protocol(&picker, &params)?;
                }
                AppCommand::Quit => break,
            }
        }
    }

    // Surface any image encoding error if it happened.
    if let Some(result) = protocol.last_encoding_result() {
        result?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_render_args_round_trips_through_parse_args() {
        let params = EnvParams {
            ar: 31,
            d1r: 24,
            d1l: 10,
            d2r: 7,
            rr: 15,
        };

        let parsed = parse_args(&build_render_args(&params));

        assert_eq!(parsed.ar, params.ar);
        assert_eq!(parsed.d1r, params.d1r);
        assert_eq!(parsed.d1l, params.d1l);
        assert_eq!(parsed.d2r, params.d2r);
        assert_eq!(parsed.rr, params.rr);
    }

    #[test]
    fn adjust_selected_value_clamps_to_field_range() {
        let mut params = EnvParams::default();

        assert!(adjust_selected_value(&mut params, 0, 100));
        assert_eq!(params.ar, PARAM_SPECS[0].max);

        assert!(adjust_selected_value(&mut params, 4, -100));
        assert_eq!(params.rr, 0);
    }

    #[test]
    fn handle_key_updates_selection_and_page_steps() {
        let mut params = EnvParams::default();
        let mut selected = 0usize;

        assert!(matches!(
            handle_key(KeyCode::Down, &mut selected, &mut params),
            AppCommand::Continue
        ));
        assert_eq!(selected, 1);

        assert!(matches!(
            handle_key(KeyCode::PageUp, &mut selected, &mut params),
            AppCommand::RefreshImage
        ));
        assert_eq!(params.d1r, EnvParams::default().d1r + PAGE_STEP as u32);

        assert!(matches!(
            handle_key(KeyCode::PageDown, &mut selected, &mut params),
            AppCommand::RefreshImage
        ));
        assert_eq!(params.d1r, EnvParams::default().d1r);
    }
}
