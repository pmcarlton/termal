// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

#[derive(Clone, Debug, PartialEq)]
pub struct LineEditor {
    chars: Vec<char>,
    cursor: usize,
}

impl LineEditor {
    pub fn new() -> Self {
        Self {
            chars: Vec::new(),
            cursor: 0,
        }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn insert_char(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.chars.len();
    }
}

#[cfg(test)]
mod tests {
    use super::LineEditor;

    #[test]
    fn insert_and_backspace() {
        let mut editor = LineEditor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.move_left();
        editor.insert_char('c');
        assert_eq!(editor.text(), "acb");
        editor.backspace();
        assert_eq!(editor.text(), "ab");
    }

    #[test]
    fn move_home_end() {
        let mut editor = LineEditor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.insert_char('c');
        editor.move_home();
        editor.insert_char('z');
        assert_eq!(editor.text(), "zabc");
        editor.move_end();
        editor.insert_char('x');
        assert_eq!(editor.text(), "zabcx");
    }
}
