// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Thomas Junier
// Modifications (c) 2026 Peter Carlton
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;

use super::{
    line_editor::LineEditor,
    InputMode,
    InputMode::{
        Command, ConfirmOverwrite, ConfirmReject, ConfirmSessionOverwrite, ConfirmViewDelete,
        ExportSvg, Help, LabelSearch, Normal, Notes, PendingCount, Search, SearchList, SessionList,
        SessionSave, TreeNav, ViewCreate, ViewCreateWithList, ViewDelete, ViewList, ViewMove,
    },
    //SearchDirection,
    {NotesTarget, RejectMode, ZoomLevel, UI},
};
use crate::app::{RejectAction, RejectResult, SearchKind};
use std::collections::HashSet;

fn handle_notes(
    ui: &mut UI,
    key_event: KeyEvent,
    mut editor: super::notes_editor::NotesEditor,
    target: NotesTarget,
) {
    match key_event.code {
        KeyCode::Esc => {
            match target {
                NotesTarget::Global => ui.app.set_notes(editor.text()),
                NotesTarget::View => ui.app.set_view_notes(editor.text()),
            }
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            editor.newline();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char('m') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.newline();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char(c)
            if (c.is_ascii_graphic() || c == ' ')
                && !key_event.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            editor.insert_char(c);
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Left if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.move_word_left();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Right if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.move_word_right();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Up => {
            editor.move_up();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Down => {
            editor.move_down();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char('w') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.delete_word_left();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char('a') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.move_line_start();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char('e') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.move_line_end();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.move_word_left();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        KeyCode::Char('f') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.move_word_right();
            ui.input_mode = InputMode::Notes { editor, target };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_reject_result(ui: &mut UI, result: RejectResult) {
    match result.action {
        RejectAction::RejectedToFile => {
            if result.count == 0 {
                ui.app.info_msg("No sequences rejected");
            } else {
                ui.app
                    .info_msg(format!("Rejected {} sequences", result.count));
            }
            mark_dirty(ui);
        }
        RejectAction::RemovedFromView => {
            if result.count == 0 {
                ui.app.info_msg("No sequences removed");
            } else {
                ui.app
                    .info_msg(format!("Removed {} sequences from view", result.count));
            }
            mark_dirty(ui);
        }
        RejectAction::AlreadyRejected => {
            ui.app.info_msg("No sequences rejected");
            mark_dirty(ui);
        }
    }
}
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
        SessionSave { editor } => handle_session_save(ui, key_event, editor),
        ConfirmSessionOverwrite { editor, path } => {
            handle_confirm_session_overwrite(ui, key_event, editor, path)
        }
        SearchList { selected } => handle_search_list(ui, key_event, selected),
        SessionList { selected, files } => handle_session_list(ui, key_event, selected, &files),
        Notes { editor, target } => handle_notes(ui, key_event, editor, target),
        ConfirmReject { mode } => handle_confirm_reject(ui, key_event, mode),
        ConfirmViewDelete { name } => handle_confirm_view_delete(ui, key_event, &name),
        TreeNav { nav } => handle_tree_nav(ui, key_event, nav),
        ViewList { selected } => handle_view_list(ui, key_event, selected),
        ViewCreate { editor } => handle_view_create(ui, key_event, editor),
        ViewCreateWithList { editor } => handle_view_create_with_list(ui, key_event, editor),
        ViewDelete { selected } => handle_view_delete(ui, key_event, selected),
        ViewMove { selected, ranks } => handle_view_move(ui, key_event, selected, &ranks),
    };
    if ui.has_exit_message() {
        true
    } else {
        done
    }
}

fn selected_ranks(ui: &UI) -> Vec<usize> {
    ui.app.selection_ranks()
}

fn mark_dirty(ui: &mut UI) {
    ui.mark_dirty();
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
            mark_dirty(ui);
        }
        KeyCode::Esc => {
            ui.app.reset_lbl_search();
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        // Q, q, and Ctrl-C quit
        KeyCode::Char('q') | KeyCode::Char('Q') => done = true,
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => done = true,
        // TODO: search
        KeyCode::Char('?') => {
            ui.reset_help_scroll();
            ui.input_mode = InputMode::Help;
            mark_dirty(ui);
        }
        KeyCode::Char('"') => {
            ui.input_mode = InputMode::LabelSearch {
                pattern: String::from(""),
            };
            ui.app
                .argument_msg(String::from("Label search: "), String::from(""));
            mark_dirty(ui);
        }
        KeyCode::Char(':') => {
            ui.input_mode = InputMode::Command {
                editor: LineEditor::new(),
            };
            ui.app.argument_msg(String::from(":"), String::from(""));
            mark_dirty(ui);
        }
        KeyCode::Char('/') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Regex,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
            mark_dirty(ui);
        }
        KeyCode::Char('\\') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Emboss,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
            mark_dirty(ui);
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
            mark_dirty(ui);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            ui.help_scroll_by(-1);
            mark_dirty(ui);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            ui.help_scroll_by(1);
            mark_dirty(ui);
        }
        KeyCode::PageUp => {
            ui.help_scroll_by(-(ui.help_page_height() as isize));
            mark_dirty(ui);
        }
        KeyCode::PageDown | KeyCode::Char(' ') => {
            ui.help_scroll_by(ui.help_page_height() as isize);
            mark_dirty(ui);
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
            mark_dirty(ui);
        }
        // Q, q, and Ctrl-C quit
        KeyCode::Char('q') | KeyCode::Char('Q') => done = true,
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => done = true,
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        _ => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            dispatch_command(ui, key_event, Some(count));
            mark_dirty(ui);
        }
    }
    done
}

fn handle_label_search(ui: &mut UI, key_event: KeyEvent, pattern: &str) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || ' ' == c => {
            ui.app.add_argument_char(c);
            let mut updated_pattern = pattern.to_string();
            updated_pattern.push(c);
            ui.input_mode = InputMode::LabelSearch {
                pattern: updated_pattern,
            };
            mark_dirty(ui);
        }
        KeyCode::Delete | KeyCode::Backspace => {
            ui.app.pop_argument_char();
            let mut updated_pattern = pattern.to_string();
            updated_pattern.pop();
            ui.input_mode = InputMode::LabelSearch {
                pattern: updated_pattern,
            };
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            ui.app.regex_search_labels(pattern);
            ui.input_mode = InputMode::Normal;
            if let Some(_) = &ui.app.search_state {
                // Could be a malformed regex
                ui.jump_to_next_lbl_match(0);
            }
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_search(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor, kind: SearchKind) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
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
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::Search { editor, kind };
            ui.app
                .argument_msg(String::from("Search: "), ui.search_query());
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::Search { editor, kind };
            ui.app
                .argument_msg(String::from("Search: "), ui.search_query());
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::Search { editor, kind };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::Search { editor, kind };
            mark_dirty(ui);
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::Search { editor, kind };
            mark_dirty(ui);
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::Search { editor, kind };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_command(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
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
            } else if cmd.trim() == "tn" {
                if ui.app.tree().is_none() {
                    ui.app.info_msg("Running mafft...");
                    match ui.app.realign_with_mafft() {
                        Ok(()) => {
                            ui.show_tree_panel(true);
                            ui.app.info_msg("Realigned with mafft");
                        }
                        Err(e) => {
                            ui.app.error_msg(format!("mafft failed: {}", e));
                            mark_dirty(ui);
                            return;
                        }
                    }
                }
                match ui.app.tree() {
                    Some(tree) => match super::build_tree_nav(ui.app, tree) {
                        Ok(nav) => {
                            apply_tree_nav_selection(ui, &nav);
                            ui.input_mode = InputMode::TreeNav { nav };
                        }
                        Err(e) => ui
                            .app
                            .error_msg(format!("Tree navigation unavailable: {}", e)),
                    },
                    None => ui.app.warning_msg("No tree available"),
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
            } else if cmd.trim() == "ss" {
                let default_path = ui.app.default_session_path();
                let mut editor = LineEditor::new();
                for c in default_path.to_string_lossy().chars() {
                    editor.insert_char(c);
                }
                ui.input_mode = InputMode::SessionSave { editor };
                ui.app
                    .argument_msg(String::from("Session: "), ui.session_save_text());
            } else if cmd.trim() == "vc" {
                let editor = LineEditor::new();
                ui.input_mode = InputMode::ViewCreate { editor };
                ui.app
                    .argument_msg(String::from("View name: "), String::new());
            } else if cmd.trim() == "vx" {
                if ui.app.selection_ranks().is_empty() {
                    ui.app.warning_msg("No selected sequences");
                } else {
                    let editor = LineEditor::new();
                    ui.input_mode = InputMode::ViewCreateWithList { editor };
                    ui.app
                        .argument_msg(String::from("View name: "), String::new());
                }
            } else if cmd.trim() == "vs" {
                if ui.app.view_names().is_empty() {
                    ui.app.warning_msg("No views available");
                } else {
                    ui.input_mode = InputMode::ViewList { selected: 0 };
                }
            } else if cmd.trim() == "vd" {
                let views = ui.app.view_names();
                let first = views
                    .iter()
                    .position(|name| !crate::app::App::is_protected_view(name));
                if let Some(selected) = first {
                    ui.input_mode = InputMode::ViewDelete { selected };
                } else {
                    ui.app.warning_msg("No deletable views");
                }
            } else if cmd.trim() == "cc" {
                ui.app.clear_cursor();
                ui.app.info_msg("Cleared cursor");
            } else if cmd.trim_start().starts_with("mv") {
                let arg = cmd.trim_start()[2..].trim();
                let ranks = if arg.is_empty() {
                    selected_ranks(ui)
                } else {
                    match parse_rank_list(arg) {
                        Ok(ranks) => ranks,
                        Err(msg) => {
                            ui.app.warning_msg(msg);
                            mark_dirty(ui);
                            return;
                        }
                    }
                };
                if ranks.is_empty() {
                    ui.app.warning_msg("No sequences to move");
                    mark_dirty(ui);
                    return;
                }
                let views = ui.app.view_names();
                let first = views
                    .iter()
                    .position(|name| crate::app::App::is_move_target_view(name));
                if let Some(selected) = first {
                    ui.input_mode = InputMode::ViewMove { selected, ranks };
                } else {
                    ui.app.warning_msg("No target views available");
                }
            } else if cmd.trim() == "rs" {
                let ranks = selected_ranks(ui);
                if ranks.is_empty() {
                    ui.app.warning_msg("No selected sequences");
                    mark_dirty(ui);
                    return;
                }
                let out_path = ui.app.rejected_output_path();
                match ui.app.reject_sequences(&ranks, &out_path) {
                    Ok(result) => handle_reject_result(ui, result),
                    Err(e) => ui.app.error_msg(format!("Rejection failed: {}", e)),
                }
            } else if cmd.trim() == "sl" {
                let read_dir = match fs::read_dir(".") {
                    Ok(read_dir) => read_dir,
                    Err(e) => {
                        ui.app.error_msg(format!("Session list failed: {}", e));
                        mark_dirty(ui);
                        return;
                    }
                };
                let mut files: Vec<String> = read_dir
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let path = entry.path();
                        let name = path.file_name()?.to_string_lossy().to_string();
                        if path.extension().and_then(|s| s.to_str()) == Some("trml") {
                            Some(name)
                        } else {
                            None
                        }
                    })
                    .collect();
                files.sort();
                if files.is_empty() {
                    ui.app.warning_msg("No .trml files found");
                } else {
                    ui.input_mode = InputMode::SessionList { selected: 0, files };
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
                        let out_path = ui.app.rejected_output_path();
                        match ui.app.reject_sequences(&ranks, &out_path) {
                            Ok(result) => handle_reject_result(ui, result),
                            Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
                        }
                    }
                    Err(msg) => ui.app.warning_msg(msg),
                }
            } else {
                ui.app.warning_msg(format!("Unknown command: {}", cmd));
            }
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::Command { editor };
            ui.app.argument_msg(String::from(":"), ui.command_text());
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::Command { editor };
            ui.app.argument_msg(String::from(":"), ui.command_text());
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::Command { editor };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::Command { editor };
            mark_dirty(ui);
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::Command { editor };
            mark_dirty(ui);
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::Command { editor };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn apply_tree_nav_selection(ui: &mut UI, nav: &super::TreeNav) {
    let ranks = nav.selected_leaf_ranks();
    let range = nav.selected_leaf_range();
    ui.app.set_label_matches_from_tree(ranks, range);
}

fn autoscroll_tree_selection(ui: &mut UI, ranks: &[usize]) {
    if ranks.is_empty() {
        return;
    }
    let visible = ui.visible_seq_rows() as usize;
    if visible == 0 {
        return;
    }
    let top = ui.top_line() as usize;
    let bottom = top + visible - 1;
    let first_rank = ranks[0];
    let first_screen = ui.app.rank_to_screenline(first_rank);
    let mut max_screen = first_screen;
    for rank in ranks.iter().skip(1) {
        let screen = ui.app.rank_to_screenline(*rank);
        if screen > max_screen {
            max_screen = screen;
        }
    }
    if first_screen >= top && max_screen <= bottom {
        return;
    }
    let context = visible / 4;
    let mut desired_top = first_screen.saturating_sub(context);
    if desired_top + visible - 1 < max_screen {
        desired_top = max_screen.saturating_sub(visible - 1);
    }
    let desired_top = desired_top.min(ui.max_top_line() as usize);
    ui.jump_to_line(desired_top as u16);
    mark_dirty(ui);
}

fn handle_tree_nav(ui: &mut UI, key_event: KeyEvent, mut nav: super::TreeNav) {
    let mut changed = false;
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
            return;
        }
        KeyCode::Up if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
            ui.scroll_one_screen_up(1);
            mark_dirty(ui);
        }
        KeyCode::Down if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
            ui.scroll_one_screen_down(1);
            mark_dirty(ui);
        }
        KeyCode::Left if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
            ui.scroll_one_screen_left(1);
            mark_dirty(ui);
        }
        KeyCode::Right if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
            ui.scroll_one_screen_right(1);
            mark_dirty(ui);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            changed = nav.move_left();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            changed = nav.move_right();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            changed = nav.move_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            changed = nav.move_down();
        }
        _ => {}
    }
    if changed {
        apply_tree_nav_selection(ui, &nav);
        autoscroll_tree_selection(ui, &nav.selected_leaf_ranks());
        mark_dirty(ui);
    }
    ui.input_mode = InputMode::TreeNav { nav };
}

fn handle_search_list(ui: &mut UI, key_event: KeyEvent, selected: usize) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
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
            mark_dirty(ui);
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
                mark_dirty(ui);
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
                mark_dirty(ui);
            }
        }
        KeyCode::Char(' ') => {
            if ui.app.toggle_saved_search(selected) {
                ui.input_mode = InputMode::SearchList { selected };
                mark_dirty(ui);
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as u8 - b'1') as usize;
            if idx < ui.app.saved_searches().len() {
                ui.input_mode = InputMode::SearchList { selected: idx };
                mark_dirty(ui);
            }
        }
        _ => {}
    }
}

fn handle_view_list(ui: &mut UI, key_event: KeyEvent, selected: usize) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let len = ui.app.view_names().len();
            if len == 0 {
                return;
            }
            let new_selected = selected.checked_sub(1).unwrap_or(len - 1);
            ui.input_mode = InputMode::ViewList {
                selected: new_selected,
            };
            mark_dirty(ui);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let len = ui.app.view_names().len();
            if len == 0 {
                return;
            }
            let new_selected = (selected + 1) % len;
            ui.input_mode = InputMode::ViewList {
                selected: new_selected,
            };
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let views = ui.app.view_names();
            if let Some(name) = views.get(selected).cloned() {
                match ui.app.switch_view(&name) {
                    Ok(()) => {
                        ui.input_mode = InputMode::Normal;
                        ui.app.info_msg(format!("View: {}", name));
                    }
                    Err(e) => ui.app.error_msg(format!("View switch failed: {}", e)),
                }
                mark_dirty(ui);
            }
        }
        _ => {}
    }
}

fn advance_view_selection(
    views: &[String],
    selected: usize,
    direction: isize,
    selectable: impl Fn(&str) -> bool,
) -> usize {
    if views.is_empty() {
        return selected;
    }
    let mut idx = selected;
    for _ in 0..views.len() {
        let mut next = idx as isize + direction;
        if next < 0 {
            next = views.len() as isize - 1;
        } else if next as usize >= views.len() {
            next = 0;
        }
        idx = next as usize;
        if selectable(&views[idx]) {
            return idx;
        }
    }
    selected
}

fn handle_view_delete(ui: &mut UI, key_event: KeyEvent, selected: usize) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let views = ui.app.view_names();
            let next = advance_view_selection(views, selected, 1, |name| {
                !crate::app::App::is_protected_view(name)
            });
            ui.input_mode = InputMode::ViewDelete { selected: next };
            mark_dirty(ui);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let views = ui.app.view_names();
            let next = advance_view_selection(views, selected, -1, |name| {
                !crate::app::App::is_protected_view(name)
            });
            ui.input_mode = InputMode::ViewDelete { selected: next };
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let views = ui.app.view_names();
            if let Some(name) = views.get(selected) {
                if crate::app::App::is_protected_view(name) {
                    ui.app.warning_msg("View cannot be deleted");
                } else {
                    ui.input_mode = InputMode::ConfirmViewDelete { name: name.clone() };
                    ui.app.info_msg(format!("Delete view {}? (y/n)", name));
                }
                mark_dirty(ui);
            }
        }
        _ => {}
    }
}

fn handle_view_move(ui: &mut UI, key_event: KeyEvent, selected: usize, ranks: &[usize]) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let views = ui.app.view_names();
            let next = advance_view_selection(views, selected, 1, |name| {
                crate::app::App::is_move_target_view(name)
            });
            ui.input_mode = InputMode::ViewMove {
                selected: next,
                ranks: ranks.to_vec(),
            };
            mark_dirty(ui);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let views = ui.app.view_names();
            let next = advance_view_selection(views, selected, -1, |name| {
                crate::app::App::is_move_target_view(name)
            });
            ui.input_mode = InputMode::ViewMove {
                selected: next,
                ranks: ranks.to_vec(),
            };
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let views = ui.app.view_names();
            if let Some(name) = views.get(selected).cloned() {
                if !crate::app::App::is_move_target_view(&name) {
                    ui.app.warning_msg("View cannot be a target");
                    mark_dirty(ui);
                    return;
                }
                let ids = ui.app.ids_for_ranks(ranks);
                if ids.is_empty() {
                    ui.app.warning_msg("No sequences to move");
                    ui.input_mode = InputMode::Normal;
                    mark_dirty(ui);
                    return;
                }
                match ui.app.add_ids_to_view(&name, &ids) {
                    Ok(count) => {
                        if count == 0 {
                            ui.app.info_msg("No sequences moved");
                        } else {
                            ui.app
                                .info_msg(format!("Added {} sequences to {}", count, name));
                        }
                    }
                    Err(e) => ui.app.error_msg(format!("Move failed: {}", e)),
                }
                ui.input_mode = InputMode::Normal;
                mark_dirty(ui);
            }
        }
        _ => {}
    }
}

fn handle_view_create(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let name = editor.text();
            match ui.app.create_view_from_current(&name) {
                Ok(()) => {
                    ui.input_mode = InputMode::Normal;
                    ui.app.info_msg(format!("Created view {}", name));
                }
                Err(e) => {
                    ui.input_mode = InputMode::ViewCreate { editor };
                    ui.app.error_msg(format!("View create failed: {}", e));
                }
            }
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::ViewCreate { editor };
            ui.app
                .argument_msg(String::from("View name: "), ui.view_create_text());
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::ViewCreate { editor };
            ui.app
                .argument_msg(String::from("View name: "), ui.view_create_text());
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::ViewCreate { editor };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::ViewCreate { editor };
            mark_dirty(ui);
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::ViewCreate { editor };
            mark_dirty(ui);
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::ViewCreate { editor };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_view_create_with_list(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let name = editor.text();
            match ui.app.create_view_from_selection(&name) {
                Ok(()) => {
                    ui.input_mode = InputMode::Normal;
                    ui.app.info_msg(format!("Created view {}", name));
                }
                Err(e) => {
                    ui.input_mode = InputMode::ViewCreateWithList { editor };
                    ui.app.error_msg(format!("View create failed: {}", e));
                }
            }
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::ViewCreateWithList { editor };
            ui.app
                .argument_msg(String::from("View name: "), ui.view_create_text());
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::ViewCreateWithList { editor };
            ui.app
                .argument_msg(String::from("View name: "), ui.view_create_text());
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::ViewCreateWithList { editor };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::ViewCreateWithList { editor };
            mark_dirty(ui);
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::ViewCreateWithList { editor };
            mark_dirty(ui);
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::ViewCreateWithList { editor };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_export_svg(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let path = editor.text();
            if path.trim().is_empty() {
                ui.input_mode = InputMode::ExportSvg { editor };
                ui.app.warning_msg("Export path cannot be empty");
                mark_dirty(ui);
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
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::ExportSvg { editor };
            ui.app.argument_msg(String::new(), ui.export_svg_text());
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::ExportSvg { editor };
            ui.app.argument_msg(String::new(), ui.export_svg_text());
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::ExportSvg { editor };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::ExportSvg { editor };
            mark_dirty(ui);
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::ExportSvg { editor };
            mark_dirty(ui);
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::ExportSvg { editor };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_session_save(ui: &mut UI, key_event: KeyEvent, mut editor: LineEditor) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            let path = editor.text();
            if path.is_empty() {
                ui.app.warning_msg("No filename supplied");
                mark_dirty(ui);
                return;
            }
            if std::path::Path::new(&path).exists() {
                ui.input_mode = InputMode::ConfirmSessionOverwrite { editor, path };
                ui.app.argument_msg(
                    String::from("File exists. Overwrite? (y/n) "),
                    String::new(),
                );
            } else {
                match ui.app.save_session(std::path::Path::new(&path)) {
                    Ok(_) => ui.app.info_msg(format!("Session saved -> {}", path)),
                    Err(e) => ui.app.error_msg(format!("Save failed: {}", e)),
                }
                ui.input_mode = InputMode::Normal;
            }
            mark_dirty(ui);
        }
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
            editor.insert_char(c);
            ui.input_mode = InputMode::SessionSave { editor };
            ui.app
                .argument_msg(String::from("Session: "), ui.session_save_text());
            mark_dirty(ui);
        }
        KeyCode::Backspace => {
            editor.backspace();
            ui.input_mode = InputMode::SessionSave { editor };
            ui.app
                .argument_msg(String::from("Session: "), ui.session_save_text());
            mark_dirty(ui);
        }
        KeyCode::Left => {
            editor.move_left();
            ui.input_mode = InputMode::SessionSave { editor };
            mark_dirty(ui);
        }
        KeyCode::Right => {
            editor.move_right();
            ui.input_mode = InputMode::SessionSave { editor };
            mark_dirty(ui);
        }
        KeyCode::Home => {
            editor.move_home();
            ui.input_mode = InputMode::SessionSave { editor };
            mark_dirty(ui);
        }
        KeyCode::End => {
            editor.move_end();
            ui.input_mode = InputMode::SessionSave { editor };
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn handle_confirm_session_overwrite(
    ui: &mut UI,
    key_event: KeyEvent,
    editor: LineEditor,
    path: String,
) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match ui.app.save_session(std::path::Path::new(&path)) {
                Ok(_) => ui.app.info_msg(format!("Session saved -> {}", path)),
                Err(e) => ui.app.error_msg(format!("Save failed: {}", e)),
            }
            ui.input_mode = InputMode::Normal;
            mark_dirty(ui);
        }
        _ => {
            ui.input_mode = InputMode::SessionSave { editor };
            ui.app
                .argument_msg(String::from("Session: "), ui.session_save_text());
            mark_dirty(ui);
        }
    }
}

fn handle_session_list(ui: &mut UI, key_event: KeyEvent, mut selected: usize, files: &[String]) {
    match key_event.code {
        KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        KeyCode::Up => {
            if selected > 0 {
                selected -= 1;
            }
            ui.input_mode = InputMode::SessionList {
                selected,
                files: files.to_vec(),
            };
            mark_dirty(ui);
        }
        KeyCode::Down => {
            if selected + 1 < files.len() {
                selected += 1;
            }
            ui.input_mode = InputMode::SessionList {
                selected,
                files: files.to_vec(),
            };
            mark_dirty(ui);
        }
        KeyCode::Enter => {
            if let Some(name) = files.get(selected) {
                match ui.app.load_session(std::path::Path::new(name)) {
                    Ok(()) => ui.app.info_msg(format!("Loaded session {}", name)),
                    Err(e) => ui.app.error_msg(format!("Load failed: {}", e)),
                }
            }
            ui.input_mode = InputMode::Normal;
            mark_dirty(ui);
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
            mark_dirty(ui);
        }
        _ => {
            ui.input_mode = InputMode::ExportSvg { editor };
            ui.app.argument_msg(String::new(), ui.export_svg_text());
            mark_dirty(ui);
        }
    }
}

fn handle_confirm_reject(ui: &mut UI, key_event: KeyEvent, mode: RejectMode) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            perform_reject(ui, mode);
            mark_dirty(ui);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            ui.app.info_msg("Reject canceled");
            mark_dirty(ui);
        }
        _ => {}
    }
}

fn perform_reject(ui: &mut UI, mode: RejectMode) {
    let out_path = ui.app.rejected_output_path();
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
        mark_dirty(ui);
        return;
    };
    if ranks.is_empty() {
        ui.app.warning_msg("No sequences to reject");
        mark_dirty(ui);
        return;
    }
    match ui.app.reject_sequences(&ranks, &out_path) {
        Ok(result) => handle_reject_result(ui, result),
        Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
    }
    mark_dirty(ui);
}

fn handle_confirm_view_delete(ui: &mut UI, key_event: KeyEvent, name: &str) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match ui.app.delete_view(name) {
                Ok(()) => ui.app.info_msg(format!("Deleted view {}", name)),
                Err(e) => ui.app.error_msg(format!("Delete failed: {}", e)),
            }
            ui.input_mode = InputMode::Normal;
            mark_dirty(ui);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            ui.input_mode = InputMode::Normal;
            ui.app.clear_msg();
            mark_dirty(ui);
        }
        _ => {}
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
            mark_dirty(ui);
        }

        // Bottom pane
        KeyCode::Char('c') => {
            if ui.bottom_pane_height == 0 {
                ui.show_bottom_pane();
            } else {
                ui.hide_bottom_pane();
            }
            mark_dirty(ui);
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
            mark_dirty(ui);
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
            mark_dirty(ui);
        }

        // Up
        KeyCode::Char('k') => {
            match ui.zoom_level() {
                ZoomLevel::ZoomedIn => ui.scroll_one_line_up(count as u16),
                ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                    ui.scroll_zoombox_one_line_up(count as u16)
                }
            }
            mark_dirty(ui);
        }
        KeyCode::Char('K') => {
            ui.scroll_one_screen_up(count as u16);
            mark_dirty(ui);
        }
        KeyCode::Char('g') => {
            ui.jump_to_top();
            mark_dirty(ui);
        }

        // Left
        KeyCode::Char('h') => {
            match ui.zoom_level() {
                ZoomLevel::ZoomedIn => ui.scroll_one_col_left(count as u16),
                ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                    ui.scroll_zoombox_one_col_left(count as u16)
                }
            }
            mark_dirty(ui);
        }
        KeyCode::Char('H') => {
            ui.scroll_one_screen_left(count as u16);
            mark_dirty(ui);
        }
        KeyCode::Char('^') => {
            ui.jump_to_begin();
            mark_dirty(ui);
        }

        // Down
        KeyCode::Char('j') => {
            match ui.zoom_level() {
                ZoomLevel::ZoomedIn => ui.scroll_one_line_down(count as u16),
                ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                    ui.scroll_zoombox_one_line_down(count as u16)
                }
            }
            mark_dirty(ui);
        }
        KeyCode::Char('J') | KeyCode::Char(' ') => {
            ui.scroll_one_screen_down(count as u16);
            mark_dirty(ui);
        }
        KeyCode::Char('G') => {
            ui.jump_to_bottom();
            mark_dirty(ui);
        }

        // Right
        KeyCode::Char('l') => {
            match ui.zoom_level() {
                ZoomLevel::ZoomedIn => ui.scroll_one_col_right(count as u16),
                ZoomLevel::ZoomedOut | ZoomLevel::ZoomedOutAR => {
                    ui.scroll_zoombox_one_col_right(count as u16)
                }
            }
            mark_dirty(ui);
        }
        KeyCode::Char('L') => {
            ui.scroll_one_screen_right(count as u16);
            mark_dirty(ui);
        }
        KeyCode::Char('$') => {
            ui.jump_to_end();
            mark_dirty(ui);
        }

        // Selection
        KeyCode::Char('x') => {
            ui.app.toggle_selection_on_cursor();
            mark_dirty(ui);
        }
        KeyCode::Char('A') => {
            ui.app.select_all_in_view();
            mark_dirty(ui);
        }
        KeyCode::Char('X') => {
            ui.app.clear_selection();
            mark_dirty(ui);
        }

        // Absolute Positions

        // Visible line
        KeyCode::Char('-') => {
            ui.jump_to_line((count as u16) - 1);
            mark_dirty(ui);
        } // -1: user is 1-based

        // Column
        KeyCode::Char('|') => {
            if count_arg.is_some() {
                ui.jump_to_col(count as u16);
                mark_dirty(ui);
            } else {
                let editor = super::notes_editor::NotesEditor::new(ui.app.view_notes());
                ui.input_mode = InputMode::Notes {
                    editor,
                    target: NotesTarget::View,
                };
                mark_dirty(ui);
            }
        }

        // Relative positions

        // Vertical
        KeyCode::Char('%') => {
            ui.jump_to_pct_line(count as u16);
            mark_dirty(ui);
        }

        // Horizontal
        KeyCode::Char('#') => {
            ui.jump_to_pct_col(count as u16);
            mark_dirty(ui);
        }

        // Cursor navigation
        KeyCode::Char('n') => {
            ui.app.move_cursor(count as isize);
            mark_dirty(ui);
        }
        KeyCode::Char('p') => {
            ui.app.move_cursor(-1 * count as isize);
            mark_dirty(ui);
        }
        KeyCode::Char('.') => {
            ui.app.toggle_cursor();
            mark_dirty(ui);
        }
        KeyCode::Char(']') => {
            ui.jump_to_next_seq_match(count as i16);
            mark_dirty(ui);
        }
        KeyCode::Char('[') => {
            ui.jump_to_next_seq_match(-1 * count as i16);
            mark_dirty(ui);
        }

        // Left Pane width
        KeyCode::Char('>') => {
            ui.widen_label_pane(count as u16);
            mark_dirty(ui);
        }
        KeyCode::Char('<') => {
            ui.reduce_label_pane(count as u16);
            mark_dirty(ui);
        }

        // Zoom
        KeyCode::Char('z') => {
            ui.cycle_zoom();
            mark_dirty(ui);
        }
        // Since there are 3 zoom levels, cycling twice amounts to cycling
        // backwards.
        KeyCode::Char('Z') => {
            ui.cycle_zoom();
            ui.cycle_zoom();
            mark_dirty(ui);
        }
        // Toggle zoom box guides
        KeyCode::Char('v') => {
            ui.set_zoombox_guides(!ui.show_zb_guides);
            mark_dirty(ui);
        }
        // Toggle zoom box visibility
        KeyCode::Char('B') => {
            ui.toggle_zoombox();
            mark_dirty(ui);
        }

        // Bottom pane position (i.e., bottom of screen or stuck to the alignment - when both
        // are possible).
        // TODO: not sure we're keeping the "bottom" position. Seems much better to stick it to the
        // last seq in the alignment.
        KeyCode::Char('b') => {
            ui.cycle_bottom_pane_position();
            mark_dirty(ui);
        }

        // ---- Visuals ----

        // Mark consensus positions that are retained in the zoom box
        KeyCode::Char('r') => {
            ui.toggle_hl_retained_cols();
            mark_dirty(ui);
        }

        // Inverse video
        KeyCode::Char('i') => {
            ui.toggle_video_mode();
            mark_dirty(ui);
        }

        KeyCode::Char('s') => {
            ui.next_color_scheme();
            mark_dirty(ui);
        }
        KeyCode::Char('S') => {
            ui.prev_color_scheme();
            mark_dirty(ui);
        }

        // Switch to next/previous colormap in the list
        KeyCode::Char('m') => {
            ui.next_colormap();
            mark_dirty(ui);
        }
        KeyCode::Char('M') => {
            ui.prev_colormap();
            mark_dirty(ui);
        }

        // Sequence Order
        KeyCode::Char('o') => {
            ui.app.next_ordering_criterion();
            mark_dirty(ui);
        }
        KeyCode::Char('O') => {
            ui.app.prev_ordering_criterion();
            mark_dirty(ui);
        }

        // Metric
        KeyCode::Char('t') => {
            ui.app.next_metric();
            mark_dirty(ui);
        }
        KeyCode::Char('T') => {
            ui.app.prev_metric();
            mark_dirty(ui);
        }

        // ----- Search -----
        KeyCode::Char('/') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Regex,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
            mark_dirty(ui);
        }
        KeyCode::Char('\\') => {
            ui.input_mode = InputMode::Search {
                editor: LineEditor::new(),
                kind: SearchKind::Emboss,
            };
            ui.app
                .argument_msg(String::from("Search: "), String::from(""));
            mark_dirty(ui);
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
            mark_dirty(ui);
        }

        // ----- Editing -----
        // Filter alignment through external command (à la Vim's '!')
        KeyCode::Char('!') => {
            let ranks = selected_ranks(ui);
            if ranks.is_empty() {
                ui.app.warning_msg("No selected sequences");
                mark_dirty(ui);
                return;
            }
            let out_path = ui.app.rejected_output_path();
            match ui.app.reject_sequences(&ranks, &out_path) {
                Ok(result) => handle_reject_result(ui, result),
                Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
            }
            mark_dirty(ui);
        }
        KeyCode::Char('W') => {
            let out_path = ui.app.current_view_output_path().to_path_buf();
            match ui.app.write_alignment_fasta(&out_path) {
                Ok(_) => ui.app.info_msg(format!(
                    "View {} -> {}",
                    ui.app.current_view_name(),
                    out_path.display()
                )),
                Err(e) => ui.app.error_msg(format!("Write failed: {}", e)),
            }
            mark_dirty(ui);
        }
        KeyCode::Char(':') => {
            ui.input_mode = InputMode::Command {
                editor: LineEditor::new(),
            };
            ui.app.argument_msg(String::from(":"), String::from(""));
            mark_dirty(ui);
        }
        KeyCode::Char('@') => {
            let editor = super::notes_editor::NotesEditor::new(ui.app.notes());
            ui.input_mode = InputMode::Notes {
                editor,
                target: NotesTarget::Global,
            };
            mark_dirty(ui);
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
