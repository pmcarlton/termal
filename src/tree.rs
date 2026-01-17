// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Peter Carlton
// Modifications (c) 2026 Peter Carlton

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
    leaf_start: usize,
    leaf_end: usize,
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
    tree_lines_and_order_with_selection(root, None)
}

pub fn tree_lines_and_order_with_selection(
    root: &TreeNode,
    selection: Option<(usize, usize)>,
) -> Result<(Vec<String>, Vec<String>), TermalError> {
    let mut root = collapse_unary(root.clone());
    let (node_map, leaves) = assign_rows_and_depths(&mut root);
    if leaves.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let lines = render_box_tree(&root, &node_map, &leaves, selection);
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
    let mut next_leaf = 0;

    fn walk(
        node: &TreeNode,
        depth: usize,
        next_y: &mut usize,
        next_leaf: &mut usize,
        node_map: &mut std::collections::HashMap<usize, NodeInfo>,
        leaves: &mut Vec<(usize, String)>,
    ) -> (usize, usize, usize) {
        if node.children.is_empty() {
            let y = *next_y;
            *next_y += 1;
            let leaf_idx = *next_leaf;
            *next_leaf += 1;
            node_map.insert(
                node as *const _ as usize,
                NodeInfo {
                    depth,
                    y,
                    leaf_start: leaf_idx,
                    leaf_end: leaf_idx,
                },
            );
            leaves.push((y, node.name.clone().unwrap_or_default()));
            return (y, leaf_idx, leaf_idx);
        }
        let child_infos: Vec<(usize, usize, usize)> = node
            .children
            .iter()
            .map(|child| walk(child, depth + 1, next_y, next_leaf, node_map, leaves))
            .collect();
        let y_top = child_infos.iter().map(|(y, _, _)| *y).min().unwrap();
        let y_bottom = child_infos.iter().map(|(y, _, _)| *y).max().unwrap();
        let y = (y_top + y_bottom) / 2;
        let leaf_start = child_infos.iter().map(|(_, s, _)| *s).min().unwrap();
        let leaf_end = child_infos.iter().map(|(_, _, e)| *e).max().unwrap();
        node_map.insert(
            node as *const _ as usize,
            NodeInfo {
                depth,
                y,
                leaf_start,
                leaf_end,
            },
        );
        (y, leaf_start, leaf_end)
    }

    walk(
        root,
        0,
        &mut next_y,
        &mut next_leaf,
        &mut node_map,
        &mut leaves,
    );
    (node_map, leaves)
}

fn render_box_tree(
    root: &TreeNode,
    node_map: &std::collections::HashMap<usize, NodeInfo>,
    leaves: &[(usize, String)],
    selection: Option<(usize, usize)>,
) -> Vec<String> {
    let n_rows = leaves.iter().map(|(y, _)| *y).max().unwrap_or(0) + 1;
    let max_depth = node_map.values().map(|info| info.depth).max().unwrap_or(0);
    let tree_width = max_depth * 2 + 1;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; tree_width]; n_rows];

    fn to_heavy(ch: char) -> char {
        match ch {
            '─' => '━',
            '│' => '┃',
            '┌' => '┏',
            '└' => '┗',
            '├' => '┣',
            '┤' => '┫',
            '┬' => '┳',
            '┴' => '┻',
            '┼' => '╋',
            other => other,
        }
    }

    fn is_horizontal(ch: char) -> bool {
        matches!(ch, '─' | '━')
    }

    fn is_vertical(ch: char) -> bool {
        matches!(ch, '│' | '┃')
    }

    fn is_heavy(ch: char) -> bool {
        matches!(ch, '━' | '┃' | '┏' | '┗' | '┣' | '┫' | '┳' | '┻' | '╋')
    }

    fn put(grid: &mut [Vec<char>], y: usize, x: usize, ch: char, heavy: bool) {
        if y >= grid.len() || x >= grid[y].len() {
            return;
        }
        let ch = if heavy { to_heavy(ch) } else { ch };
        let existing = grid[y][x];
        if existing == ' ' {
            grid[y][x] = ch;
            return;
        }
        let junctions = ['┌', '└', '├', '┼', '┏', '┗', '┣', '╋'];
        if junctions.contains(&existing) && !is_heavy(ch) {
            return;
        }
        if junctions.contains(&ch) {
            grid[y][x] = ch;
            return;
        }
        if (is_vertical(existing) && is_horizontal(ch))
            || (is_horizontal(existing) && is_vertical(ch))
        {
            grid[y][x] = if is_heavy(existing) || is_heavy(ch) {
                '╋'
            } else {
                '┼'
            };
            return;
        }
        if is_heavy(ch) {
            grid[y][x] = ch;
        }
    }

    fn draw_internal(
        node: &TreeNode,
        node_map: &std::collections::HashMap<usize, NodeInfo>,
        grid: &mut [Vec<char>],
        selection: Option<(usize, usize)>,
    ) {
        let info = node_map[&(node as *const _ as usize)];
        if node.children.is_empty() {
            return;
        }
        let parent_selected = selection
            .map(|(start, end)| start <= info.leaf_start && end >= info.leaf_end)
            .unwrap_or(false);
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
            put(grid, y, x_conn, '│', parent_selected);
        }

        for (kid, ki) in node.children.iter().zip(kid_infos.iter()) {
            let y = ki.y;
            let child_selected = selection
                .map(|(start, end)| start <= ki.leaf_start && end >= ki.leaf_end)
                .unwrap_or(false);
            let jch = if y == y_top && y != y_bottom {
                '┌'
            } else if y == y_bottom && y != y_top {
                '└'
            } else {
                '├'
            };
            put(grid, y, x_conn, jch, child_selected);
            let x_child = ki.depth * 2;
            for x in (x_conn + 1)..=x_child {
                put(grid, y, x, '─', child_selected);
            }
            draw_internal(kid, node_map, grid, selection);
        }
    }

    draw_internal(root, node_map, &mut grid, selection);

    let leaf_rows: std::collections::HashMap<usize, usize> = leaves
        .iter()
        .enumerate()
        .map(|(idx, (y, _))| (*y, idx))
        .collect();
    for (y, leaf_idx) in leaf_rows {
        let row = &grid[y];
        let mut last = None;
        for x in (0..row.len()).rev() {
            if row[x] != ' ' {
                last = Some(x);
                break;
            }
        }
        let start = last.map(|l| l + 1).unwrap_or(0);
        let leaf_selected = selection
            .map(|(start, end)| start <= leaf_idx && end >= leaf_idx)
            .unwrap_or(false);
        for x in start..tree_width {
            put(&mut grid, y, x, '─', leaf_selected);
        }
    }

    for row in &mut grid {
        for x in 1..row.len() {
            if is_horizontal(row[x - 1]) {
                row[x] = match row[x] {
                    '│' => '┤',
                    '┌' => '┬',
                    '└' => '┴',
                    '├' => '┼',
                    '┃' => '┫',
                    '┏' => '┳',
                    '┗' => '┻',
                    '┣' => '╋',
                    other => other,
                };
                if is_heavy(row[x - 1]) {
                    row[x] = to_heavy(row[x]);
                }
            }
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
