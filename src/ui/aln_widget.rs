// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use ratatui::{
    prelude::{Buffer, Position, Rect},
    style::{Color, Style},
    widgets::Widget,
};

use crate::ui::zoombox::draw_zoombox_border;

pub struct SearchHighlight<'a> {
    pub spans_by_seq: &'a [Vec<(usize, usize)>],
    pub color: Color,
}

pub struct SearchHighlightConfig {
    pub min_component: u8,
    pub gap_dim_factor: f32,
}

pub struct SeqPane<'a> {
    pub sequences: &'a [String],
    pub ordering: &'a [usize],
    pub top_i: usize,
    pub left_j: usize,
    pub style_lut: &'a [Style],
    pub highlights: &'a [SearchHighlight<'a>],
    pub highlight_config: SearchHighlightConfig,
    // TODO: not sure this is required - if not, also remove from other SeqPane* structs
    pub base_style: Style, // optional, for clearing/background
}

impl<'a> Widget for SeqPane<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = area.height as usize;
        let cols = area.width as usize;

        // Clear the pane so “extra space” doesn’t show stale cells.
        for y in 0..rows {
            for x in 0..cols {
                buf.cell_mut(Position::from((area.x + x as u16, area.y + y as u16)))
                    .expect("Wrong position")
                    .set_char(' ')
                    .set_style(self.base_style);
            }
        }

        for r in 0..rows {
            let i = self.top_i + r;
            if i >= self.ordering.len() {
                break;
            }
            let seq_index = self.ordering[i];
            let seq = self.sequences[seq_index].as_bytes();
            let highlight_color = |col: usize, ch: char| {
                highlight_color(&self.highlights, &self.highlight_config, seq_index, col, ch)
            };

            for c in 0..cols {
                let j = self.left_j + c;
                if j >= seq.len() {
                    break;
                }
                let b = seq[j];
                let mut style = self.style_lut[b as usize];
                if let Some(color) = highlight_color(j, b as char) {
                    style = style.bg(color);
                }

                buf.cell_mut(Position::from((area.x + c as u16, area.y + r as u16)))
                    .expect("Wrong position")
                    .set_char(b as char)
                    .set_style(style);
            }
        }
    }
}

pub struct SeqPaneZoomedOut<'a> {
    pub sequences: &'a [String],    // alignment.sequences
    pub ordering: &'a [usize],      // ordering map
    pub retained_rows: &'a [usize], // indices into "logical rows"
    pub retained_cols: &'a [usize], // indices into alignment columns
    pub style_lut: &'a [Style],     // style per byte (0..=255)
    pub highlights: &'a [SearchHighlight<'a>],
    pub highlight_config: SearchHighlightConfig,
    pub base_style: Style, // for clearing/background
    pub show_zoombox: bool,
    pub zb_top: usize,
    pub zb_bottom: usize,
    pub zb_left: usize,
    pub zb_right: usize,
    pub zb_style: Style,
}

impl<'a> Widget for SeqPaneZoomedOut<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = area.height as usize;
        let cols = area.width as usize;

        // Clear pane (see ZoomedIn mode)
        for y in 0..rows {
            for x in 0..cols {
                buf.cell_mut(Position::from((area.x + x as u16, area.y + y as u16)))
                    .expect("Wrong position")
                    .set_char(' ')
                    .set_style(self.base_style);
            }
        }

        // Render sampled rows/cols
        let max_r = rows.min(self.retained_rows.len());
        let max_c = cols.min(self.retained_cols.len());

        for r in 0..max_r {
            let i = self.retained_rows[r];
            // should never happen
            if i >= self.ordering.len() {
                panic!();
            }

            let seq_index = self.ordering[i];
            let seq_bytes = self.sequences[seq_index].as_bytes();
            let highlight_color = |col: usize, ch: char| {
                highlight_color(&self.highlights, &self.highlight_config, seq_index, col, ch)
            };

            for c in 0..max_c {
                let j = self.retained_cols[c];
                // should never happen
                if j >= seq_bytes.len() {
                    panic!();
                }

                let b = seq_bytes[j];
                let mut style = self.style_lut[b as usize];
                if let Some(color) = highlight_color(j, b as char) {
                    style = style.bg(color);
                }

                buf.cell_mut(Position::from((area.x + c as u16, area.y + r as u16)))
                    .expect("Wrong position")
                    .set_char(b as char)
                    .set_style(style);
            }
        }

        if self.show_zoombox {
            draw_zoombox_border(
                buf,
                area,
                self.zb_top,
                self.zb_bottom,
                self.zb_left,
                self.zb_right,
                self.zb_style,
            );
        }
    }
}

fn in_spans(spans: &[(usize, usize)], col: usize) -> bool {
    spans.iter().any(|(start, end)| *start <= col && col < *end)
}

fn highlight_color(
    highlights: &[SearchHighlight<'_>],
    config: &SearchHighlightConfig,
    seq_index: usize,
    col: usize,
    ch: char,
) -> Option<Color> {
    let colors: Vec<(u8, u8, u8)> = highlights
        .iter()
        .filter_map(|highlight| {
            highlight
                .spans_by_seq
                .get(seq_index)
                .and_then(|spans| in_spans(spans, col).then_some(highlight.color))
        })
        .filter_map(color_to_rgb)
        .collect();
    if colors.is_empty() {
        return None;
    }
    let (mut r, mut g, mut b) = blend_colors(&colors);
    normalize_min_component(&mut r, &mut g, &mut b, config.min_component);
    if is_gap(ch) {
        dim_color(&mut r, &mut g, &mut b, config.gap_dim_factor);
    }
    Some(Color::Rgb(r, g, b))
}

fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        _ => None,
    }
}

fn blend_colors(colors: &[(u8, u8, u8)]) -> (u8, u8, u8) {
    let count = colors.len() as f32;
    let (sum_r, sum_g, sum_b) = colors.iter().fold((0u32, 0u32, 0u32), |acc, color| {
        (
            acc.0 + color.0 as u32,
            acc.1 + color.1 as u32,
            acc.2 + color.2 as u32,
        )
    });
    let r = (sum_r as f32 / count).round() as u8;
    let g = (sum_g as f32 / count).round() as u8;
    let b = (sum_b as f32 / count).round() as u8;
    (r, g, b)
}

fn normalize_min_component(r: &mut u8, g: &mut u8, b: &mut u8, min_component: u8) {
    let min_val = (*r).min(*g).min(*b);
    if min_val >= min_component {
        return;
    }
    let delta = min_component.saturating_sub(min_val) as u16;
    *r = (*r as u16 + delta).min(u8::MAX as u16) as u8;
    *g = (*g as u16 + delta).min(u8::MAX as u16) as u8;
    *b = (*b as u16 + delta).min(u8::MAX as u16) as u8;
}

fn dim_color(r: &mut u8, g: &mut u8, b: &mut u8, factor: f32) {
    *r = ((*r as f32) * factor).round().min(u8::MAX as f32) as u8;
    *g = ((*g as f32) * factor).round().min(u8::MAX as f32) as u8;
    *b = ((*b as f32) * factor).round().min(u8::MAX as f32) as u8;
}

fn is_gap(c: char) -> bool {
    matches!(c, '-' | '.' | ' ')
}

#[cfg(test)]
mod tests {
    use super::{blend_colors, dim_color, normalize_min_component};

    #[test]
    fn blend_and_normalize() {
        let colors = vec![(100, 0, 0), (0, 100, 0)];
        let (mut r, mut g, mut b) = blend_colors(&colors);
        normalize_min_component(&mut r, &mut g, &mut b, 100);
        assert_eq!((r, g, b), (150, 150, 100));
    }

    #[test]
    fn dim_gap_color() {
        let mut r = 100;
        let mut g = 80;
        let mut b = 60;
        dim_color(&mut r, &mut g, &mut b, 0.5);
        assert_eq!((r, g, b), (50, 40, 30));
    }
}
