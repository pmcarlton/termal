// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier
// Modifications (c) 2026 Peter Carlton
mod aln_widget;
mod barchart;
pub mod color_map;
mod color_scheme;
pub mod key_handling;
mod line_editor;
mod msg_theme;
mod notes_editor;
pub mod render;
mod style;
mod svg;
mod zoombox;

use std::{
    cmp::{max, min},
    fmt,
    path::Path,
};

use bitflags::bitflags;

use ratatui::layout::Size;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

use self::{
    aln_widget::{SearchHighlight, SearchHighlightConfig},
    color_map::colormap_gecos,
    color_scheme::{ColorScheme, Theme},
    line_editor::LineEditor,
    notes_editor::NotesEditor,
};

use crate::{
    app::{App, SearchKind, SeqOrdering},
    errors::TermalError,
    tree::TreeNode,
};

const V_SCROLLBAR_WIDTH: u16 = 1;
const MIN_COLS_SHOWN: u16 = 1;
const BORDER_WIDTH: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZoomLevel {
    ZoomedIn,
    ZoomedOut,
    ZoomedOutAR,
}

#[derive(Debug)]
enum BottomPanePosition {
    Adjacent,
    ScreenBottom,
}

#[derive(Clone, Copy, PartialEq)]
enum VideoMode {
    Direct,
    Inverse,
}

#[derive(Clone, PartialEq)]
enum InputMode {
    Normal,
    Help,
    PendingCount {
        count: usize,
    },
    LabelSearch {
        pattern: String,
    },
    Search {
        editor: LineEditor,
        kind: SearchKind,
    },
    Command {
        editor: LineEditor,
    },
    ExportSvg {
        editor: LineEditor,
    },
    ConfirmOverwrite {
        editor: LineEditor,
        path: String,
    },
    SessionSave {
        editor: LineEditor,
    },
    ConfirmSessionOverwrite {
        editor: LineEditor,
        path: String,
    },
    SearchList {
        selected: usize,
    },
    SessionList {
        selected: usize,
        files: Vec<String>,
    },
    Notes {
        editor: NotesEditor,
        target: NotesTarget,
    },
    ConfirmReject {
        mode: RejectMode,
    },
    ConfirmViewDelete {
        name: String,
    },
    ViewList {
        selected: usize,
    },
    ViewCreate {
        editor: LineEditor,
    },
    ViewCreateWithList {
        editor: LineEditor,
    },
    ViewDelete {
        selected: usize,
    },
    ViewMove {
        selected: usize,
        ranks: Vec<usize>,
    },
    TreeNav {
        nav: TreeNav,
    },
    // ExCommand { buffer: String },
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum NotesTarget {
    Global,
    View,
}

#[derive(Clone, Copy, PartialEq)]
enum RejectMode {
    Current,
    Unmatched,
    Matched,
}

#[derive(Clone, PartialEq)]
struct TreeNavNode {
    parent: Option<usize>,
    children: Vec<usize>,
    depth: usize,
    leaf_range: (usize, usize),
}

#[derive(Clone, PartialEq)]
struct TreeNav {
    nodes: Vec<TreeNavNode>,
    depth_nodes: Vec<Vec<usize>>,
    current: usize,
    leaf_names: Vec<String>,
    leaf_ranks: Vec<usize>,
}

impl TreeNav {
    fn selected_leaf_range(&self) -> (usize, usize) {
        self.nodes[self.current].leaf_range
    }

    fn selected_leaf_ranks(&self) -> Vec<usize> {
        let (start, end) = self.selected_leaf_range();
        self.leaf_ranks[start..=end].to_vec()
    }

    fn move_right(&mut self) -> bool {
        let children = &self.nodes[self.current].children;
        if children.is_empty() {
            return false;
        }
        let mut best = children[0];
        let mut best_start = self.nodes[best].leaf_range.0;
        for child in children.iter().skip(1) {
            let start = self.nodes[*child].leaf_range.0;
            if start < best_start {
                best = *child;
                best_start = start;
            }
        }
        self.current = best;
        true
    }

    fn move_left(&mut self) -> bool {
        if let Some(parent) = self.nodes[self.current].parent {
            self.current = parent;
            return true;
        }
        false
    }

    fn move_down(&mut self) -> bool {
        self.move_level(1)
    }

    fn move_up(&mut self) -> bool {
        self.move_level(-1)
    }

    fn move_level(&mut self, delta: isize) -> bool {
        let depth = self.nodes[self.current].depth;
        let level = match self.depth_nodes.get(depth) {
            Some(level) if !level.is_empty() => level,
            _ => return false,
        };
        let pos = level
            .iter()
            .position(|node_id| *node_id == self.current)
            .unwrap_or(0) as isize;
        let len = level.len() as isize;
        let next = (pos + delta).rem_euclid(len) as usize;
        self.current = level[next];
        true
    }
}

fn build_tree_nav(app: &App, tree: &TreeNode) -> Result<TreeNav, TermalError> {
    let mut nodes: Vec<TreeNavNode> = Vec::new();
    let mut depth_nodes: Vec<Vec<usize>> = Vec::new();
    let mut leaf_names: Vec<String> = Vec::new();

    fn walk(
        node: &TreeNode,
        parent: Option<usize>,
        depth: usize,
        nodes: &mut Vec<TreeNavNode>,
        depth_nodes: &mut Vec<Vec<usize>>,
        leaf_names: &mut Vec<String>,
    ) -> (usize, usize, usize) {
        let id = nodes.len();
        nodes.push(TreeNavNode {
            parent,
            children: Vec::new(),
            depth,
            leaf_range: (0, 0),
        });
        if depth_nodes.len() <= depth {
            depth_nodes.push(Vec::new());
        }
        depth_nodes[depth].push(id);
        let start = leaf_names.len();
        if node.children.is_empty() {
            leaf_names.push(node.name.clone().unwrap_or_default());
            nodes[id].leaf_range = (start, start);
            return (id, start, start);
        }
        let mut child_ids = Vec::new();
        let mut end = start;
        for child in &node.children {
            let (child_id, _cstart, cend) =
                walk(child, Some(id), depth + 1, nodes, depth_nodes, leaf_names);
            child_ids.push(child_id);
            end = end.max(cend);
        }
        nodes[id].children = child_ids;
        nodes[id].leaf_range = (start, end);
        (id, start, end)
    }

    walk(tree, None, 0, &mut nodes, &mut depth_nodes, &mut leaf_names);
    for level in &mut depth_nodes {
        level.sort_by_key(|node_id| nodes[*node_id].leaf_range.0);
    }
    let leaf_ranks = app.map_tree_leaf_ranks(&leaf_names)?;
    Ok(TreeNav {
        nodes,
        depth_nodes,
        current: 0,
        leaf_names,
        leaf_ranks,
    })
}

pub const USER_GUIDE: &str = include_str!("ui/bindings.md");

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum LabelSearchDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum SearchDirection {
    Forward,
    Backward,
    Up,
    Down,
}

impl fmt::Display for VideoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            VideoMode::Direct => "Dir",
            VideoMode::Inverse => "Inv",
        };
        write!(f, "{}", s)
    }
}

// A bit field that denotes if the alignment is too wide (with respect to the sequence panel), too
// tall, both, or neither.

bitflags! {
    #[derive(PartialEq)]
    pub struct AlnWRTSeqPane: u8 {
        const Fits           = 0b00;
        const TooTall        = 0b01;
        const TooWide        = 0b10;
        const TooTallAndWide = 0b11;
    }
}

pub struct UI<'a> {
    app: &'a mut App,
    color_schemes: Vec<ColorScheme>,
    current_color_scheme_index: usize,
    zoom_level: ZoomLevel,
    show_zoombox: bool,
    //zoombox_color: Style,
    show_zb_guides: bool,
    show_scrollbars: bool,
    highlight_retained_cols: bool,
    top_line: u16,
    leftmost_col: u16,
    left_pane_width: u16,
    previous_left_pane_width: u16, // To restore width after hiding pane
    bottom_pane_height: u16,
    previous_bottom_pane_height: u16,
    bottom_pane_position: BottomPanePosition,
    // These cannot be known when the structure is initialized, so they are Options -- but it is
    // possible that they need not be stored at all, as they can in principle be computed when the
    // layout is known.
    aln_pane_size: Option<Size>,
    frame_size: Option<Size>, // whole app
    full_screen: bool,
    video_mode: VideoMode,
    input_mode: InputMode,
    help_scroll: usize,
    help_page_height: usize,
    exit_message: Option<String>,
    show_tree_panel: bool,
    dirty: bool,
}

impl<'a> UI<'a> {
    pub fn new(app: &'a mut App) -> Self {
        let macromolecule_type = app.alignment.macromolecule_type();
        app.info_msg("Press '?' for help");
        let color_schemes = vec![
            ColorScheme::color_scheme_dark(macromolecule_type),
            ColorScheme::color_scheme_light(macromolecule_type),
            ColorScheme::color_scheme_monochrome(),
        ];
        let default_color_scheme_index = color_schemes.len() - 1;
        UI {
            app,
            color_schemes,
            current_color_scheme_index: default_color_scheme_index,
            zoom_level: ZoomLevel::ZoomedIn,
            show_zoombox: true,
            show_zb_guides: true,
            show_scrollbars: true,
            highlight_retained_cols: false,
            top_line: 0,
            leftmost_col: 0,
            left_pane_width: 18, // Reasonable default, I'd say...
            previous_left_pane_width: 0,
            bottom_pane_height: 5,
            previous_bottom_pane_height: 0,
            bottom_pane_position: BottomPanePosition::Adjacent,
            aln_pane_size: None,
            frame_size: None,
            full_screen: false,
            video_mode: VideoMode::Direct,
            input_mode: InputMode::Normal,
            help_scroll: 0,
            help_page_height: 1,
            exit_message: None,
            show_tree_panel: false,
            dirty: false,
        }
    }

    pub fn reset_help_scroll(&mut self) {
        self.help_scroll = 0;
    }

    pub fn help_scroll_by(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        let cur = self.help_scroll as isize;
        let next = (cur + delta).max(0);
        self.help_scroll = next as usize;
    }

    pub fn help_page_height(&self) -> usize {
        self.help_page_height.max(1)
    }

    pub fn set_exit_message(&mut self, msg: impl Into<String>) {
        self.exit_message = Some(msg.into());
    }

    pub fn has_exit_message(&self) -> bool {
        self.exit_message.is_some()
    }

    pub fn take_exit_message(&mut self) -> Option<String> {
        self.exit_message.take()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn take_dirty(&mut self) -> bool {
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

    pub fn show_tree_panel(&mut self, show: bool) {
        self.show_tree_panel = show;
    }

    pub fn set_user_ordering_from_headers(&mut self) -> Result<(), TermalError> {
        let headers = self.app.alignment.headers.clone();
        self.app.set_user_ordering(headers)
    }

    pub fn sync_tree_panel_with_ordering(&mut self) {
        if !self.app.has_tree_panel() {
            self.show_tree_panel = false;
            return;
        }
        self.show_tree_panel = matches!(self.app.get_seq_ordering(), SeqOrdering::User);
    }

    pub fn toggle_tree_panel(&mut self) {
        self.show_tree_panel = !self.show_tree_panel;
    }

    pub fn is_tree_panel_visible(&self) -> bool {
        self.show_tree_panel && self.app.has_tree_panel()
    }

    pub fn num_sequences(&self) -> u16 {
        self.app.num_seq()
    }

    pub fn cursor_rank(&self) -> Option<usize> {
        self.app.cursor_rank()
    }

    pub fn selection_len(&self) -> usize {
        self.app.selection_ranks().len()
    }

    pub fn select_label_by_rank(&mut self, rank: usize) -> Result<(), TermalError> {
        self.app.select_label_by_rank(rank)?;
        if let Some(line) = self.app.current_label_match_screenlinenum() {
            self.jump_to_line(line as u16);
        }
        Ok(())
    }

    pub fn export_svg(&mut self, path: &Path) -> Result<(), TermalError> {
        svg::export_current_view(self, path)
    }

    pub fn frame_size(&self) -> Option<Size> {
        self.frame_size
    }

    // ****************************************************************
    /*
     * Dimensions
     *
     * The layout determines the maximal number of sequences and columns shown; this in turn
     * affects the maximal top line and leftmost column, etc.
     * */

    fn max_nb_seq_shown(&self) -> u16 {
        let height = self.aln_pane_size.unwrap().height;
        height.saturating_sub(2) // Borders - TODO: use constants!
    }

    pub fn visible_seq_rows(&self) -> u16 {
        self.max_nb_seq_shown()
    }

    fn max_nb_col_shown(&self) -> u16 {
        let width = self.aln_pane_size.unwrap().width;
        width.saturating_sub(2) // Borders - TODO: use constants!
    }

    // Resizing (as when the user resizes the terminal window where Termal runs) affects
    // max_top_line and max_leftmost_col (because the number of available lines (resp. columns)
    // will generally change), so top_line and leftmost_col may now exceed them. This function,
    // which should be called after the layout is solved but before the widgets are drawn, makes
    // sure that l_max corresponds to the size of the alignment panel, so that l does not exceed
    // l_max, etc.

    pub fn adjust_seq_pane_position(&mut self) {
        if self.leftmost_col > self.max_leftmost_col() {
            self.leftmost_col = self.max_leftmost_col();
        }
        if self.top_line > self.max_top_line() {
            self.top_line = self.max_top_line();
        }
    }

    /****************************************************************/
    // Location within the alignment

    pub fn top_line(&self) -> u16 {
        self.top_line
    }
    pub fn leftmost_col(&self) -> u16 {
        self.leftmost_col
    }

    // FIXME: use saturating arithmetic (also next fn)
    pub fn max_top_line(&self) -> u16 {
        if self.app.num_seq() >= self.max_nb_seq_shown() {
            self.app.num_seq() - self.max_nb_seq_shown()
        } else {
            0
        }
    }

    pub fn max_leftmost_col(&self) -> u16 {
        if self.app.aln_len() >= self.max_nb_col_shown() {
            self.app.aln_len() - self.max_nb_col_shown()
        } else {
            0
        }
    }

    // Side panel dimensions

    pub fn set_left_pane_width(&mut self, width: u16) {
        self.left_pane_width = width;
    }

    // Also stores previous width
    pub fn hide_label_pane(&mut self) {
        self.previous_left_pane_width = self.left_pane_width;
        self.left_pane_width = 0;
    }

    pub fn show_label_pane(&mut self) {
        self.left_pane_width = self.previous_left_pane_width;
    }

    // Number of columns needed to write the highest sequence number, e.g. 4 for 1000. This does
    // NOT take into account any borders.
    pub fn seq_num_max_len(&self) -> u16 {
        self.app.num_seq().ilog10() as u16 + 1
    }

    // Width of seq num pane, which is the length of the longest seq num + border width (1).
    // TODO: Express border width as a constant
    pub fn seq_num_pane_width(&self) -> u16 {
        self.seq_num_max_len() + 1
    }

    pub fn widen_label_pane(&mut self, amount: u16) {
        self.left_pane_width = min(
            self.left_pane_width + amount,
            self.frame_size.unwrap().width - (V_SCROLLBAR_WIDTH + MIN_COLS_SHOWN + BORDER_WIDTH),
        );
    }

    pub fn reduce_label_pane(&mut self, amount: u16) {
        self.left_pane_width = max(
            self.seq_num_pane_width() + self.metric_pane_width(),
            self.left_pane_width.saturating_sub(amount),
        );
    }

    pub fn metric_pane_width(&self) -> u16 {
        // Two chars for the histogram, and one for the border
        3
    }

    // Bottom pane dimensions

    pub fn set_bottom_pane_height(&mut self, height: u16) {
        self.bottom_pane_height = height;
    }

    pub fn hide_bottom_pane(&mut self) {
        self.previous_bottom_pane_height = self.bottom_pane_height;
        self.bottom_pane_height = 0;
    }

    pub fn show_bottom_pane(&mut self) {
        self.bottom_pane_height = 5;
    }

    // ****************************************************************
    // Zooming

    // Determines if the alignment fits on the screen or is too wide or tall (it can be both).
    // TODO: might be an inner function of cycle_zoom, as it is not used anywhere else.
    fn aln_wrt_seq_pane(&self) -> AlnWRTSeqPane {
        let mut rel = AlnWRTSeqPane::Fits;
        if self.app.aln_len() > self.max_nb_col_shown() {
            rel |= AlnWRTSeqPane::TooWide;
        }
        if self.app.num_seq() > self.max_nb_seq_shown() {
            rel |= AlnWRTSeqPane::TooTall;
        }

        rel
    }

    // TODO: is this accessor needed?
    pub fn zoom_level(&self) -> ZoomLevel {
        self.zoom_level
    }

    pub fn cycle_zoom(&mut self) {
        self.zoom_level = match self.zoom_level {
            ZoomLevel::ZoomedIn => {
                // ZoomedOut, unless alignment fits
                if self.aln_wrt_seq_pane() == AlnWRTSeqPane::Fits {
                    ZoomLevel::ZoomedIn
                } else {
                    ZoomLevel::ZoomedOut
                }
            }
            ZoomLevel::ZoomedOut => ZoomLevel::ZoomedOutAR,
            ZoomLevel::ZoomedOutAR => ZoomLevel::ZoomedIn,
        }
    }

    pub fn h_ratio(&self) -> f64 {
        self.max_nb_col_shown() as f64 / self.app.aln_len() as f64
    }

    pub fn v_ratio(&self) -> f64 {
        self.max_nb_seq_shown() as f64 / self.app.num_seq() as f64
    }

    // ZoomLevel::ZoomedOutAR mode uses a _single_ ratio, which is usually the minimum of the
    // vertical and horizontal ratios, but it _can_ use the mmaximum if the resulting alignment
    // still fits.
    pub fn common_ratio(&self) -> f64 {
        let min_ratio = self.h_ratio().min(self.v_ratio());
        let max_ratio = self.h_ratio().max(self.v_ratio());
        let max_r_cols = (self.app.aln_len() as f64 * max_ratio).floor() as u16;
        let max_r_seqs = (self.app.num_seq() as f64 * max_ratio).floor() as u16;

        if max_r_cols == self.max_nb_col_shown() && max_r_seqs == self.max_nb_seq_shown() {
            max_ratio
        } else {
            min_ratio
        }
    }

    pub fn set_zoombox(&mut self, state: bool) {
        self.show_zoombox = state;
    }

    pub fn toggle_zoombox(&mut self) {
        self.show_zoombox = !self.show_zoombox;
    }

    pub fn zoombox_top(&self) -> usize {
        match self.zoom_level {
            ZoomLevel::ZoomedOut => ((self.top_line as f64) * self.v_ratio()).floor() as usize,
            ZoomLevel::ZoomedOutAR => {
                let ratio = self.common_ratio();
                let mut zb_top = ((self.top_line as f64) * ratio).floor() as usize;
                if zb_top >= self.max_nb_seq_shown() as usize {
                    zb_top = (self.max_nb_seq_shown() as usize).saturating_sub(1);
                }
                zb_top
            }
            _ => panic!(
                "zoombox_top() should not be called in {:?} mode\n",
                self.zoom_level
            ),
        }
    }

    pub fn zoombox_bottom(&self, seq_para_len: usize) -> usize {
        match self.zoom_level {
            ZoomLevel::ZoomedOut => {
                let mut zb_bottom: usize = (((self.top_line + self.max_nb_seq_shown()) as f64)
                    * self.v_ratio())
                .round() as usize;
                // If h_a < h_p
                if zb_bottom > self.app.num_seq() as usize {
                    zb_bottom = self.app.num_seq() as usize;
                }
                zb_bottom
            }
            ZoomLevel::ZoomedOutAR => {
                let ratio = self.common_ratio();
                let aln_para_height = min(seq_para_len as u16, self.max_nb_seq_shown());
                let mut zb_bottom =
                    (((self.top_line + self.max_nb_seq_shown()) as f64) * ratio).round() as usize;
                // If h_a < h_p
                if zb_bottom > aln_para_height as usize {
                    zb_bottom = aln_para_height as usize;
                }
                zb_bottom
            }
            _ => panic!(
                "zoombox_bottom() should not be called in {:?} mode\n",
                self.zoom_level
            ),
        }
    }

    pub fn zoombox_left(&self) -> usize {
        match self.zoom_level {
            ZoomLevel::ZoomedOut => ((self.leftmost_col as f64) * self.h_ratio()).floor() as usize,
            ZoomLevel::ZoomedOutAR => {
                let ratio = self.common_ratio();
                ((self.leftmost_col as f64) * ratio).floor() as usize
            }
            _ => panic!(
                "zoombox_left() should not be called in {:?} mode\n",
                self.zoom_level
            ),
        }
    }

    pub fn zoombox_right(&self, max_nb_col_shown_ar: usize) -> usize {
        match self.zoom_level {
            ZoomLevel::ZoomedOut => {
                let mut zb_right = (((self.leftmost_col + self.max_nb_col_shown()) as f64)
                    * self.h_ratio())
                .floor() as usize;
                // If w_a < w_p
                if zb_right > self.app.aln_len() as usize {
                    zb_right = self.app.aln_len() as usize;
                }
                zb_right
            }
            ZoomLevel::ZoomedOutAR => {
                let ratio = self.common_ratio();
                let aln_para_width = min(max_nb_col_shown_ar as u16, self.max_nb_col_shown());
                let mut zb_right = (((self.leftmost_col + self.max_nb_col_shown()) as f64) * ratio)
                    .floor() as usize;
                // If w_a < w_p
                if zb_right > aln_para_width as usize {
                    zb_right = aln_para_width as usize;
                }
                zb_right
            }
            _ => panic!(
                "zoombox_left() should not be called in {:?} mode\n",
                self.zoom_level
            ),
        }
    }

    pub fn cycle_bottom_pane_position(&mut self) {
        self.bottom_pane_position = match self.bottom_pane_position {
            BottomPanePosition::Adjacent => BottomPanePosition::ScreenBottom,
            BottomPanePosition::ScreenBottom => BottomPanePosition::Adjacent,
        }
    }

    pub fn set_zoombox_guides(&mut self, state: bool) {
        self.show_zb_guides = state;
    }

    pub fn toggle_hl_retained_cols(&mut self) {
        self.highlight_retained_cols = !self.highlight_retained_cols;
    }

    // ****************************************************************
    // Colors and Styles

    pub fn theme(&self) -> Theme {
        self.color_scheme().theme
    }

    pub fn color_scheme(&self) -> &ColorScheme {
        &self.color_schemes[self.current_color_scheme_index]
    }

    fn color_scheme_mut(&mut self) -> &mut ColorScheme {
        &mut self.color_schemes[self.current_color_scheme_index]
    }

    pub fn next_color_scheme(&mut self) {
        self.current_color_scheme_index += 1;
        self.current_color_scheme_index %= self.color_schemes.len();
    }

    pub fn prev_color_scheme(&mut self) {
        let nb_color_schemes = self.color_schemes.len();
        self.current_color_scheme_index += nb_color_schemes - 1;
        self.current_color_scheme_index %= nb_color_schemes;
    }

    pub fn set_monochrome(&mut self) {
        // NOTE: this relies on the convention that the monochrome color scheme is last in the
        // list.
        self.current_color_scheme_index = self.color_schemes.len() - 1;
    }

    pub fn add_user_colormap(&mut self, cmap_fname: &String) {
        let get_cmap = colormap_gecos(cmap_fname);
        match get_cmap {
            Ok(cmap) => {
                // Iterate over color schemes, add cmap unless monochrome
                for cs in &mut self.color_schemes {
                    cs.add_colormap(cmap.clone());
                }
            }
            Err(_) => self
                .app
                .error_msg(format!("Error reading colormap {}.", cmap_fname)),
        }
    }

    pub fn next_colormap(&mut self) {
        let cs: &mut ColorScheme = self.color_scheme_mut();
        cs.next_colormap();
    }

    pub fn prev_colormap(&mut self) {
        let cs: &mut ColorScheme = self.color_scheme_mut();
        cs.prev_colormap();
    }

    pub fn search_query(&self) -> String {
        match &self.input_mode {
            InputMode::Search { editor, .. } => editor.text(),
            _ => String::new(),
        }
    }

    pub fn command_text(&self) -> String {
        match &self.input_mode {
            InputMode::Command { editor } => editor.text(),
            _ => String::new(),
        }
    }

    pub fn export_svg_text(&self) -> String {
        match &self.input_mode {
            InputMode::ExportSvg { editor } => editor.text(),
            InputMode::ConfirmOverwrite { editor, .. } => editor.text(),
            _ => String::new(),
        }
    }

    pub fn session_save_text(&self) -> String {
        match &self.input_mode {
            InputMode::SessionSave { editor } => editor.text(),
            InputMode::ConfirmSessionOverwrite { editor, .. } => editor.text(),
            _ => String::new(),
        }
    }

    pub fn view_create_text(&self) -> String {
        match &self.input_mode {
            InputMode::ViewCreate { editor } => editor.text(),
            InputMode::ViewCreateWithList { editor } => editor.text(),
            _ => String::new(),
        }
    }

    pub fn search_highlights(&self) -> (Vec<SearchHighlight<'_>>, SearchHighlightConfig) {
        let mut highlights: Vec<SearchHighlight> = Vec::new();
        let config = self.app.search_color_config();
        let current_match = self.app.current_seq_match();
        if let Some(spans) = self.app.seq_search_spans() {
            highlights.push(SearchHighlight {
                spans_by_seq: spans,
                color: Color::Rgb(
                    config.current_search.0,
                    config.current_search.1,
                    config.current_search.2,
                ),
            });
        }
        for entry in self.app.saved_searches() {
            if !entry.enabled {
                continue;
            }
            highlights.push(SearchHighlight {
                spans_by_seq: &entry.spans_by_seq,
                color: Color::Rgb(entry.color.0, entry.color.1, entry.color.2),
            });
        }
        (
            highlights,
            SearchHighlightConfig {
                min_component: config.min_component,
                gap_dim_factor: config.gap_dim_factor,
                luminance_threshold: config.luminance_threshold,
                current_match,
            },
        )
    }

    pub fn search_list_selected(&self) -> Option<usize> {
        match self.input_mode {
            InputMode::SearchList { selected } => Some(selected),
            _ => None,
        }
    }

    pub fn session_list_state(&self) -> Option<(usize, &[String])> {
        match &self.input_mode {
            InputMode::SessionList { selected, files } => Some((*selected, files.as_slice())),
            _ => None,
        }
    }

    pub fn view_list_selected(&self) -> Option<usize> {
        match self.input_mode {
            InputMode::ViewList { selected } => Some(selected),
            _ => None,
        }
    }

    pub fn view_delete_selected(&self) -> Option<usize> {
        match self.input_mode {
            InputMode::ViewDelete { selected } => Some(selected),
            _ => None,
        }
    }

    pub fn view_move_state(&self) -> Option<(usize, &[usize])> {
        match &self.input_mode {
            InputMode::ViewMove { selected, ranks } => Some((*selected, ranks.as_slice())),
            _ => None,
        }
    }

    pub(crate) fn notes_state(&self) -> Option<(&NotesEditor, NotesTarget)> {
        match &self.input_mode {
            InputMode::Notes { editor, target } => Some((editor, *target)),
            _ => None,
        }
    }

    fn search_kind_label(kind: SearchKind) -> &'static str {
        match kind {
            SearchKind::Regex => "R",
            SearchKind::Emboss => "E",
        }
    }

    pub fn search_status_line_spans(&self) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::raw("Saved: "));
        let enabled: Vec<_> = self
            .app
            .saved_searches()
            .iter()
            .filter(|entry| entry.enabled)
            .collect();
        if enabled.is_empty() {
            spans.push(Span::raw("-"));
        } else {
            for (idx, entry) in enabled.iter().enumerate() {
                if idx > 0 {
                    spans.push(Span::raw(","));
                }
                let label = format!(
                    "{}:{}:{}",
                    entry.id,
                    Self::search_kind_label(entry.kind),
                    entry.name
                );
                let color = Color::Rgb(entry.color.0, entry.color.1, entry.color.2);
                spans.push(Span::styled(label, Style::default().bg(color)));
            }
        }
        spans.push(Span::raw(" | Current: "));
        let current = match &self.input_mode {
            InputMode::Search { editor, kind } => {
                format!("{}:{}", Self::search_kind_label(*kind), editor.text())
            }
            _ => {
                let kind = self.app.current_seq_search_kind();
                let pattern = self
                    .app
                    .current_seq_search_pattern()
                    .unwrap_or("-")
                    .to_string();
                match kind {
                    Some(k) => format!("{}:{}", Self::search_kind_label(k), pattern),
                    None => "-".to_string(),
                }
            }
        };
        spans.push(Span::raw(current));
        spans
    }

    pub fn toggle_video_mode(&mut self) {
        self.video_mode = match self.video_mode {
            VideoMode::Direct => VideoMode::Inverse,
            VideoMode::Inverse => VideoMode::Direct,
        }
    }

    pub fn get_label_num_color(&self) -> Color {
        self.color_scheme().label_num_color
    }

    pub fn get_zoombox_color(&self) -> Color {
        match self.color_scheme().theme {
            Theme::Dark | Theme::Light => self.color_scheme().zoombox_color,
            Theme::Monochrome => Color::Reset,
        }
    }

    pub fn get_seq_metric_style(&self) -> Style {
        match self.color_scheme().theme {
            Theme::Dark | Theme::Light => Style::default().fg(self.color_scheme().seq_metric_color),
            // For now, we let monochrome theme use terminal defaults
            Theme::Monochrome => Style::default().fg(Color::Reset).bg(Color::Reset),
        }
    }

    // ****************************************************************
    // Scrolling

    pub fn disable_scrollbars(&mut self) {
        self.show_scrollbars = false;
    }

    // By lines (zoomed in)

    pub fn scroll_one_line_up(&mut self, count: u16) {
        self.top_line = self.top_line.saturating_sub(count);
    }

    pub fn scroll_one_col_left(&mut self, count: u16) {
        self.leftmost_col = self.leftmost_col.saturating_sub(count);
    }

    pub fn scroll_one_line_down(&mut self, count: u16) {
        self.top_line = min(self.top_line.saturating_add(count), self.max_top_line());
    }

    pub fn scroll_one_col_right(&mut self, count: u16) {
        self.leftmost_col = min(
            self.leftmost_col.saturating_add(count),
            self.max_leftmost_col(),
        );
    }

    // By screens

    pub fn scroll_one_screen_up(&mut self, count: u16) {
        self.top_line = self
            .top_line
            .saturating_sub(count.saturating_mul(self.max_nb_seq_shown()));
    }

    pub fn scroll_one_screen_left(&mut self, count: u16) {
        self.leftmost_col = self
            .leftmost_col
            .saturating_sub(count.saturating_mul(self.max_nb_col_shown()));
    }

    pub fn scroll_one_screen_down(&mut self, count: u16) {
        self.top_line = min(
            self.top_line
                .saturating_add(count.saturating_mul(self.max_nb_seq_shown())),
            self.max_top_line(),
        );
    }

    pub fn scroll_one_screen_right(&mut self, count: u16) {
        self.leftmost_col = min(
            self.leftmost_col
                .saturating_add(count.saturating_mul(self.max_nb_col_shown())),
            self.max_leftmost_col(),
        );
    }

    // By lines, zoomed out
    pub fn scroll_zoombox_one_line_up(&mut self, count: u16) {
        self.top_line = self
            .top_line
            .saturating_sub((count as f64 / self.v_ratio()).round() as u16);
    }

    pub fn scroll_zoombox_one_col_left(&mut self, count: u16) {
        self.leftmost_col = self
            .leftmost_col
            .saturating_sub((count as f64 / self.h_ratio()).round() as u16);
    }

    pub fn scroll_zoombox_one_line_down(&mut self, count: u16) {
        self.top_line = min(
            self.top_line
                .saturating_add((count as f64 / self.v_ratio()).round() as u16),
            self.max_top_line(),
        );
    }

    pub fn scroll_zoombox_one_col_right(&mut self, count: u16) {
        self.leftmost_col = min(
            self.leftmost_col
                .saturating_add((count as f64 / self.h_ratio()).round() as u16),
            self.max_leftmost_col(),
        );
    }

    // ********************************************************
    // Jumps

    pub fn jump_to_top(&mut self) {
        self.top_line = 0
    }

    pub fn jump_to_begin(&mut self) {
        self.leftmost_col = 0
    }

    pub fn jump_to_bottom(&mut self) {
        self.top_line = self.max_top_line()
    }

    pub fn jump_to_end(&mut self) {
        self.leftmost_col = self.max_leftmost_col()
    }

    // Jump to (0-based) line.
    pub fn jump_to_line(&mut self, line: u16) {
        self.top_line = min(line, self.max_top_line());
    }

    pub fn jump_to_col(&mut self, col: u16) {
        // -1 <- 1-based
        self.leftmost_col = min(col - 1, self.max_leftmost_col());
    }

    pub fn jump_to_pct_line(&mut self, pct: u16) {
        let clamped_pct = min(100, pct);
        let tgt_line = (clamped_pct as f64 / 100.0 * self.app.num_seq() as f64).round() as u16;
        self.top_line = tgt_line;
    }

    pub fn jump_to_pct_col(&mut self, pct: u16) {
        let clamped_pct = min(100, pct);
        let tgt_col = (clamped_pct as f64 / 100.0 * self.app.aln_len() as f64).round() as u16;
        self.leftmost_col = tgt_col;
    }

    pub fn jump_to_next_lbl_match(&mut self, count: i16) {
        self.app.increment_current_lbl_match(count as isize);
        let next_match_orig_line = self.app.current_label_match_screenlinenum();
        if let Some(line) = next_match_orig_line {
            self.jump_to_line(line as u16);
        }
    }

    pub fn jump_to_next_seq_match(&mut self, count: i16) {
        if let Some((cur, total)) = self.app.increment_current_seq_match(count as isize) {
            if let Some(m) = self.app.current_seq_match() {
                let screenline = self.app.rank_to_screenline(m.seq_index) as u16;
                self.jump_to_line(screenline);
                self.leftmost_col = m.start as u16;
            }
            self.app.info_msg(format!("match {} of {}", cur, total));
        } else {
            self.app.info_msg("No current search");
        }
    }

    // Debugging

    pub fn assert_invariants(&self) {
        if self.max_nb_col_shown() > self.app.aln_len() {
            assert!(self.max_leftmost_col() == 0);
        } else {
            assert!(
                self.max_leftmost_col() + self.max_nb_col_shown() == self.app.aln_len(),
                "l_max: {} + w_p: {} == w_a: {} failed",
                self.max_leftmost_col(),
                self.max_nb_col_shown(),
                self.app.aln_len()
            );
        }
        assert!(
            self.leftmost_col <= self.max_leftmost_col(),
            "l: {}<= l_max: {}",
            self.leftmost_col,
            self.max_leftmost_col()
        )
    }
}
