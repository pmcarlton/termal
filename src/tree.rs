// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use crate::errors::TermalError;

#[derive(Debug)]
pub struct TreeNode {
    pub name: Option<String>,
    pub children: Vec<TreeNode>,
}

pub fn parse_newick(input: &str) -> Result<TreeNode, TermalError> {
    let mut parser = Parser::new(input);
    let node = parser.parse_node()?;
    parser.skip_whitespace();
    if parser.peek() == Some(';') {
        parser.pos += 1;
    }
    Ok(node)
}

pub fn tree_lines_and_order(root: &TreeNode) -> Result<(Vec<String>, Vec<String>), TermalError> {
    let mut lines: Vec<String> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    build_lines(root, String::new(), &mut lines, &mut order);
    for name in &order {
        if name.is_empty() {
            return Err(TermalError::Format(String::from("Missing leaf name")));
        }
    }
    Ok((lines, order))
}

fn build_lines(node: &TreeNode, prefix: String, lines: &mut Vec<String>, order: &mut Vec<String>) {
    if node.children.is_empty() {
        let name = node.name.clone().unwrap_or_default();
        lines.push(format!("{}└─{}", prefix, name));
        order.push(name);
        return;
    }
    let count = node.children.len();
    for (idx, child) in node.children.iter().enumerate() {
        let last = idx + 1 == count;
        let branch = if last { "└─" } else { "├─" };
        let child_prefix = format!("{}{}", prefix, if last { "  " } else { "│ " });
        if child.children.is_empty() {
            let name = child.name.clone().unwrap_or_default();
            lines.push(format!("{}{}{}", prefix, branch, name));
            order.push(name);
        } else {
            build_lines(child, child_prefix, lines, order);
        }
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn parse_node(&mut self) -> Result<TreeNode, TermalError> {
        self.skip_whitespace();
        if self.peek() == Some('(') {
            self.pos += 1;
            let mut children = Vec::new();
            loop {
                let child = self.parse_node()?;
                children.push(child);
                self.skip_whitespace();
                match self.peek() {
                    Some(',') => {
                        self.pos += 1;
                    }
                    Some(')') => {
                        self.pos += 1;
                        break;
                    }
                    _ => {
                        return Err(TermalError::Format(String::from("Malformed Newick tree")));
                    }
                }
            }
            let name = self.parse_name_opt();
            self.skip_branch_length();
            Ok(TreeNode { name, children })
        } else {
            let name = self.parse_name()?;
            self.skip_branch_length();
            Ok(TreeNode {
                name: Some(name),
                children: Vec::new(),
            })
        }
    }

    fn parse_name_opt(&mut self) -> Option<String> {
        self.skip_whitespace();
        match self.peek() {
            Some(':' | ',' | ')' | ';') | None => None,
            _ => self.parse_name().ok(),
        }
    }

    fn parse_name(&mut self) -> Result<String, TermalError> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if matches!(c, ':' | ',' | ')' | '(' | ';') || c.is_whitespace() {
                break;
            }
            self.pos += 1;
        }
        if start == self.pos {
            return Err(TermalError::Format(String::from("Missing node name")));
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn skip_branch_length(&mut self) {
        self.skip_whitespace();
        if self.peek() != Some(':') {
            return;
        }
        self.pos += 1;
        while let Some(c) = self.peek() {
            if matches!(c, ',' | ')' | ';') || c.is_whitespace() {
                break;
            }
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_newick_leaf_order() {
        let tree = parse_newick("(A,(B,C));").unwrap();
        let (_lines, order) = tree_lines_and_order(&tree).unwrap();
        assert_eq!(order, vec!["A", "B", "C"]);
    }
}
