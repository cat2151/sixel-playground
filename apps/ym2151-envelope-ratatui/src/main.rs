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
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use ym2151_envelope_core::{parse_args, EnvParams};
use ym2151_ratatui_image::render_ym2151_as_image_from_args_with_size;

const PARAMETER_COUNT: usize = 5;
const PAGE_STEP: i32 = 4;
const GRAPH_RENDER_SCALE_X: u32 = 4;
const GRAPH_RENDER_SCALE_Y: u32 = 8;
const MIN_GRAPH_RENDER_WIDTH: u32 = 64;
const MIN_GRAPH_RENDER_HEIGHT: u32 = 32;

struct ParamSpec {
    label: &'static str,
    max: u32,
}

#[derive(Clone, Copy)]
struct UiLayout {
    params_area: Rect,
    params_inner: Rect,
    graph_area: Rect,
    graph_inner: Rect,
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

fn ui_layout(area: Rect) -> UiLayout {
    let block_inner = Block::default()
        .borders(Borders::ALL)
        .inner(area)
        .inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
    let [params_area, graph_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(10)])
        .areas(block_inner);
    let params_inner = Block::default()
        .borders(Borders::ALL)
        .inner(params_area)
        .inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
    let graph_inner = Block::default()
        .borders(Borders::ALL)
        .inner(graph_area)
        .inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
    UiLayout {
        params_area,
        params_inner,
        graph_area,
        graph_inner,
    }
}

fn render_size_for_graph_area(area: Rect) -> (u32, u32) {
    let width = (u32::from(area.width) * GRAPH_RENDER_SCALE_X).max(MIN_GRAPH_RENDER_WIDTH);
    let height = (u32::from(area.height) * GRAPH_RENDER_SCALE_Y).max(MIN_GRAPH_RENDER_HEIGHT);
    (width, height)
}

fn render_protocol(
    picker: &Picker,
    params: &EnvParams,
    render_size: (u32, u32),
) -> Result<StatefulProtocol, Box<dyn std::error::Error>> {
    let (width, height) = render_size;
    let image = render_ym2151_as_image_from_args_with_size(&build_render_args(params), width, height)?;
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

    let _raw_mode_guard = RawModeGuard::acquire()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut current_render_size =
        render_size_for_graph_area(ui_layout(terminal.size()?.into()).graph_inner);
    let mut protocol = render_protocol(&picker, &params, current_render_size)?;
    let mut selected = 0usize;

    loop {
        let layout = ui_layout(terminal.size()?.into());
        terminal.draw(|f| {
            let area = f.area();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("YM2151 Envelope (ratatui-image) - ↑↓ select, ←→ ±1, PgUp/PgDn ±4, q quit");
            let params_block = Block::default().borders(Borders::ALL).title("Parameters");
            let graph_block = Block::default().borders(Borders::ALL).title("Live Graph");

            f.render_widget(block, area);
            f.render_widget(params_block, layout.params_area);
            f.render_widget(
                Paragraph::new(parameter_lines(&params, selected)),
                layout.params_inner,
            );
            f.render_widget(graph_block, layout.graph_area);
            f.render_stateful_widget(StatefulImage::default(), layout.graph_inner, &mut protocol);
        })?;

        match event::read()? {
            Event::Key(key) => match handle_key(key.code, &mut selected, &mut params) {
                AppCommand::Continue => {}
                AppCommand::RefreshImage => {
                    current_render_size = render_size_for_graph_area(layout.graph_inner);
                    protocol = render_protocol(&picker, &params, current_render_size)?;
                }
                AppCommand::Quit => break,
            },
            Event::Resize(_, _) => {
                let resized_graph_inner = ui_layout(terminal.size()?.into()).graph_inner;
                let resized_render_size = render_size_for_graph_area(resized_graph_inner);
                if resized_render_size != current_render_size {
                    current_render_size = resized_render_size;
                    protocol = render_protocol(&picker, &params, current_render_size)?;
                }
            }
            _ => {}
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

    #[test]
    fn render_size_for_graph_area_scales_from_current_pane_size() {
        let graph_area = Rect::new(0, 0, 100, 20);

        assert_eq!(
            render_size_for_graph_area(graph_area),
            (100 * GRAPH_RENDER_SCALE_X, 20 * GRAPH_RENDER_SCALE_Y)
        );
    }

    #[test]
    fn render_size_for_graph_area_respects_minimums() {
        let graph_area = Rect::new(0, 0, 1, 1);

        assert_eq!(
            render_size_for_graph_area(graph_area),
            (MIN_GRAPH_RENDER_WIDTH, MIN_GRAPH_RENDER_HEIGHT)
        );
    }
}
