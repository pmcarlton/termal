// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use crate::errors::TermalError;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Copy)]
struct NodeInfo {
    depth: usize,
    y: usize,
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
    let mut root = collapse_unary(root.clone());
    let (node_map, leaves) = assign_rows_and_depths(&mut root);
    if leaves.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let lines = render_box_tree(&root, &node_map, &leaves);
    let order: Vec<String> = leaves.iter().map(|(_, name)| name.clone()).collect();
    for name in &order {
        if name.is_empty() {
            return Err(TermalError::Format(String::from("Missing leaf name")));
        }
    }
    Ok((lines, order))
}

fn collapse_unary(mut node: TreeNode) -> TreeNode {
    while node.children.len() == 1 {
        let mut child = node.children.remove(0);
        if node.name.is_some() && child.name.is_none() {
            child.name = node.name.take();
        }
        node = child;
    }
    if !node.children.is_empty() {
        node.children = node.children.into_iter().map(collapse_unary).collect();
    }
    node
}

fn assign_rows_and_depths(
    root: &TreeNode,
) -> (
    std::collections::HashMap<usize, NodeInfo>,
    Vec<(usize, String)>,
) {
    let mut node_map = std::collections::HashMap::new();
    let mut leaves = Vec::new();
    let mut next_y = 0;

    fn walk(
        node: &TreeNode,
        depth: usize,
        next_y: &mut usize,
        node_map: &mut std::collections::HashMap<usize, NodeInfo>,
        leaves: &mut Vec<(usize, String)>,
    ) -> usize {
        if node.children.is_empty() {
            let y = *next_y;
            *next_y += 1;
            node_map.insert(node as *const _ as usize, NodeInfo { depth, y });
            leaves.push((y, node.name.clone().unwrap_or_default()));
            return y;
        }
        let child_ys: Vec<usize> = node
            .children
            .iter()
            .map(|child| walk(child, depth + 1, next_y, node_map, leaves))
            .collect();
        let y = (child_ys.iter().min().unwrap() + child_ys.iter().max().unwrap()) / 2;
        node_map.insert(node as *const _ as usize, NodeInfo { depth, y });
        y
    }

    walk(root, 0, &mut next_y, &mut node_map, &mut leaves);
    (node_map, leaves)
}

fn render_box_tree(
    root: &TreeNode,
    node_map: &std::collections::HashMap<usize, NodeInfo>,
    leaves: &[(usize, String)],
) -> Vec<String> {
    let n_rows = leaves.iter().map(|(y, _)| *y).max().unwrap_or(0) + 1;
    let max_depth = node_map.values().map(|info| info.depth).max().unwrap_or(0);
    let tree_width = max_depth * 2;
    let label_start = tree_width + 2;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; label_start]; n_rows];

    fn put(grid: &mut [Vec<char>], y: usize, x: usize, ch: char) {
        if y >= grid.len() || x >= grid[y].len() {
            return;
        }
        let existing = grid[y][x];
        if existing == ' ' {
            grid[y][x] = ch;
            return;
        }
        let junctions = ['┌', '└', '├', '┼'];
        if junctions.contains(&existing) {
            return;
        }
        if junctions.contains(&ch) {
            grid[y][x] = ch;
            return;
        }
        if (existing == '│' && ch == '─') || (existing == '─' && ch == '│') {
            grid[y][x] = '┼';
        }
    }

    fn draw_internal(
        node: &TreeNode,
        node_map: &std::collections::HashMap<usize, NodeInfo>,
        grid: &mut [Vec<char>],
    ) {
        let info = node_map[&(node as *const _ as usize)];
        if node.children.is_empty() {
            return;
        }
        let x_node = info.depth * 2;
        let x_conn = x_node + 1;
        let kid_infos: Vec<NodeInfo> = node
            .children
            .iter()
            .map(|kid| node_map[&(kid as *const _ as usize)])
            .collect();
        let ys: Vec<usize> = kid_infos.iter().map(|k| k.y).collect();
        let y_top = *ys.iter().min().unwrap();
        let y_bottom = *ys.iter().max().unwrap();

        for y in (y_top + 1)..y_bottom {
            put(grid, y, x_conn, '│');
        }

        for (kid, ki) in node.children.iter().zip(kid_infos.iter()) {
            let y = ki.y;
            let jch = if y == y_top && y != y_bottom {
                '┌'
            } else if y == y_bottom && y != y_top {
                '└'
            } else {
                '├'
            };
            put(grid, y, x_conn, jch);
            let x_child = ki.depth * 2;
            for x in (x_conn + 1)..=x_child {
                put(grid, y, x, '─');
            }
            draw_internal(kid, node_map, grid);
        }
    }

    draw_internal(root, node_map, &mut grid);

    let leaf_rows: std::collections::HashSet<usize> = leaves.iter().map(|(y, _)| *y).collect();
    for y in leaf_rows {
        let row = &grid[y];
        let mut last = None;
        for x in (0..row.len()).rev() {
            if row[x] != ' ' {
                last = Some(x);
                break;
            }
        }
        let start = last.map(|l| l + 1).unwrap_or(0);
        for x in start..label_start {
            put(&mut grid, y, x, '─');
        }
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>().trim_end().to_string())
        .collect()
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
