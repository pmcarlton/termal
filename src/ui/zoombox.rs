use ratatui::{buffer::Buffer, layout::Rect, style::Style};

// Draw a single-line border rectangle (┌─┐ │ │ └─┘) into `buf`.
//
// Coordinates are *pane-local* (0-based) and `right`/`bottom` are *exclusive*:
// - left .. right   in columns
// - top  .. bottom  in rows
//
// So the rectangle covers rows [top, bottom) and cols [left, right),
// and the border is drawn on the perimeter cells.
//
// Requirements:
// - right >= left + 2
// - bottom >= top + 2
pub fn draw_zoombox_border(
    buf: &mut Buffer,
    area: Rect,      // the pane area on screen
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
    style: Style,
) {
    // Quick rejects / clamps to pane
    let pane_h = area.height as usize;
    let pane_w = area.width as usize;

    if right <= left + 1 || bottom <= top + 1 {
        // TODO: special cases
        return; // too small to draw a box
    }
    if top >= pane_h || left >= pane_w {
        // TODO: should perhaps panic, as this should never happen
        return;
    }

    let bottom = bottom.min(pane_h);
    let right = right.min(pane_w);

    if right <= left + 1 || bottom <= top + 1 {
        return;
    }

    let x0 = area.x + left as u16;
    let x1 = area.x + (right - 1) as u16;   // inclusive last col
    let y0 = area.y + top as u16;
    let y1 = area.y + (bottom - 1) as u16;  // inclusive last row

    // Top edge
    buf.get_mut(x0, y0).set_char('┌').set_style(style);
    for x in (x0 + 1)..x1 {
        buf.get_mut(x, y0).set_char('─').set_style(style);
    }
    buf.get_mut(x1, y0).set_char('┐').set_style(style);

    // Sides
    for y in (y0 + 1)..y1 {
        buf.get_mut(x0, y).set_char('│').set_style(style);
        buf.get_mut(x1, y).set_char('│').set_style(style);
    }

    // Bottom edge
    buf.get_mut(x0, y1).set_char('└').set_style(style);
    for x in (x0 + 1)..x1 {
        buf.get_mut(x, y1).set_char('─').set_style(style);
    }
    buf.get_mut(x1, y1).set_char('┘').set_style(style);
}


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
