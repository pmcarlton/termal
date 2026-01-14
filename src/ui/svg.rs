// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::{fs, path::Path};

use ratatui::{
    backend::TestBackend,
    buffer::{Buffer, Cell},
    prelude::{Position, Rect, Terminal},
    style::Color,
    TerminalOptions, Viewport,
};

use crate::{errors::TermalError, ui::render::render_ui, ui::UI};

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
    let svg = buffer_to_svg(&buffer);
    fs::write(path, svg)?;
    Ok(())
}

fn buffer_to_svg(buf: &Buffer) -> String {
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
            let (r, g, b) = text_color(cell);
            let color = format!("#{:02x}{:02x}{:02x}", r, g, b);
            let x_px = (x * CELL_WIDTH) as u32;
            let y_px = (y * CELL_HEIGHT) as u32;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>\n",
                x_px,
                y_px,
                color,
                escape_svg_char(ch)
            ));
        }
    }

    out.push_str("</g>\n</svg>\n");
    out
}

fn text_color(cell: &Cell) -> (u8, u8, u8) {
    if let Some((r, g, b)) = color_to_rgb(cell.bg) {
        if !(r == 0 && g == 0 && b == 0) {
            return (r, g, b);
        }
    }
    (0, 0, 0)
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
        let svg = buffer_to_svg(&buf);
        assert!(svg.contains("fill=\"#0a141e\""));
    }
}
