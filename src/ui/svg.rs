// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Peter Carlton

use std::{fs, path::Path};

use ratatui::{
    backend::TestBackend,
    buffer::{Buffer, Cell},
    layout::{Constraint, Direction, Layout},
    prelude::{Position, Rect, Terminal},
    style::Color,
    TerminalOptions, Viewport,
};

use crate::errors::TermalError;
use crate::ui::{render::render_ui, BottomPanePosition, UI};

const FONT_SIZE: u16 = 14;
const CELL_WIDTH: u16 = 8;
const CELL_HEIGHT: u16 = 16;

pub fn export_current_view(ui: &mut UI, path: &Path) -> Result<(), TermalError> {
    let size = ui
        .frame_size()
        .ok_or_else(|| TermalError::Format(String::from("No frame size yet")))?;
    let backend = TestBackend::new(size.width, size.height);
    let viewport = Viewport::Fixed(Rect::new(0, 0, size.width, size.height));
    let mut terminal = Terminal::with_options(backend, TerminalOptions { viewport })
        .map_err(|e| TermalError::Format(format!("SVG backend error: {}", e)))?;
    terminal
        .draw(|f| render_ui(f, ui))
        .map_err(|e| TermalError::Format(format!("SVG render error: {}", e)))?;
    let buffer = terminal.backend().buffer().clone();
    let seq_rect = sequence_pane_rect(ui, Rect::new(0, 0, size.width, size.height));
    let svg = buffer_to_svg(&buffer, seq_rect);
    fs::write(path, svg)?;
    Ok(())
}

fn buffer_to_svg(buf: &Buffer, seq_rect: Rect) -> String {
    let area = buf.area;
    let width_px = area.width.saturating_mul(CELL_WIDTH) as u32;
    let height_px = area.height.saturating_mul(CELL_HEIGHT) as u32;
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width_px, height_px, width_px, height_px
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n");
    out.push_str(&format!(
        "<g font-family=\"monospace\" font-size=\"{}\" dominant-baseline=\"hanging\">\n",
        FONT_SIZE
    ));

    for y in 0..area.height {
        for x in 0..area.width {
            let cell = buf.cell(Position::from((x, y))).expect("buffer position");
            let ch = cell.symbol().chars().next().unwrap_or(' ');
            if ch == ' ' {
                continue;
            }
            let (r, g, b, bold) = text_color(cell, seq_rect, x, y);
            let color = format!("#{:02x}{:02x}{:02x}", r, g, b);
            let x_px = (x * CELL_WIDTH) as u32;
            let y_px = (y * CELL_HEIGHT) as u32;
            let weight = if bold { " font-weight=\"bold\"" } else { "" };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" fill=\"{}\"{}>{}</text>\n",
                x_px,
                y_px,
                color,
                weight,
                escape_svg_char(ch)
            ));
        }
    }

    out.push_str("</g>\n</svg>\n");
    out
}

fn text_color(cell: &Cell, seq_rect: Rect, x: u16, y: u16) -> (u8, u8, u8, bool) {
    let highlight = match color_to_rgb(cell.bg) {
        Some((0, 0, 0)) => None,
        other => other,
    };
    let bold = highlight.is_some() && is_within(seq_rect, x, y);
    if let Some((r, g, b)) = highlight {
        return (r, g, b, bold);
    }
    (0, 0, 0, false)
}

fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Black => Some((0, 0, 0)),
        Color::White => Some((255, 255, 255)),
        Color::Gray => Some((128, 128, 128)),
        Color::DarkGray => Some((64, 64, 64)),
        Color::LightRed => Some((255, 128, 128)),
        Color::LightGreen => Some((128, 255, 128)),
        Color::LightBlue => Some((128, 128, 255)),
        Color::LightYellow => Some((255, 255, 128)),
        Color::LightMagenta => Some((255, 128, 255)),
        Color::LightCyan => Some((128, 255, 255)),
        Color::Red => Some((255, 0, 0)),
        Color::Green => Some((0, 255, 0)),
        Color::Blue => Some((0, 0, 255)),
        Color::Yellow => Some((255, 255, 0)),
        Color::Magenta => Some((255, 0, 255)),
        Color::Cyan => Some((0, 255, 255)),
        _ => None,
    }
}

fn escape_svg_char(ch: char) -> String {
    match ch {
        '&' => String::from("&amp;"),
        '<' => String::from("&lt;"),
        '>' => String::from("&gt;"),
        '"' => String::from("&quot;"),
        '\'' => String::from("&#39;"),
        _ => ch.to_string(),
    }
}

fn is_within(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn max_num_seq(area: Rect, ui: &UI) -> u16 {
    match ui.zoom_level {
        super::ZoomLevel::ZoomedOut | super::ZoomLevel::ZoomedIn => ui.app.num_seq(),
        super::ZoomLevel::ZoomedOutAR => {
            let v_constraints = vec![Constraint::Fill(1), Constraint::Max(ui.bottom_pane_height)];
            let top_chunk = Layout::new(Direction::Vertical, v_constraints).split(area)[0];
            let aln_pane = Layout::new(
                Direction::Horizontal,
                vec![Constraint::Max(ui.left_pane_width), Constraint::Fill(1)],
            )
            .split(top_chunk)[1];

            let v_ratio = (aln_pane.height - 2) as f64 / ui.app.num_seq() as f64;
            let h_ratio = (aln_pane.width - 2) as f64 / ui.app.aln_len() as f64;
            let ratio = h_ratio.min(v_ratio);

            (ui.app.num_seq() as f64 * ratio).round() as u16
        }
    }
}

fn sequence_pane_rect(ui: &UI, area: Rect) -> Rect {
    let mns = max_num_seq(area, ui);
    let constraints: Vec<Constraint> = match ui.bottom_pane_position {
        BottomPanePosition::Adjacent => vec![
            Constraint::Max(mns + 2),
            Constraint::Max(ui.bottom_pane_height),
        ],
        BottomPanePosition::ScreenBottom => {
            vec![Constraint::Fill(1), Constraint::Max(ui.bottom_pane_height)]
        }
    };
    let v_panes = Layout::new(Direction::Vertical, constraints).split(area);
    let min_seq_pane_width = super::V_SCROLLBAR_WIDTH + super::MIN_COLS_SHOWN + super::BORDER_WIDTH;
    let upper_panes = Layout::new(
        Direction::Horizontal,
        vec![
            Constraint::Max(ui.left_pane_width),
            Constraint::Min(min_seq_pane_width),
        ],
    )
    .split(v_panes[0]);
    upper_panes[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, prelude::Rect, style::Style};

    #[test]
    fn svg_uses_bg_as_text_color() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        buf.cell_mut(Position::from((0, 0)))
            .expect("buffer position")
            .set_char('A')
            .set_style(Style::default().bg(Color::Rgb(10, 20, 30)));
        let svg = buffer_to_svg(&buf, Rect::new(0, 0, 1, 1));
        assert!(svg.contains("fill=\"#0a141e\""));
    }

    #[test]
    fn svg_bolds_sequence_highlights() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        buf.cell_mut(Position::from((0, 0)))
            .expect("buffer position")
            .set_char('A')
            .set_style(Style::default().bg(Color::Rgb(10, 20, 30)));
        let svg = buffer_to_svg(&buf, Rect::new(0, 0, 1, 1));
        assert!(svg.contains("font-weight=\"bold\""));
    }
}
