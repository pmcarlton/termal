use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Position,
    style::Style
};

use log::debug;

/*
pub fn draw_zoombox_border(
    buf: &mut Buffer,
    area: Rect,      // the pane area on screen
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
    style: Style,
) {

    let x0 = area.x + left as u16;
    let x1 = area.x + (right - 1) as u16;   // inclusive last col
    let y0 = area.y + top as u16;
    let y1 = area.y + (bottom - 1) as u16;  // inclusive last row

    if right > left+1 && bottom > top+1 {
        draw_zoombox_border_general_case(buf, x0, x1, y0, y1, style);
    } else if right <= left+1 && bottom <= top+1 {
        draw_zoombox_border_point(buf, x0, y0, style);
    } else if right <= left+1 {
        draw_zoombox_border_zero_width(buf, x0, y0, y1, style);
    }
}
*/

pub fn draw_zoombox_border(
    buf: &mut Buffer,
    area: Rect,
    zb_top: usize,
    zb_bottom: usize, // exclusive
    zb_left: usize,
    zb_right: usize, // exclusive
    style: Style,
) {
    let pane_h = area.height as usize;
    let pane_w = area.width as usize;
    if pane_h == 0 || pane_w == 0 {
        return;
    }

    // Clamp to pane bounds (exclusive max)
    /*
    let zb_top = zb_top.min(pane_h - 1);
    let zb_left = zb_left.min(pane_w - 1);
    let zb_bottom = zb_bottom.min(pane_h);
    let zb_right = zb_right.min(pane_w);
    */

    let w = zb_right.saturating_sub(zb_left); // in cells
    let h = zb_bottom.saturating_sub(zb_top); // in cells

    let x0 = area.x + zb_left as u16;
    let y0 = area.y + zb_top as u16;
    let x1 = area.x + (zb_right.saturating_sub(1)) as u16;
    let y1 = area.y + (zb_bottom.saturating_sub(1)) as u16;

    // 1x1 (or degenerate) => point marker
    if w <= 1 && h <= 1 {
        draw_zoombox_border_point(buf, x0, y0, style);
        return;
    }

    // single column
    if w <= 1 {
        draw_zoombox_border_zero_width(buf, x0, y0, y1, style);
        return;
    }

    // single row
    if h <= 1 {
       draw_zoombox_border_zero_height(buf, x0, x1, y0, style);
       return;
    }

    // general case (>= 2x2)
    draw_zoombox_border_general_case(buf, x0, x1, y0, y1, style);
}

fn draw_zoombox_border_general_case(
    buf: &mut Buffer,
    zb_left: u16,
    zb_right: u16,
    zb_top: u16,
    zb_bottom: u16,
    style: Style,
) {
    // Top edge
    buf.cell_mut(Position::from((zb_left, zb_top))).expect("Wrong position").set_char('┌').set_style(style);
    for x in (zb_left + 1)..zb_right {
        buf.cell_mut(Position::from((x, zb_top))).expect("Wrong position").set_char('─').set_style(style);
    }
    buf.cell_mut(Position::from((zb_right, zb_top))).expect("Wrong position").set_char('┐').set_style(style);

    // Sides
    for y in (zb_top + 1)..zb_bottom {
        buf.cell_mut(Position::from((zb_left, y))).expect("Wrong position").set_char('│').set_style(style);
        buf.cell_mut(Position::from((zb_right, y))).expect("Wrong position").set_char('│').set_style(style);
    }

    // Bottom edge
    buf.cell_mut(Position::from((zb_left, zb_bottom))).expect("Wrong position")
        .set_char('└')
        .set_style(style);
    for x in (zb_left + 1)..zb_right {
        buf.cell_mut(Position::from((x, zb_bottom))).expect("Wrong position").set_char('─').set_style(style);
    }
    buf.cell_mut(Position::from((zb_right, zb_bottom))).expect("Wrong position")
        .set_char('┘')
        .set_style(style);
}

fn draw_zoombox_border_point(buf: &mut Buffer, zb_left: u16, zb_top: u16, style: Style) {
    buf.cell_mut(Position::from((zb_left, zb_top))).expect("Wrong position").set_char('▯').set_style(style);
}

fn draw_zoombox_border_zero_width(
    buf: &mut Buffer,
    zb_left: u16, // zb_right == zb_left
    zb_top: u16,
    zb_bottom: u16,
    style: Style,
) {
    // Top cell
    buf.cell_mut(Position::from((zb_left, zb_top))).expect("Wrong position").set_char('╿').set_style(style);
    // Inner cells
    for y in (zb_top + 1)..zb_bottom {
        buf.cell_mut(Position::from((zb_left, y))).expect("Wrong position").set_char('│').set_style(style);
    }
    // Bottom cell
    buf.cell_mut(Position::from((zb_left, zb_bottom))).expect("Wrong position") .set_char('╽').set_style(style);
}


fn draw_zoombox_border_zero_height(
    buf: &mut Buffer,
    zb_left: u16, 
    zb_right: u16,
    zb_top: u16,    // zb_bottom = zb_top
    style: Style,
) {
    // Leftmost col
    buf.cell_mut(Position::from((zb_left, zb_top))).expect("Wrong position").set_char('╾').set_style(style);
    // Inner cells
    for x in (zb_left + 1)..zb_right {
        buf.cell_mut(Position::from((x, zb_top))).expect("Wrong position").set_char('─').set_style(style);
    }
    // Bottom edge
    buf.cell_mut(Position::from((zb_right, zb_top))).expect("Wrong position").set_char('╼').set_style(style);
}

//
// fn mark_zoombox_zero_height(
//     seq_para: &mut [Line],
//     zb_top: usize, // zb_bottom == zb_top
//     zb_left: usize,
//     zb_right: usize,
//     zb_style: Style,
// ) {
//     let l: &mut Line = &mut seq_para[zb_top];
//     let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("╾", zb_style));
//     for c in zb_left + 1..zb_right {
//         let _ = std::mem::replace(&mut l.spans[c], Span::styled("─", zb_style));
//     }
//     let _ = std::mem::replace(&mut l.spans[zb_right - 1], Span::styled("╼", zb_style));
// }
//

// // Auxiliary fn for mark_zoombox() - _could_ use an internal fn or a closure, but that would make
// // the function too long for my taste.
// //
// fn mark_zoombox_general_case(
//     seq_para: &mut [Line],
//     zb_top: usize,
//     zb_bottom: usize,
//     zb_left: usize,
//     zb_right: usize,
//     zb_style: Style,
// ) {
//     let mut l: &mut Line = &mut seq_para[zb_top];
//     for c in zb_left + 1..zb_right {
//         let _ = std::mem::replace(&mut l.spans[c], Span::styled("─", zb_style));
//     }
//     let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("┌", zb_style));
//     let _ = std::mem::replace(&mut l.spans[zb_right - 1], Span::styled("┐", zb_style));
//
//     // NOTE: Clippy suggests using an iterator here, but if I want, say, residues 600-680, then
//     // there are going to be 600 useless iterations. I imagine indexing is faster, though
//     // admittedly I did not benchmark it... except with my eye-o-meter, which indeed did not detect
//     // any difference on a 11th Gen Intel(R) Core(TM) i7-11850H @ 2.50GHz machine running WSL2, and
//     // a 144-column by 33-lines terminal.
//
//     // mine
//     /*
//     for s in zb_top+1 .. zb_bottom {
//         l = &mut seq_para[s];
//         let _ = std::mem::replace(&mut l.spans[zb_left], Span::raw("│"));
//         let _ = std::mem::replace(&mut l.spans[zb_right-1], Span::raw("│"));
//     }
//     */
//
//     // Clippy
//     // /*
//     for l in seq_para.iter_mut().take(zb_bottom).skip(zb_top + 1) {
//         // let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("│", zb_style));
//         let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("│", zb_style));
//         let _ = std::mem::replace(&mut l.spans[zb_right - 1], Span::styled("│", zb_style));
//     }
//     //*/
//     l = &mut seq_para[zb_bottom - 1];
//     //FIXME: it should not be necessary to iterate _twice_ from zb_left+1 to zb_right
//     for c in zb_left + 1..zb_right {
//         let _ = std::mem::replace(&mut l.spans[c], Span::styled("─", zb_style));
//     }
//     let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("└", zb_style));
//     let _ = std::mem::replace(&mut l.spans[zb_right - 1], Span::styled("┘", zb_style));
// }
//
// // Auxiliary fn for mark_zoombox() - see remarks on previous fn.
// // Auxiliary fn for mark_zoombox() - see remarks on previous fn.
//
// fn mark_zoombox_zero_width(
//     seq_para: &mut [Line],
//     zb_top: usize,
//     zb_bottom: usize,
//     zb_left: usize, // zb_right == zb_left
//     zb_style: Style,
// ) {
//     let mut l: &mut Line = &mut seq_para[zb_top];
//     let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("╿", zb_style));
//
//     for l in seq_para.iter_mut().take(zb_bottom).skip(zb_top + 1) {
//         let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("│", zb_style));
//     }
//
//     l = &mut seq_para[zb_bottom - 1];
//     let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("╽", zb_style));
// }
//
// // Auxiliary fn for mark_zoombox() - see remarks on previous fn.
// //
// fn mark_zoombox_point(
//     seq_para: &mut [Line],
//     zb_top: usize,
//     zb_left: usize, // zb_bottom == zb_top, zb_right == zb_left
//     zb_style: Style,
// ) {
//     let l: &mut Line = &mut seq_para[zb_top];
//     let _ = std::mem::replace(&mut l.spans[zb_left], Span::styled("▯", zb_style));
// }
//
// // Draws the zoombox (just overwrites the sequence area with box-drawing characters).
// //
// fn mark_zoombox(seq_para: &mut [Line], ui: &UI) {
//     // I want zb_top to be immutable, but I may need to change it just after intialization
//     let zb_top = ui.zoombox_top();
//     let zb_bottom = ui.zoombox_bottom(seq_para.len());
//     let zb_left = ui.zoombox_left();
//     let zb_right = ui.zoombox_right(seq_para[0].spans.len());
//     /*
//     let mut zb_right: usize =
//         (((ui.leftmost_col + ui.max_nb_col_shown()) as f64) * ui.h_ratio()).round() as usize;
//     // If w_a < w_p
//     if zb_right > ui.app.aln_len() as usize {
//         zb_right = ui.app.aln_len() as usize;
//     }
//     ui.assert_invariants();
//     */
//
//     let zoombox_color = ui.get_zoombox_color();
//     let zb_style = Style::new().fg(zoombox_color);
//
//     if zb_bottom - zb_top < 2 {
//         if zb_right - zb_left < 2 {
//             // Zoom box is on a single line & column
//             mark_zoombox_point(seq_para, zb_top, zb_left, zb_style);
//         } else {
//             // Zoom box has a height of 1 line
//             mark_zoombox_zero_height(seq_para, zb_top, zb_left, zb_right, zb_style);
//         }
//     } else if zb_right - zb_left < 2 {
//         // Zoom box has a width of 1 column
//         mark_zoombox_zero_width(seq_para, zb_top, zb_bottom, zb_left, zb_style);
//     } else {
//         // General case: height and width both > 1
//         mark_zoombox_general_case(seq_para, zb_top, zb_bottom, zb_left, zb_right, zb_style);
//     }
// }
