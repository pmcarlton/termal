// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesEditor {
    lines: Vec<String>,
    row: usize,
    col: usize,
    scroll: usize,
}

impl NotesEditor {
    pub fn new(text: &str) -> Self {
        let mut lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            lines,
            row: 0,
            col: 0,
            scroll: 0,
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn row(&self) -> usize {
        self.row
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    #[allow(dead_code)]
    pub fn set_scroll(&mut self, scroll: usize) {
        self.scroll = scroll;
    }

    pub fn insert_char(&mut self, c: char) {
        let idx = self.col;
        let line = self.current_line_mut();
        let insert_at = idx.min(line.len());
        line.insert(insert_at, c);
        self.col = insert_at + 1;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            let idx = self.col;
            let line = self.current_line_mut();
            let idx = idx.min(line.len());
            if idx > 0 {
                line.remove(idx - 1);
                self.col = idx - 1;
            }
        } else if self.row > 0 {
            let current = self.lines.remove(self.row);
            self.row -= 1;
            let prev_len = self.lines[self.row].len();
            self.lines[self.row].push_str(&current);
            self.col = prev_len;
        }
    }

    pub fn delete_word_left(&mut self) {
        let start = {
            let line = self.current_line();
            if line.is_empty() {
                self.col = 0;
                return;
            }
            let mut idx = self.col.min(line.len());
            while idx > 0 && line.as_bytes()[idx - 1].is_ascii_whitespace() {
                idx -= 1;
            }
            while idx > 0 && !line.as_bytes()[idx - 1].is_ascii_whitespace() {
                idx -= 1;
            }
            idx
        };
        while self.col > start {
            self.backspace();
        }
    }

    pub fn newline(&mut self) {
        let idx = self.col;
        let line = self.current_line_mut();
        let split_at = idx.min(line.len());
        let remainder = line.split_off(split_at);
        self.row += 1;
        self.lines.insert(self.row, remainder);
        self.col = 0;
    }

    pub fn move_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.lines[self.row].len();
        }
    }

    pub fn move_right(&mut self) {
        let len = self.current_line().len();
        if self.col < len {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(self.current_line().len());
        }
    }

    pub fn move_down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(self.current_line().len());
        }
    }

    pub fn move_line_start(&mut self) {
        self.col = 0;
    }

    pub fn move_line_end(&mut self) {
        self.col = self.current_line().len();
    }

    pub fn move_word_left(&mut self) {
        let line = self.current_line();
        if line.is_empty() {
            self.col = 0;
            return;
        }
        let mut idx = self.col.min(line.len());
        while idx > 0 && line.as_bytes()[idx - 1].is_ascii_whitespace() {
            idx -= 1;
        }
        while idx > 0 && !line.as_bytes()[idx - 1].is_ascii_whitespace() {
            idx -= 1;
        }
        self.col = idx;
    }

    pub fn move_word_right(&mut self) {
        let line = self.current_line();
        let len = line.len();
        let mut idx = self.col.min(len);
        while idx < len && !line.as_bytes()[idx].is_ascii_whitespace() {
            idx += 1;
        }
        while idx < len && line.as_bytes()[idx].is_ascii_whitespace() {
            idx += 1;
        }
        self.col = idx;
    }

    pub fn ensure_visible(&mut self, height: usize) {
        if self.row < self.scroll {
            self.scroll = self.row;
        } else if self.row >= self.scroll + height {
            self.scroll = self.row.saturating_sub(height.saturating_sub(1));
        }
    }

    fn current_line(&self) -> &String {
        &self.lines[self.row]
    }

    fn current_line_mut(&mut self) -> &mut String {
        &mut self.lines[self.row]
    }
}

#[cfg(test)]
mod tests {
    use super::NotesEditor;

    #[test]
    fn moves_word_left_and_right() {
        let mut editor = NotesEditor::new("abc def");
        editor.move_line_end();
        editor.move_word_left();
        assert_eq!(editor.col(), 4);
        editor.move_word_left();
        assert_eq!(editor.col(), 0);
        editor.move_word_right();
        assert_eq!(editor.col(), 4);
    }

    #[test]
    fn delete_word_left_removes_word() {
        let mut editor = NotesEditor::new("abc def");
        editor.move_line_end();
        editor.delete_word_left();
        assert_eq!(editor.text(), "abc ");
        editor.delete_word_left();
        assert_eq!(editor.text(), "");
    }
}
