// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Thomas Junier
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    line_editor::LineEditor,
    InputMode,
    InputMode::{
        Command, ConfirmOverwrite, ConfirmReject, ExportSvg, Help, LabelSearch, Normal,
        PendingCount, Search, SearchList,
    },
    //SearchDirection,
    {RejectMode, ZoomLevel, UI},
};
use crate::app::SearchKind;
use std::collections::HashSet;

pub fn handle_key_press(ui: &mut UI, key_event: KeyEvent) -> bool {
    let mut done = false;
    let mode = ui.input_mode.clone();
    match mode {
        Normal => done = handle_normal_key(ui, key_event),
        Help => handle_help_key(ui, key_event),
        PendingCount { count } => done = handle_pending_count_key(ui, key_event, count),
        LabelSearch { pattern } => handle_label_search(ui, key_event, &pattern),
        Search { editor, kind } => handle_search(ui, key_event, editor, kind),
        Command { editor } => handle_command(ui, key_event, editor),
        ExportSvg { editor } => handle_export_svg(ui, key_event, editor),
        ConfirmOverwrite { editor, path } => handle_confirm_overwrite(ui, key_event, editor, path),
        SearchList { selected } => handle_search_list(ui, key_event, selected),
        ConfirmReject { mode } => handle_confirm_reject(ui, key_event, mode),
    };
    if ui.has_exit_message() {
        true
    } else {
        done
    }
}

fn handle_normal_key(ui: &mut UI, key_event: KeyEvent) -> bool {
    let mut done = false;
    match key_event.code {
        // 1-9: enter pending count mode
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let d = (c as u8 - b'0') as usize;
            ui.input_mode = InputMode::PendingCount { count: d };
            ui.app.clear_msg();
            ui.app.add_argument_char(c);
        }
        KeyCode::Esc => {
            ui.app.reset_lbl_search();
            ui.app.clear_msg();
        }
        // Q, q, and Ctrl-C quit
        KeyCode::Char('q') | KeyCode::Char('Q') => done = true,
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => done = true,
        // TODO: search
        KeyCode::Char('?') => {
            ui.reset_help_scroll();
            ui.input_mode = InputMode::Help;
        }
        KeyCode::Char('"') => {
            ui.input_mode = InputMode::LabelSearch {
                pattern: String::from(""),
            };
            ui.app
                .argument_msg(String::from("Label search: "), String::from(""));
        }
        KeyCode::Char(':') => {
            ui.input_mode = InputMode::Command {
                editor: LineEditor::new(),
            };
            ui.app.argument_msg(String::from(":"), String::from(""));
        }
        KeyCode::Char('/') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Regex,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
        }
        KeyCode::Char('\\') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Emboss,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
        }
        // Anything else: dispatch corresponding command, without count
        _ => dispatch_command(ui, key_event, None),
    }
    done
}

fn handle_help_key(ui: &mut UI, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('?') => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        KeyCode::Up | KeyCode::Char('k') => ui.help_scroll_by(-1),
        KeyCode::Down | KeyCode::Char('j') => ui.help_scroll_by(1),
        KeyCode::PageUp => ui.help_scroll_by(-(ui.help_page_height() as isize)),
        KeyCode::PageDown | KeyCode::Char(' ') => {
            ui.help_scroll_by(ui.help_page_height() as isize);
        }
        _ => {}
    }
}

fn parse_rank_list(arg: &str) -> Result<Vec<usize>, String> {
    let mut ranks: HashSet<usize> = HashSet::new();
    for part in arg.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start_str, end_str)) = part.split_once('-') {
            let start = start_str
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid number: {}", start_str.trim()))?;
            let end = end_str
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid number: {}", end_str.trim()))?;
            if start == 0 || end == 0 {
                return Err(String::from("Sequence numbers start at 1"));
            }
            if start > end {
                return Err(format!("Invalid range: {}-{}", start, end));
            }
            for num in start..=end {
                ranks.insert(num - 1);
            }
        } else {
            let num = part
                .parse::<usize>()
                .map_err(|_| format!("Invalid number: {}", part))?;
            if num == 0 {
                return Err(String::from("Sequence numbers start at 1"));
            }
            ranks.insert(num - 1);
        }
    }
    if ranks.is_empty() {
        return Err(String::from("No sequence numbers provided"));
    }
    let mut result: Vec<usize> = ranks.into_iter().collect();
    result.sort_unstable();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::parse_rank_list;

    #[test]
    fn parse_rank_list_single_and_range() {
        let result = parse_rank_list("1,4,6-8").unwrap();
        assert_eq!(result, vec![0, 3, 5, 6, 7]);
    }

    #[test]
    fn parse_rank_list_rejects_zero() {
        assert!(parse_rank_list("0").is_err());
    }
}

fn handle_pending_count_key(ui: &mut UI, key_event: KeyEvent, count: usize) -> bool {
    let mut done = false;
    match key_event.code {
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let d = (c as u8 - b'0') as usize;
            let updated_count = count.saturating_mul(10).saturating_add(d);
            ui.input_mode = InputMode::PendingCount {
                count: updated_count,
            };
            ui.app.add_argument_char(c);
        }
        // Q, q, and Ctrl-C quit
        KeyCode::Char('q') | KeyCode::Char('Q') => done = true,
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => done = true,
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        _ => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            dispatch_command(ui, key_event, Some(count));
        }
    }
    done
}

fn handle_label_search(ui: &mut UI, key_event: KeyEvent, pattern: &str) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || ' ' == c => {
            ui.app.add_argument_char(c);
            let mut updated_pattern = pattern.to_string();
            updated_pattern.push(c);
            ui.input_mode = InputMode::LabelSearch {
                pattern: updated_pattern,
            }
        }
        KeyCode::Delete | KeyCode::Backspace => {
            ui.app.pop_argument_char();
            let mut updated_pattern = pattern.to_string();
            updated_pattern.pop();
            ui.input_mode = InputMode::LabelSearch {
                pattern: updated_pattern,
            };
        }
        KeyCode::Enter => {
            ui.app.regex_search_labels(pattern);
            ui.input_mode = InputMode::Normal;
            if let Some(_) = &ui.app.search_state {
                // Could be a malformed regex
                ui.jump_to_next_lbl_match(0);
            }
        }
        _ => {}
    }
}

fn handle_search(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor, kind: SearchKind) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        KeyCode::Enter => {
            let query = editor.text();
            match kind {
                SearchKind::Regex => ui.app.regex_search_sequences(&query),
                SearchKind::Emboss => ui.app.emboss_search_sequences(&query),
            }
            ui.input_mode = InputMode::Normal;
            if let Some((total, sequences)) = ui.app.seq_search_counts() {
                ui.app
                    .info_msg(format!("{total} matches in {sequences} sequences"));
            } else if query.is_empty() {
                ui.app.info_msg("0 matches in 0 sequences");
            }
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::Search { editor, kind };
            ui.app
                .argument_msg(String::from("Search: "), ui.search_query());
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::Search { editor, kind };
            ui.app
                .argument_msg(String::from("Search: "), ui.search_query());
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::Search { editor, kind };
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::Search { editor, kind };
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::Search { editor, kind };
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::Search { editor, kind };
        }
        _ => {}
    }
}

fn handle_command(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        KeyCode::Enter => {
            let cmd = editor.text();
            ui.input_mode = InputMode::Normal;
            if cmd.trim() == "s" {
                let selected = if ui.app.saved_searches().is_empty() {
                    0
                } else {
                    0
                };
                ui.input_mode = InputMode::SearchList { selected };
            } else if cmd.trim() == "es" {
                let default_path = format!("{}.svg", ui.app.filename);
                let mut editor = LineEditor::new();
                for c in default_path.chars() {
                    editor.insert_char(c);
                }
                ui.input_mode = InputMode::ExportSvg { editor };
                ui.app.argument_msg(String::new(), ui.export_svg_text());
            } else if cmd.trim() == "ra" {
                ui.app.info_msg("Running mafft...");
                match ui.app.realign_with_mafft() {
                    Ok(()) => {
                        ui.show_tree_panel(true);
                        ui.app.info_msg("Realigned with mafft");
                    }
                    Err(e) => ui.app.error_msg(format!("mafft failed: {}", e)),
                }
            } else if cmd.trim() == "tt" {
                if ui.app.has_tree_panel() {
                    ui.toggle_tree_panel();
                } else {
                    ui.app.warning_msg("No tree available");
                }
            } else if cmd.trim() == "rc" {
                ui.input_mode = InputMode::ConfirmReject {
                    mode: RejectMode::Current,
                };
                ui.app.info_msg("Reject current match? (y/n)");
            } else if cmd.trim() == "ru" {
                ui.input_mode = InputMode::ConfirmReject {
                    mode: RejectMode::Unmatched,
                };
                ui.app.info_msg("Reject unmatched sequences? (y/n)");
            } else if cmd.trim() == "rm" {
                ui.input_mode = InputMode::ConfirmReject {
                    mode: RejectMode::Matched,
                };
                ui.app.info_msg("Reject matched sequences? (y/n)");
            } else if cmd.trim() == "ur" {
                match ui.app.undo_last_reject() {
                    Ok(count) => {
                        if count == 0 {
                            ui.app.info_msg("Nothing to undo");
                        } else {
                            ui.app.info_msg(format!("Undid {} rejection(s)", count));
                        }
                    }
                    Err(e) => ui.app.error_msg(format!("Undo failed: {}", e)),
                }
            } else if cmd.trim_start().starts_with("sn") {
                let arg = cmd.trim_start()[2..].trim();
                match arg.parse::<usize>() {
                    Ok(num) if num > 0 => {
                        let rank = num - 1;
                        match ui.select_label_by_rank(rank) {
                            Ok(()) => ui.app.info_msg(format!("Selected #{}", num)),
                            Err(e) => ui.app.error_msg(format!("Select failed: {}", e)),
                        }
                    }
                    _ => ui.app.warning_msg("Usage: :sn <number>"),
                }
            } else if cmd.trim_start().starts_with("rn") {
                let arg = cmd.trim_start()[2..].trim();
                match parse_rank_list(arg) {
                    Ok(ranks) => {
                        let out_path = ui.app.rejected_path();
                        match ui.app.reject_sequences(&ranks, &out_path) {
                            Ok(count) => {
                                if count == 0 {
                                    ui.app.warning_msg("No sequences to reject");
                                } else {
                                    ui.app.info_msg(format!(
                                        "Rejected {} -> {}",
                                        count,
                                        out_path.display()
                                    ));
                                    if ui.app.alignment.num_seq() == 0 {
                                        ui.set_exit_message(
                                            "all sequences have been rejected, ending program",
                                        );
                                    }
                                }
                            }
                            Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
                        }
                    }
                    Err(msg) => ui.app.warning_msg(msg),
                }
            } else {
                ui.app.warning_msg(format!("Unknown command: {}", cmd));
            }
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::Command { editor };
            ui.app.argument_msg(String::from(":"), ui.command_text());
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::Command { editor };
            ui.app.argument_msg(String::from(":"), ui.command_text());
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::Command { editor };
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::Command { editor };
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::Command { editor };
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::Command { editor };
        }
        _ => {}
    }
}

fn handle_search_list(ui: &mut UI, key_event: KeyEvent, selected: usize) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        KeyCode::Char('a') => {
            if let Some(query) = ui.app.current_seq_search_pattern() {
                let name = query.to_string();
                let kind = ui
                    .app
                    .current_seq_search_kind()
                    .unwrap_or(SearchKind::Regex);
                match ui
                    .app
                    .add_saved_search_with_kind(name, query.to_string(), kind)
                {
                    Ok(_) => {
                        let last = ui.app.saved_searches().len().saturating_sub(1);
                        ui.input_mode = InputMode::SearchList { selected: last };
                        ui.app.info_msg("Added saved search");
                    }
                    Err(e) => ui.app.error_msg(e),
                }
            } else {
                ui.app.warning_msg("No current query to save");
            }
        }
        KeyCode::Char('d') => {
            if ui.app.delete_saved_search(selected) {
                let len = ui.app.saved_searches().len();
                let new_selected = if len == 0 {
                    0
                } else if selected >= len {
                    len - 1
                } else {
                    selected
                };
                ui.input_mode = InputMode::SearchList {
                    selected: new_selected,
                };
            }
        }
        KeyCode::Char('c') => {
            if let Some(entry) = ui.app.saved_searches().get(selected) {
                let query = entry.query.clone();
                match entry.kind {
                    SearchKind::Regex => ui.app.regex_search_sequences(&query),
                    SearchKind::Emboss => ui.app.emboss_search_sequences(&query),
                }
                ui.app.info_msg("Current search set");
            }
        }
        KeyCode::Char(' ') => {
            if ui.app.toggle_saved_search(selected) {
                ui.input_mode = InputMode::SearchList { selected };
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as u8 - b'1') as usize;
            if idx < ui.app.saved_searches().len() {
                ui.input_mode = InputMode::SearchList { selected: idx };
            }
        }
        _ => {}
    }
}

fn handle_export_svg(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
        }
        KeyCode::Enter => {
            let path = editor.text();
            if path.trim().is_empty() {
                ui.input_mode = InputMode::ExportSvg { editor };
                ui.app.warning_msg("Export path cannot be empty");
                return;
            }
            if std::path::Path::new(&path).exists() {
                ui.input_mode = InputMode::ConfirmOverwrite { editor, path };
                ui.app.info_msg("Overwrite SVG? (y/n)");
            } else {
                ui.app.argument_msg(String::new(), path.clone());
                match ui.export_svg(std::path::Path::new(&path)) {
                    Ok(_) => {}
                    Err(e) => ui.app.error_msg(format!("Export failed: {}", e)),
                }
                ui.input_mode = InputMode::Normal;
            }
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::ExportSvg { editor };
            ui.app.argument_msg(String::new(), ui.export_svg_text());
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::ExportSvg { editor };
            ui.app.argument_msg(String::new(), ui.export_svg_text());
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::ExportSvg { editor };
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::ExportSvg { editor };
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::ExportSvg { editor };
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::ExportSvg { editor };
        }
        _ => {}
    }
}

fn handle_confirm_overwrite(ui: &mut UI, key_event: KeyEvent, editor: LineEditor, path: String) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            ui.app.argument_msg(String::new(), path.clone());
            match ui.export_svg(std::path::Path::new(&path)) {
                Ok(_) => {}
                Err(e) => ui.app.error_msg(format!("Export failed: {}", e)),
            }
            ui.input_mode = InputMode::Normal;
        }
        _ => {
            ui.input_mode = InputMode::ExportSvg { editor };
            ui.app.argument_msg(String::new(), ui.export_svg_text());
        }
    }
}

fn handle_confirm_reject(ui: &mut UI, key_event: KeyEvent, mode: RejectMode) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            perform_reject(ui, mode);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            ui.app.info_msg("Reject canceled");
        }
        _ => {}
    }
}

fn perform_reject(ui: &mut UI, mode: RejectMode) {
    let out_path = ui.app.rejected_path();
    let ranks = match mode {
        RejectMode::Current => ui.app.current_seq_match().map(|m| vec![m.seq_index]),
        RejectMode::Matched => ui.app.seq_search_spans().map(|spans| {
            spans
                .iter()
                .enumerate()
                .filter_map(|(idx, spans)| (!spans.is_empty()).then_some(idx))
                .collect::<Vec<usize>>()
        }),
        RejectMode::Unmatched => ui.app.seq_search_spans().map(|spans| {
            spans
                .iter()
                .enumerate()
                .filter_map(|(idx, spans)| spans.is_empty().then_some(idx))
                .collect::<Vec<usize>>()
        }),
    };
    let Some(ranks) = ranks else {
        if matches!(mode, RejectMode::Current) {
            if ui.app.seq_search_spans().is_some() {
                ui.app.warning_msg("No current match");
            } else {
                ui.app.warning_msg("No current sequence search");
            }
        } else {
            ui.app.warning_msg("No current sequence search");
        }
        return;
    };
    if ranks.is_empty() {
        ui.app.warning_msg("No sequences to reject");
        return;
    }
    match ui.app.reject_sequences(&ranks, &out_path) {
        Ok(count) => {
            ui.app
                .info_msg(format!("Rejected {} -> {}", count, out_path.display()));
            if ui.app.alignment.num_seq() == 0 {
                ui.set_exit_message("all sequences have been rejected, ending program");
            }
        }
        Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
    }
}

fn dispatch_command(ui: &mut UI, key_event: KeyEvent, count_arg: Option<usize>) {
    let count = count_arg.unwrap_or(1);

    // debug!("key event: {:#?}", key_event.code);
    match key_event.code {
        // ----- Hide/Show panes -----

        // Left pane
        KeyCode::Char('a') => {
            if ui.left_pane_width == 0 {
                ui.show_label_pane();
            } else {
                ui.hide_label_pane();
            }
        }

        // Bottom pane
        KeyCode::Char('c') => {
            if ui.bottom_pane_height == 0 {
                ui.show_bottom_pane();
            } else {
                ui.hide_bottom_pane();
            }
        }

        // Both panes
        KeyCode::Char('f') => {
            if ui.full_screen {
                ui.show_label_pane();
                ui.show_bottom_pane();
                ui.full_screen = false;
            } else {
                ui.hide_label_pane();
                ui.hide_bottom_pane();
                ui.full_screen = true;
            }
        }

        // ----- Motion -----

        // Arrows - late introduction, but might be friendlier to new users.
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            // Non-shifted arrow keys
            if !key_event.modifiers.contains(KeyModifiers::SHIFT) {
                match key_event.code {
                    KeyCode::Down => match ui.zoom_level() {
                        ZoomLevel::ZoomedIn => ui.scroll_one_line_down(count as u16),
                        ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                            ui.scroll_zoombox_one_line_down(count as u16)
                        }
                    },
                    KeyCode::Up => match ui.zoom_level() {
                        ZoomLevel::ZoomedIn => ui.scroll_one_line_up(count as u16),
                        ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                            ui.scroll_zoombox_one_line_up(count as u16)
                        }
                    },
                    KeyCode::Right => match ui.zoom_level() {
                        ZoomLevel::ZoomedIn => ui.scroll_one_col_right(count as u16),
                        ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                            ui.scroll_zoombox_one_col_right(count as u16)
                        }
                    },
                    KeyCode::Left => match ui.zoom_level() {
                        ZoomLevel::ZoomedIn => ui.scroll_one_col_left(count as u16),
                        ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                            ui.scroll_zoombox_one_col_left(count as u16)
                        }
                    },

                    _ => panic!("Expected only arrow keycodes"),
                }
            } else {
                // Shifted arrow keys
                match key_event.code {
                    KeyCode::Up => ui.scroll_one_screen_up(count as u16),
                    KeyCode::Left => ui.scroll_one_screen_left(count as u16),
                    KeyCode::Down => ui.scroll_one_screen_down(count as u16),
                    KeyCode::Right => ui.scroll_one_screen_right(count as u16),

                    _ => panic!("Expected only arrow keycodes"),
                }
            }
        }

        // Up
        KeyCode::Char('k') => match ui.zoom_level() {
            ZoomLevel::ZoomedIn => ui.scroll_one_line_up(count as u16),
            ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                ui.scroll_zoombox_one_line_up(count as u16)
            }
        },
        KeyCode::Char('K') => ui.scroll_one_screen_up(count as u16),
        KeyCode::Char('g') => ui.jump_to_top(),

        // Left
        KeyCode::Char('h') => match ui.zoom_level() {
            ZoomLevel::ZoomedIn => ui.scroll_one_col_left(count as u16),
            ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                ui.scroll_zoombox_one_col_left(count as u16)
            }
        },
        KeyCode::Char('H') => ui.scroll_one_screen_left(count as u16),
        KeyCode::Char('^') => ui.jump_to_begin(),

        // Down
        KeyCode::Char('j') => match ui.zoom_level() {
            ZoomLevel::ZoomedIn => ui.scroll_one_line_down(count as u16),
            ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                ui.scroll_zoombox_one_line_down(count as u16)
            }
        },
        KeyCode::Char('J') | KeyCode::Char(' ') => ui.scroll_one_screen_down(count as u16),
        KeyCode::Char('G') => ui.jump_to_bottom(),

        // Right
        KeyCode::Char('l') => match ui.zoom_level() {
            ZoomLevel::ZoomedIn => ui.scroll_one_col_right(count as u16),
            ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                ui.scroll_zoombox_one_col_right(count as u16)
            }
        },
        KeyCode::Char('L') => ui.scroll_one_screen_right(count as u16),
        KeyCode::Char('$') => ui.jump_to_end(),

        // Absolute Positions

        // Visible line
        KeyCode::Char('-') => ui.jump_to_line((count as u16) - 1), // -1: user is 1-based

        // Column
        KeyCode::Char('|') => ui.jump_to_col(count as u16),

        // Relative positions

        // Vertical
        KeyCode::Char('%') => ui.jump_to_pct_line(count as u16),

        // Horizontal
        KeyCode::Char('#') => ui.jump_to_pct_col(count as u16),

        // To search matches
        KeyCode::Char('n') => ui.jump_to_next_lbl_match(count as i16),
        KeyCode::Char('p') => ui.jump_to_next_lbl_match(-1 * count as i16),
        KeyCode::Char(']') => ui.jump_to_next_seq_match(count as i16),
        KeyCode::Char('[') => ui.jump_to_next_seq_match(-1 * count as i16),

        // Left Pane width
        KeyCode::Char('>') => ui.widen_label_pane(count as u16),
        KeyCode::Char('<') => ui.reduce_label_pane(count as u16),

        // Zoom
        KeyCode::Char('z') => ui.cycle_zoom(),
        // Since there are 3 zoom levels, cycling twice amounts to cycling
        // backwards.
        KeyCode::Char('Z') => {
            ui.cycle_zoom();
            ui.cycle_zoom();
        }
        // Toggle zoom box guides
        KeyCode::Char('v') => {
            ui.set_zoombox_guides(!ui.show_zb_guides);
        }
        // Toggle zoom box visibility
        KeyCode::Char('B') => {
            ui.toggle_zoombox();
        }

        // Bottom pane position (i.e., bottom of screen or stuck to the alignment - when both
        // are possible).
        // TODO: not sure we're keeping the "bottom" position. Seems much better to stick it to the
        // last seq in the alignment.
        KeyCode::Char('b') => {
            ui.cycle_bottom_pane_position();
        }

        // ---- Visuals ----

        // Mark consensus positions that are retained in the zoom box
        KeyCode::Char('r') => ui.toggle_hl_retained_cols(),

        // Inverse video
        KeyCode::Char('i') => {
            ui.toggle_video_mode();
        }

        KeyCode::Char('s') => ui.next_color_scheme(),
        KeyCode::Char('S') => ui.prev_color_scheme(),

        // Switch to next/previous colormap in the list
        KeyCode::Char('m') => ui.next_colormap(),
        KeyCode::Char('M') => ui.prev_colormap(),

        // Sequence Order
        KeyCode::Char('o') => ui.app.next_ordering_criterion(),
        KeyCode::Char('O') => ui.app.prev_ordering_criterion(),

        // Metric
        KeyCode::Char('t') => ui.app.next_metric(),
        KeyCode::Char('T') => ui.app.prev_metric(),

        // ----- Search -----
        KeyCode::Char('/') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Regex,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
        }
        KeyCode::Char('\\') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Emboss,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
        }
        KeyCode::Char('P') => {
            if let (Some(query), Some(kind)) = (
                ui.app.current_seq_search_pattern(),
                ui.app.current_seq_search_kind(),
            ) {
                match ui
                    .app
                    .add_saved_search_with_kind(query.to_string(), query.to_string(), kind)
                {
                    Ok(_) => {
                        ui.app.clear_seq_search();
                        ui.app.info_msg("Saved current search");
                    }
                    Err(e) => ui.app.error_msg(e),
                }
            } else {
                ui.app.warning_msg("No current search to save");
            }
        }

        // ----- Editing -----
        // Filter alignment through external command (à la Vim's '!')
        KeyCode::Char('!') => {
            if let Some(rank) = ui.app.current_label_match_rank() {
                let out_path = ui.app.rejected_path();
                match ui.app.reject_sequences(&[rank], &out_path) {
                    Ok(count) => {
                        if count == 0 {
                            ui.app.warning_msg("No current label match");
                        } else {
                            ui.app
                                .info_msg(format!("Rejected -> {}", out_path.display()));
                            if ui.app.alignment.num_seq() == 0 {
                                ui.set_exit_message(
                                    "all sequences have been rejected, ending program",
                                );
                            }
                        }
                    }
                    Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
                }
            } else {
                ui.app.warning_msg("No current label match");
            }
        }
        KeyCode::Char('W') => {
            let out_path = ui.app.filtered_path();
            match ui.app.write_alignment_fasta(&out_path) {
                Ok(_) => ui
                    .app
                    .info_msg(format!("Filtered -> {}", out_path.display())),
                Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
            }
        }
        KeyCode::Char(':') => {
            ui.input_mode = InputMode::Command {
                editor: LineEditor::new(),
            };
            ui.app.argument_msg(String::from(":"), String::from(""));
        }

        _ => {
            // let the user know this key is not bound
            //
            // TODO: there are pros and cons about this - first, the user can probably guess
            // that if nothing happens then the key isn't bound. Second, the message should be
            // disabled after the user presses a bound key, which would force us to either add
            // code to that effect for _every single_ key binding, or do a first match on every
            // valid key (to disable the message) and then match on each individual key to
            // launch the desired action. Not sure it's worth it, frankly.
            // ui.warning_msg(format!("'{}' not bound", c));
        }
    }
}
