use ratatui::{
    prelude::{
        Buffer,
        Rect,
    },
    style::Style,
    widgets::Widget
};

use crate::ui::{
    color_map::ColorMap,
    render::get_residue_style,
    Theme,
    VideoMode,
};

pub struct SeqPane<'a> {
    pub sequences: &'a [String],
    pub ordering: &'a [usize],
    pub top_i: usize,
    pub left_j: usize,
    pub video_mode: VideoMode,
    pub theme: &'a Theme,
    pub colormap: &'a ColorMap,
    pub base_style: Style, // optional, for clearing/background
}

impl<'a> SeqPane<'a> {
    #[inline]
    fn style_for_byte(&self, b: u8) -> Style {
        // assuming ASCII residues
        let ch = b as char;
        get_residue_style(self.video_mode, *self.theme, self.colormap.get(ch))
    }
}

impl<'a> Widget for SeqPane<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = area.height as usize;
        let cols = area.width as usize;

        // Clear the pane so “extra space” doesn’t show stale cells.
        for y in 0..rows {
            for x in 0..cols {
                buf.get_mut(area.x + x as u16, area.y + y as u16)
                    .set_char(' ')
                    .set_style(self.base_style);
            }
        }

        for r in 0..rows {
            let i = self.top_i + r;
            if i >= self.ordering.len() {
                break;
            }
            let seq = self.sequences[self.ordering[i]].as_bytes();

            for c in 0..cols {
                let j = self.left_j + c;
                if j >= seq.len() {
                    break;
                }
                let b = seq[j];
                let style = self.style_for_byte(b);

                buf.get_mut(area.x + c as u16, area.y + r as u16)
                    .set_char(b as char)
                    .set_style(style);
            }
        }
    }
}

