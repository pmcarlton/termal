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

pub struct SeqPane<'a> {
    pub sequences: &'a [String],
    pub ordering: &'a [usize],
    pub top_i: usize,
    pub left_j: usize,
    pub style_lut: &'a [Style],
    pub highlights: &'a [SearchHighlight<'a>],
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
            let highlight_color = |col: usize| highlight_color(&self.highlights, seq_index, col);

            for c in 0..cols {
                let j = self.left_j + c;
                if j >= seq.len() {
                    break;
                }
                let b = seq[j];
                let mut style = self.style_lut[b as usize];
                if let Some(color) = highlight_color(j) {
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
            let highlight_color = |col: usize| highlight_color(&self.highlights, seq_index, col);

            for c in 0..max_c {
                let j = self.retained_cols[c];
                // should never happen
                if j >= seq_bytes.len() {
                    panic!();
                }

                let b = seq_bytes[j];
                let mut style = self.style_lut[b as usize];
                if let Some(color) = highlight_color(j) {
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
    seq_index: usize,
    col: usize,
) -> Option<Color> {
    highlights.iter().find_map(|highlight| {
        highlight
            .spans_by_seq
            .get(seq_index)
            .and_then(|spans| in_spans(spans, col).then_some(highlight.color))
    })
}
