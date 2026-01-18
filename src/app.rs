// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier
// Modifications (c) 2026 Peter Carlton

use std::{
    collections::{HashMap, HashSet},
    fmt, fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
};

use hex_color::HexColor;
use regex::{Regex, RegexBuilder};
use serde_json::Value;

use crate::{
    alignment::Alignment,
    app::Metric::{PctIdWrtConsensus, SeqLen},
    app::SeqOrdering::{MetricDecr, MetricIncr, SearchMatch, SourceFile, User},
    errors::TermalError,
    seq::fasta::read_fasta_file,
    session::{
        SessionCurrentSearch, SessionFile, SessionLabelSearch, SessionLabelSource,
        SessionSearchEntry, SessionSearchKind, SessionView,
    },
    tree::{parse_newick, tree_lines_and_order, tree_lines_and_order_with_selection, TreeNode},
};

type SearchColor = (u8, u8, u8);

const DEFAULT_SEARCH_PALETTE: [SearchColor; 6] = [
    (100, 0, 0),
    (0, 100, 0),
    (0, 0, 100),
    (100, 100, 0),
    (0, 100, 100),
    (100, 0, 100),
];
const DEFAULT_CURRENT_SEARCH_COLOR: SearchColor = (80, 80, 80);
const DEFAULT_MIN_COMPONENT: u8 = 100;
const DEFAULT_GAP_DIM_FACTOR: f32 = 0.5;
const DEFAULT_LUMINANCE_THRESHOLD: f32 = 0.55;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeqOrdering {
    SourceFile,
    MetricIncr,
    MetricDecr,
    SearchMatch,
    User,
}

impl fmt::Display for SeqOrdering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sord = match self {
            SourceFile => '-',
            MetricIncr => '↑',
            MetricDecr => '↓',
            SearchMatch => 's',
            User => 'u',
        };
        write!(f, "{}", sord)
    }
}

#[derive(Clone, Copy)]
pub enum Metric {
    PctIdWrtConsensus,
    SeqLen,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SearchKind {
    Regex,
    Emboss,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabelSearchSource {
    Regex,
    Tree,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metric = match self {
            PctIdWrtConsensus => "%id (cons)",
            SeqLen => "seq len",
        };
        write!(f, "{}", metric)
    }
}

impl Metric {
    fn short_label(&self) -> &'static str {
        match self {
            PctIdWrtConsensus => "%id",
            SeqLen => "length",
        }
    }
}

pub struct SearchState {
    // These will eventually be used, when we highlight the actual matching parts of the label, and
    // to allow more informative messages like "pattern 'xyz' has no match".
    #[allow(dead_code)]
    pub pattern: String,
    #[allow(dead_code)]
    regex: Regex,
    // Only the matching linenums; used for jumping to next match on screen. As many elements as
    // there are _matches_.
    pub match_linenums: Vec<usize>,
    pub current: usize,
}

#[derive(Clone)]
struct RemovedSeq {
    rank: usize,
    id: usize,
    header: String,
    sequence: String,
}

#[derive(Clone)]
struct SeqRecord {
    header: String,
    sequence: String,
}

#[derive(Clone)]
struct ViewState {
    name: String,
    sequence_ids: Vec<usize>,
    tree: Option<TreeNode>,
    tree_newick: Option<String>,
    tree_lines: Vec<String>,
    tree_panel_width: u16,
    current_search: Option<SessionCurrentSearch>,
    label_search: Option<SessionLabelSearch>,
    active_search_ids: HashSet<usize>,
    user_ordering: Option<Vec<String>>,
    output_path: PathBuf,
    notes: String,
    selected_ids: HashSet<usize>,
    cursor_id: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewKind {
    Original,
    Filtered,
    Rejected,
    Custom,
}

pub struct SeqSearchState {
    pub kind: SearchKind,
    pub pattern: String,
    pub spans_by_seq: Vec<Vec<(usize, usize)>>,
    pub total_matches: usize,
    pub sequences_with_matches: usize,
    pub matches: Vec<SeqMatch>,
    pub current_match: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectAction {
    RejectedToFile,
    RemovedFromView,
    AlreadyRejected,
}

pub struct RejectResult {
    pub count: usize,
    pub action: RejectAction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeqMatch {
    pub seq_index: usize,
    pub start: usize,
    pub end: usize,
}

pub struct SearchColorConfig {
    pub palette: Vec<SearchColor>,
    pub current_search: SearchColor,
    pub min_component: u8,
    pub gap_dim_factor: f32,
    pub luminance_threshold: f32,
}

impl SearchColorConfig {
    pub fn from_value(value: &Value) -> Self {
        let palette = value
            .get("palette")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| parse_color_value(item).ok())
                    .collect::<Vec<SearchColor>>()
            })
            .unwrap_or_else(|| DEFAULT_SEARCH_PALETTE.to_vec());
        let current_search = value
            .get("current_search")
            .and_then(|v| parse_color_value(v).ok())
            .unwrap_or(DEFAULT_CURRENT_SEARCH_COLOR);
        let min_component = value
            .get("min_component")
            .and_then(|v| v.as_u64())
            .map(|v| v.min(u8::MAX as u64) as u8)
            .unwrap_or(DEFAULT_MIN_COMPONENT);
        let gap_dim_factor = value
            .get("gap_dim_factor")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.0, 1.0) as f32)
            .unwrap_or(DEFAULT_GAP_DIM_FACTOR);
        let luminance_threshold = value
            .get("luminance_threshold")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.0, 1.0) as f32)
            .unwrap_or(DEFAULT_LUMINANCE_THRESHOLD);
        Self {
            palette: if palette.is_empty() {
                DEFAULT_SEARCH_PALETTE.to_vec()
            } else {
                palette
            },
            current_search,
            min_component,
            gap_dim_factor,
            luminance_threshold,
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, TermalError> {
        let contents = fs::read_to_string(path)?;
        let value: Value =
            serde_json::from_str(&contents).map_err(|e| format!("Invalid JSON: {}", e))?;
        Ok(Self::from_value(&value))
    }
}

impl Default for SearchColorConfig {
    fn default() -> Self {
        Self {
            palette: DEFAULT_SEARCH_PALETTE.to_vec(),
            current_search: DEFAULT_CURRENT_SEARCH_COLOR,
            min_component: DEFAULT_MIN_COMPONENT,
            gap_dim_factor: DEFAULT_GAP_DIM_FACTOR,
            luminance_threshold: DEFAULT_LUMINANCE_THRESHOLD,
        }
    }
}

pub struct SearchEntry {
    pub id: usize,
    pub name: String,
    pub query: String,
    pub kind: SearchKind,
    pub enabled: bool,
    pub color: SearchColor,
    pub spans_by_seq: Vec<Vec<(usize, usize)>>,
}

pub struct SearchRegistry {
    searches: Vec<SearchEntry>,
    palette: Vec<SearchColor>,
    next_color_index: usize,
}

#[derive(Clone)]
pub enum MessageKind {
    Info,
    Warning,
    Error,
    Debug,
    Argument,
}

// Simple, 1-line message (possibly just "")
pub struct CurrentMessage {
    pub prefix: String,
    pub message: String,
    pub kind: MessageKind,
}

#[derive(Clone, Default)]
pub struct ToolsConfig {
    pub emboss_bin_dir: Option<PathBuf>,
    pub mafft_bin_dir: Option<PathBuf>,
}

impl ToolsConfig {
    pub fn from_value(value: &Value) -> Self {
        let emboss_bin_dir = value
            .get("emboss_bin_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        let mafft_bin_dir = value
            .get("mafft_bin_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        Self {
            emboss_bin_dir,
            mafft_bin_dir,
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, TermalError> {
        let contents = fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&contents)
            .map_err(|e| TermalError::Format(format!("Invalid JSON {}: {}", path.display(), e)))?;
        Ok(Self::from_value(&value))
    }
}

pub struct TermalConfig {
    pub search_colors: SearchColorConfig,
    pub tools: ToolsConfig,
}

impl TermalConfig {
    pub fn from_file(path: &Path) -> Result<Self, TermalError> {
        let contents = fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&contents)
            .map_err(|e| TermalError::Format(format!("Invalid JSON {}: {}", path.display(), e)))?;
        Ok(Self {
            search_colors: SearchColorConfig::from_value(&value),
            tools: ToolsConfig::from_value(&value),
        })
    }
}

pub struct App {
    pub filename: String,
    pub alignment: Alignment,
    records: Vec<SeqRecord>,
    views: HashMap<String, ViewState>,
    view_order: Vec<String>,
    current_view: String,
    current_view_ids: Vec<usize>,
    ordering_criterion: SeqOrdering,
    metric: Metric,
    // Specifies in which order the aligned sequences should be displayed. The elements of this Vec
    // are _indices_ into the Vec's of headers and sequences that together make up the alignment.
    // By default, they are just ordered from 0 to num_seq - 1, but the user can choose to order
    // according to the current metric, in which case the ordering becomes that of the metric's
    // value for each sequence.
    pub ordering: Vec<usize>,
    pub reverse_ordering: Vec<usize>,
    user_ordering: Option<Vec<String>>,
    pub search_state: Option<SearchState>,
    seq_search_state: Option<SeqSearchState>,
    search_registry: SearchRegistry,
    search_color_config: SearchColorConfig,
    current_msg: CurrentMessage,
    label_search_source: Option<LabelSearchSource>,
    tree_selection_range: Option<(usize, usize)>,
    emboss_bin_dir: Option<PathBuf>,
    mafft_bin_dir: Option<PathBuf>,
    notes: String,
    view_notes: String,
    tree_lines: Vec<String>,
    tree_panel_width: u16,
    tree: Option<TreeNode>,
    tree_newick: Option<String>,
    active_search_ids: HashSet<usize>,
    current_view_output_path: PathBuf,
    rejected_ids: HashSet<usize>,
    selected_ids: HashSet<usize>,
    cursor_id: Option<usize>,
}

impl App {
    fn build_alignment_for_ids(&self, ids: &[usize]) -> Alignment {
        let mut headers = Vec::with_capacity(ids.len());
        let mut sequences = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(record) = self.records.get(*id) {
                headers.push(record.header.clone());
                sequences.push(record.sequence.clone());
            }
        }
        Alignment::from_vecs(headers, sequences)
    }

    fn update_records_from_alignment(
        &mut self,
        alignment: &Alignment,
        ids: &[usize],
    ) -> Result<(), TermalError> {
        let mut seq_map: HashMap<&String, &String> = HashMap::new();
        for (header, sequence) in alignment.headers.iter().zip(alignment.sequences.iter()) {
            seq_map.insert(header, sequence);
        }
        for id in ids {
            let record = self
                .records
                .get_mut(*id)
                .ok_or_else(|| TermalError::Format(format!("Unknown sequence id {}", id)))?;
            let seq = seq_map.get(&record.header).ok_or_else(|| {
                TermalError::Format(format!(
                    "Realigned sequence missing header {}",
                    record.header
                ))
            })?;
            record.sequence = (*seq).clone();
        }
        Ok(())
    }

    fn sync_search_registry_enabled(&mut self) {
        let enabled_ids = self.active_search_ids.clone();
        for entry in &mut self.search_registry.searches {
            entry.enabled = enabled_ids.contains(&entry.id);
        }
    }

    fn capture_current_view_state(&self) -> ViewState {
        let current_search = self
            .seq_search_state
            .as_ref()
            .map(|state| SessionCurrentSearch {
                kind: SessionSearchKind::from(state.kind),
                pattern: state.pattern.clone(),
                current_match: Some(state.current_match),
            });
        let label_search = self.search_state.as_ref().map(|state| SessionLabelSearch {
            pattern: state.pattern.clone(),
            current: Some(state.current),
            matches: Some(state.match_linenums.clone()),
            source: self.label_search_source.map(SessionLabelSource::from),
            tree_range: self.tree_selection_range,
        });
        ViewState {
            name: self.current_view.clone(),
            sequence_ids: self.current_view_ids.clone(),
            tree: self.tree.clone(),
            tree_newick: self.tree_newick.clone(),
            tree_lines: self.tree_lines.clone(),
            tree_panel_width: self.tree_panel_width,
            current_search,
            label_search,
            active_search_ids: self.active_search_ids.clone(),
            user_ordering: self.user_ordering.clone(),
            output_path: self.current_view_output_path.clone(),
            notes: self.view_notes.clone(),
            selected_ids: self.selected_ids.clone(),
            cursor_id: self.cursor_id,
        }
    }

    fn store_current_view_state(&mut self) {
        let view = self.capture_current_view_state();
        self.views.insert(self.current_view.clone(), view);
    }

    fn load_view_state(&mut self, view: ViewState) -> Result<(), TermalError> {
        self.current_view = view.name.clone();
        self.current_view_ids = view.sequence_ids.clone();
        self.alignment = self.build_alignment_for_ids(&self.current_view_ids);
        let len = self.alignment.num_seq();
        self.ordering = (0..len).collect();
        self.reverse_ordering = (0..len).collect();
        self.user_ordering = view.user_ordering.clone();
        self.tree = view.tree.clone();
        self.tree_newick = view.tree_newick.clone();
        self.tree_lines = view.tree_lines.clone();
        self.tree_panel_width = view.tree_panel_width;
        self.tree_selection_range = view
            .label_search
            .as_ref()
            .and_then(|label| label.tree_range);
        self.label_search_source = view
            .label_search
            .as_ref()
            .and_then(|label| label.source)
            .map(LabelSearchSource::from);
        self.search_state = None;
        if let Some(label) = &view.label_search {
            if let Some(matches) = label.matches.clone() {
                let state = build_label_state_from_matches(
                    label.pattern.clone(),
                    matches,
                    self.alignment.headers.len(),
                );
                self.search_state = Some(state);
            } else {
                self.regex_search_labels(&label.pattern);
            }
            if let Some(state) = &mut self.search_state {
                if let Some(idx) = label.current {
                    if idx < state.match_linenums.len() {
                        state.current = idx;
                    }
                }
            }
        }
        if self.selected_ids.is_empty() {
            let matches = self
                .search_state
                .as_ref()
                .map(|state| state.match_linenums.clone());
            if let Some(matches) = matches {
                self.set_selection_from_ranks(&matches);
            }
        }
        self.seq_search_state = None;
        if let Some(current) = &view.current_search {
            match SearchKind::from(current.kind) {
                SearchKind::Regex => self.regex_search_sequences(&current.pattern),
                SearchKind::Emboss => self.emboss_search_sequences(&current.pattern),
            }
            if let Some(state) = &mut self.seq_search_state {
                if let Some(idx) = current.current_match {
                    if idx < state.matches.len() {
                        state.current_match = idx;
                    }
                }
            }
        }
        self.active_search_ids = view.active_search_ids.clone();
        self.sync_search_registry_enabled();
        self.refresh_saved_searches();
        self.recompute_ordering();
        self.current_view_output_path = view.output_path.clone();
        self.view_notes = view.notes.clone();
        self.selected_ids = view.selected_ids.clone();
        self.cursor_id = view.cursor_id;
        self.prune_selection_and_cursor();
        if self.tree.is_some() {
            self.update_tree_lines_for_selection();
        }
        Ok(())
    }

    pub fn current_view_name(&self) -> &str {
        &self.current_view
    }

    pub fn view_names(&self) -> &[String] {
        &self.view_order
    }

    fn view_kind(name: &str) -> ViewKind {
        match name {
            "original" => ViewKind::Original,
            "filtered" => ViewKind::Filtered,
            "rejected" => ViewKind::Rejected,
            _ => ViewKind::Custom,
        }
    }

    fn current_view_kind(&self) -> ViewKind {
        Self::view_kind(&self.current_view)
    }

    pub fn is_protected_view(name: &str) -> bool {
        matches!(name, "original" | "filtered" | "rejected")
    }

    pub fn is_move_target_view(name: &str) -> bool {
        name != "original"
    }

    pub fn switch_view(&mut self, name: &str) -> Result<(), TermalError> {
        if name == self.current_view {
            return Ok(());
        }
        let view = self
            .views
            .get(name)
            .cloned()
            .ok_or_else(|| TermalError::Format(format!("Unknown view {}", name)))?;
        if view.sequence_ids.is_empty() {
            return Err(TermalError::Format(format!(
                "View {} has no sequences",
                name
            )));
        }
        self.store_current_view_state();
        self.load_view_state(view)?;
        self.clear_selection();
        self.clear_cursor();
        Ok(())
    }

    fn prune_selection_and_cursor(&mut self) {
        let allowed: HashSet<usize> = self.current_view_ids.iter().copied().collect();
        self.selected_ids.retain(|id| allowed.contains(id));
        if let Some(id) = self.cursor_id {
            if !allowed.contains(&id) {
                self.cursor_id = None;
            }
        }
    }

    pub fn delete_view(&mut self, name: &str) -> Result<(), TermalError> {
        if Self::is_protected_view(name) {
            return Err(TermalError::Format(format!(
                "View {} cannot be deleted",
                name
            )));
        }
        if !self.views.contains_key(name) {
            return Err(TermalError::Format(format!("Unknown view {}", name)));
        }
        if name != self.current_view {
            self.store_current_view_state();
        }
        self.views.remove(name);
        self.view_order.retain(|view_name| view_name != name);
        if name == self.current_view {
            self.current_view = String::from("original");
            if let Some(view) = self.views.get(&self.current_view).cloned() {
                self.load_view_state(view)?;
            }
        }
        Ok(())
    }

    fn sanitize_view_tag(name: &str) -> String {
        let mut out: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if out.is_empty() {
            out.push_str("view");
        }
        out
    }

    fn output_path_for_view(&self, name: &str) -> PathBuf {
        let tag = match name {
            "filtered" => "filt".to_string(),
            "rejected" => "rej".to_string(),
            "original" => "orig".to_string(),
            _ => Self::sanitize_view_tag(name),
        };
        next_available_output_path(&self.filename, &tag)
    }

    pub fn create_view_from_current(&mut self, name: &str) -> Result<(), TermalError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(TermalError::Format(String::from(
                "View name cannot be empty",
            )));
        }
        if self.views.contains_key(name) {
            return Err(TermalError::Format(format!("View {} already exists", name)));
        }
        let view = ViewState {
            name: name.to_string(),
            sequence_ids: self.current_view_ids.clone(),
            tree: self.tree.clone(),
            tree_newick: self.tree_newick.clone(),
            tree_lines: self.tree_lines.clone(),
            tree_panel_width: self.tree_panel_width,
            current_search: self
                .seq_search_state
                .as_ref()
                .map(|state| SessionCurrentSearch {
                    kind: SessionSearchKind::from(state.kind),
                    pattern: state.pattern.clone(),
                    current_match: Some(state.current_match),
                }),
            label_search: self.search_state.as_ref().map(|state| SessionLabelSearch {
                pattern: state.pattern.clone(),
                current: Some(state.current),
                matches: None,
                source: self.label_search_source.map(SessionLabelSource::from),
                tree_range: self.tree_selection_range,
            }),
            active_search_ids: self.active_search_ids.clone(),
            user_ordering: self.user_ordering.clone(),
            output_path: self.output_path_for_view(name),
            notes: String::new(),
            selected_ids: self.selected_ids.clone(),
            cursor_id: self.cursor_id,
        };
        self.views.insert(name.to_string(), view);
        self.view_order.push(name.to_string());
        Ok(())
    }

    pub fn create_view_from_selection(&mut self, name: &str) -> Result<(), TermalError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(TermalError::Format(String::from(
                "View name cannot be empty",
            )));
        }
        if self.views.contains_key(name) {
            return Err(TermalError::Format(format!("View {} already exists", name)));
        }
        if self.selected_ids.is_empty() {
            return Err(TermalError::Format(String::from("No selected sequences")));
        }
        let sequence_ids: Vec<usize> = self
            .current_view_ids
            .iter()
            .copied()
            .filter(|id| self.selected_ids.contains(id))
            .collect();
        if sequence_ids.is_empty() {
            return Err(TermalError::Format(String::from("No selected sequences")));
        }
        let user_ordering = Some(
            sequence_ids
                .iter()
                .filter_map(|id| self.records.get(*id))
                .map(|rec| rec.header.clone())
                .collect(),
        );
        let view = ViewState {
            name: name.to_string(),
            sequence_ids: sequence_ids.clone(),
            tree: None,
            tree_newick: None,
            tree_lines: Vec::new(),
            tree_panel_width: 0,
            current_search: None,
            label_search: None,
            active_search_ids: self.active_search_ids.clone(),
            user_ordering,
            output_path: self.output_path_for_view(name),
            notes: String::new(),
            selected_ids: sequence_ids.iter().copied().collect(),
            cursor_id: sequence_ids.first().copied(),
        };
        self.views.insert(name.to_string(), view);
        self.view_order.push(name.to_string());
        Ok(())
    }

    fn ensure_filtered_rejected_views(&mut self) {
        let current_search = self
            .seq_search_state
            .as_ref()
            .map(|state| SessionCurrentSearch {
                kind: SessionSearchKind::from(state.kind),
                pattern: state.pattern.clone(),
                current_match: Some(state.current_match),
            });
        let label_search = self.search_state.as_ref().map(|state| SessionLabelSearch {
            pattern: state.pattern.clone(),
            current: Some(state.current),
            matches: None,
            source: self.label_search_source.map(SessionLabelSource::from),
            tree_range: self.tree_selection_range,
        });
        let active_search_ids = self.active_search_ids.clone();
        if !self.views.contains_key("filtered") {
            let view = ViewState {
                name: String::from("filtered"),
                sequence_ids: (0..self.records.len()).collect(),
                tree: None,
                tree_newick: None,
                tree_lines: Vec::new(),
                tree_panel_width: 0,
                current_search: current_search.clone(),
                label_search: label_search.clone(),
                active_search_ids: active_search_ids.clone(),
                user_ordering: None,
                output_path: self.output_path_for_view("filtered"),
                notes: String::new(),
                selected_ids: HashSet::new(),
                cursor_id: None,
            };
            self.views.insert(String::from("filtered"), view);
            self.view_order.push(String::from("filtered"));
        }
        if !self.views.contains_key("rejected") {
            let view = ViewState {
                name: String::from("rejected"),
                sequence_ids: Vec::new(),
                tree: None,
                tree_newick: None,
                tree_lines: Vec::new(),
                tree_panel_width: 0,
                current_search,
                label_search,
                active_search_ids,
                user_ordering: None,
                output_path: self.output_path_for_view("rejected"),
                notes: String::new(),
                selected_ids: HashSet::new(),
                cursor_id: None,
            };
            self.views.insert(String::from("rejected"), view);
            self.view_order.push(String::from("rejected"));
        }
    }

    fn rebuild_filtered_rejected_views(&mut self) {
        let all_ids: Vec<usize> = (0..self.records.len()).collect();
        if let Some(filtered) = self.views.get_mut("filtered") {
            filtered.sequence_ids = all_ids
                .iter()
                .copied()
                .filter(|id| !self.rejected_ids.contains(id))
                .collect();
        }
        if let Some(rejected) = self.views.get_mut("rejected") {
            rejected.sequence_ids = all_ids
                .iter()
                .copied()
                .filter(|id| self.rejected_ids.contains(id))
                .collect();
        }
    }

    pub fn current_view_output_path(&self) -> &Path {
        &self.current_view_output_path
    }

    pub fn ids_for_ranks(&self, ranks: &[usize]) -> Vec<usize> {
        let mut ids = Vec::new();
        for &rank in ranks {
            if let Some(id) = self.current_view_ids.get(rank).copied() {
                ids.push(id);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn append_ids_in_order(existing: &mut Vec<usize>, ids: &[usize]) -> usize {
        let mut seen: HashSet<usize> = existing.iter().copied().collect();
        let mut added = 0;
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for id in sorted {
            if seen.insert(id) {
                existing.push(id);
                added += 1;
            }
        }
        added
    }

    pub fn add_ids_to_view(&mut self, name: &str, ids: &[usize]) -> Result<usize, TermalError> {
        if name == "original" {
            return Ok(0);
        }
        if ids.is_empty() {
            return Ok(0);
        }
        if name == "filtered" || name == "rejected" {
            self.ensure_filtered_rejected_views();
            for id in ids {
                if name == "rejected" {
                    self.rejected_ids.insert(*id);
                } else {
                    self.rejected_ids.remove(id);
                }
            }
            self.rebuild_filtered_rejected_views();
            if matches!(self.current_view.as_str(), "filtered" | "rejected") {
                if let Some(view) = self.views.get(&self.current_view).cloned() {
                    self.load_view_state(view)?;
                }
            }
            return Ok(ids.len());
        }

        let view = self
            .views
            .get_mut(name)
            .ok_or_else(|| TermalError::Format(format!("Unknown view {}", name)))?;
        let added = Self::append_ids_in_order(&mut view.sequence_ids, ids);
        if name == self.current_view {
            self.current_view_ids = view.sequence_ids.clone();
            self.alignment = self.build_alignment_for_ids(&self.current_view_ids);
            let len = self.alignment.num_seq();
            self.ordering = (0..len).collect();
            self.reverse_ordering = (0..len).collect();
            self.refresh_saved_searches();
            self.recompute_ordering();
            self.prune_selection_and_cursor();
        }
        Ok(added)
    }
    pub fn from_session_file(path: &Path) -> Result<Self, TermalError> {
        let contents = fs::read_to_string(path)?;
        let session: SessionFile = serde_json::from_str(&contents)
            .map_err(|e| TermalError::Format(format!("Invalid session JSON: {}", e)))?;
        let filename = if session.source_filename.is_empty() {
            path.to_string_lossy().to_string()
        } else {
            session.source_filename.clone()
        };
        let alignment = Alignment::from_vecs(session.headers.clone(), session.sequences.clone());
        let mut app = App::new(&filename, alignment, None);
        app.apply_session(session, filename)?;
        Ok(app)
    }
    pub fn new(path: &str, alignment: Alignment, usr_ord: Option<Vec<String>>) -> Self {
        let len = alignment.num_seq();
        let records: Vec<SeqRecord> = alignment
            .headers
            .iter()
            .cloned()
            .zip(alignment.sequences.iter().cloned())
            .enumerate()
            .map(|(_id, (header, sequence))| SeqRecord { header, sequence })
            .collect();
        let cur_msg = CurrentMessage {
            prefix: String::from(""),
            message: String::from(""),
            kind: MessageKind::Info,
        };
        let search_color_config = SearchColorConfig::default();
        let original_output_path = next_available_output_path(path, "orig");
        let mut views = HashMap::new();
        let mut active_search_ids = HashSet::new();
        let original_view = ViewState {
            name: String::from("original"),
            sequence_ids: (0..len).collect(),
            tree: None,
            tree_newick: None,
            tree_lines: Vec::new(),
            tree_panel_width: 0,
            current_search: None,
            label_search: None,
            active_search_ids: HashSet::new(),
            user_ordering: usr_ord.clone(),
            output_path: original_output_path.clone(),
            notes: String::new(),
            selected_ids: HashSet::new(),
            cursor_id: None,
        };
        active_search_ids.extend(original_view.active_search_ids.iter().copied());
        views.insert(String::from("original"), original_view);
        App {
            filename: path.to_string(),
            alignment,
            records,
            views,
            view_order: vec![String::from("original")],
            current_view: String::from("original"),
            current_view_ids: (0..len).collect(),
            ordering_criterion: SourceFile,
            metric: PctIdWrtConsensus,
            ordering: (0..len).collect(),
            reverse_ordering: (0..len).collect(),
            user_ordering: usr_ord,
            search_state: None,
            seq_search_state: None,
            search_registry: SearchRegistry::new(search_color_config.palette.clone()),
            search_color_config,
            current_msg: cur_msg,
            label_search_source: None,
            tree_selection_range: None,
            emboss_bin_dir: None,
            mafft_bin_dir: None,
            notes: String::new(),
            view_notes: String::new(),
            tree_lines: Vec::new(),
            tree_panel_width: 0,
            tree: None,
            tree_newick: None,
            active_search_ids,
            current_view_output_path: original_output_path,
            rejected_ids: HashSet::new(),
            selected_ids: HashSet::new(),
            cursor_id: None,
        }
    }

    // Computed properties (TODO: could be set in a struct member, as they do not change)
    // FIXME where do we need num_seq as u16?

    pub fn num_seq(&self) -> u16 {
        self.alignment.num_seq().try_into().unwrap()
    }

    pub fn aln_len(&self) -> u16 {
        self.alignment.aln_len().try_into().unwrap()
    }

    pub fn all_sequences_rejected(&self) -> bool {
        !self.records.is_empty() && self.rejected_ids.len() == self.records.len()
    }

    pub fn default_session_path(&self) -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file_name = Path::new(&self.filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(self.filename.as_str());
        let stem = Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name);
        cwd.join(format!("{}.trml", stem))
    }

    pub fn save_session(&mut self, path: &Path) -> Result<(), TermalError> {
        self.store_current_view_state();
        let session = self.to_session_file();
        let json = serde_json::to_string_pretty(&session)
            .map_err(|e| TermalError::Format(format!("Invalid session JSON: {}", e)))?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_session(&mut self, path: &Path) -> Result<(), TermalError> {
        let contents = fs::read_to_string(path)?;
        let session: SessionFile = serde_json::from_str(&contents)
            .map_err(|e| TermalError::Format(format!("Invalid session JSON: {}", e)))?;
        let filename = if session.source_filename.is_empty() {
            path.to_string_lossy().to_string()
        } else {
            session.source_filename.clone()
        };
        self.apply_session(session, filename)?;
        Ok(())
    }

    fn to_session_file(&self) -> SessionFile {
        let headers: Vec<String> = self.records.iter().map(|rec| rec.header.clone()).collect();
        let sequences: Vec<String> = self
            .records
            .iter()
            .map(|rec| rec.sequence.clone())
            .collect();
        let saved_searches = self
            .saved_searches()
            .iter()
            .map(|entry| SessionSearchEntry {
                id: entry.id,
                name: entry.name.clone(),
                query: entry.query.clone(),
                kind: SessionSearchKind::from(entry.kind),
                enabled: entry.enabled,
                color: entry.color,
            })
            .collect();
        let mut views: Vec<SessionView> = Vec::new();
        for name in &self.view_order {
            if let Some(view) = self.views.get(name) {
                views.push(SessionView {
                    name: view.name.clone(),
                    sequence_ids: view.sequence_ids.clone(),
                    tree_newick: view.tree_newick.clone(),
                    tree_lines: if view.tree_lines.is_empty() {
                        None
                    } else {
                        Some(view.tree_lines.clone())
                    },
                    current_search: view.current_search.clone(),
                    label_search: view.label_search.clone(),
                    active_search_ids: view.active_search_ids.iter().copied().collect(),
                    user_ordering: view.user_ordering.clone(),
                    notes: if view.notes.is_empty() {
                        None
                    } else {
                        Some(view.notes.clone())
                    },
                    selected_ids: if view.selected_ids.is_empty() {
                        None
                    } else {
                        Some(view.selected_ids.iter().copied().collect())
                    },
                    cursor_id: view.cursor_id,
                });
            }
        }
        SessionFile {
            version: 3,
            source_filename: self.filename.clone(),
            headers,
            sequences,
            views: Some(views),
            current_view: Some(self.current_view.clone()),
            tree_lines: None,
            tree_newick: None,
            saved_searches,
            current_search: None,
            label_search: None,
            notes: if self.notes.is_empty() {
                None
            } else {
                Some(self.notes.clone())
            },
        }
    }

    fn apply_session(&mut self, session: SessionFile, filename: String) -> Result<(), TermalError> {
        self.filename = filename;
        self.records = session
            .headers
            .into_iter()
            .zip(session.sequences.into_iter())
            .enumerate()
            .map(|(_id, (header, sequence))| SeqRecord { header, sequence })
            .collect();
        let original_ids: Vec<usize> = (0..self.records.len()).collect();
        self.alignment = self.build_alignment_for_ids(&original_ids);
        self.ordering_criterion = SourceFile;
        let len = self.alignment.num_seq();
        self.ordering = (0..len).collect();
        self.reverse_ordering = (0..len).collect();
        self.user_ordering = None;

        self.views.clear();
        self.view_order.clear();
        self.current_view_ids = original_ids.clone();
        if let Some(views) = session.views {
            for view in views {
                let tree = match &view.tree_newick {
                    Some(newick) => Some(parse_newick(newick)?),
                    None => None,
                };
                let tree_lines = if let Some(ref tree) = tree {
                    tree_lines_and_order(tree)?.0
                } else {
                    view.tree_lines.clone().unwrap_or_default()
                };
                let tree_panel_width = tree_lines
                    .iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0)
                    .min(u16::MAX as usize) as u16;
                let output_path = self.output_path_for_view(&view.name);
                let active_search_ids: HashSet<usize> =
                    view.active_search_ids.iter().copied().collect();
                let view_state = ViewState {
                    name: view.name.clone(),
                    sequence_ids: view.sequence_ids,
                    tree,
                    tree_newick: view.tree_newick,
                    tree_lines,
                    tree_panel_width,
                    current_search: view.current_search,
                    label_search: view.label_search,
                    active_search_ids,
                    user_ordering: view.user_ordering,
                    output_path,
                    notes: view.notes.unwrap_or_default(),
                    selected_ids: view.selected_ids.unwrap_or_default().into_iter().collect(),
                    cursor_id: view.cursor_id,
                };
                self.view_order.push(view.name.clone());
                self.views.insert(view.name, view_state);
            }
        } else {
            let tree = match &session.tree_newick {
                Some(newick) => Some(parse_newick(newick)?),
                None => None,
            };
            let tree_lines = if let Some(ref tree) = tree {
                tree_lines_and_order(tree)?.0
            } else {
                session.tree_lines.unwrap_or_default()
            };
            let tree_panel_width = tree_lines
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0)
                .min(u16::MAX as usize) as u16;
            let view = ViewState {
                name: String::from("original"),
                sequence_ids: original_ids.clone(),
                tree,
                tree_newick: session.tree_newick,
                tree_lines,
                tree_panel_width,
                current_search: session.current_search,
                label_search: session.label_search,
                active_search_ids: HashSet::new(),
                user_ordering: None,
                output_path: self.output_path_for_view("original"),
                notes: String::new(),
                selected_ids: HashSet::new(),
                cursor_id: None,
            };
            self.view_order.push(String::from("original"));
            self.views.insert(String::from("original"), view);
        }
        self.current_view = session
            .current_view
            .unwrap_or_else(|| String::from("original"));
        if !self.views.contains_key(&self.current_view) {
            self.current_view = String::from("original");
        }
        self.rejected_ids = self
            .views
            .get("rejected")
            .map(|view| view.sequence_ids.iter().copied().collect())
            .unwrap_or_else(HashSet::new);

        self.search_registry = SearchRegistry::new(self.search_color_config.palette.clone());
        for entry in session.saved_searches {
            self.search_registry.searches.push(SearchEntry {
                id: entry.id,
                name: entry.name,
                query: entry.query,
                kind: SearchKind::from(entry.kind),
                enabled: entry.enabled,
                color: entry.color,
                spans_by_seq: Vec::new(),
            });
        }
        self.search_registry.next_color_index = self.search_registry.searches.len();

        self.notes = session.notes.unwrap_or_default();

        self.current_msg = CurrentMessage {
            prefix: String::new(),
            message: String::new(),
            kind: MessageKind::Info,
        };
        if let Some(view) = self.views.get(&self.current_view).cloned() {
            self.load_view_state(view)?;
        }
        Ok(())
    }

    fn recompute_ordering(&mut self) {
        match self.ordering_criterion {
            MetricIncr => {
                self.ordering = order(self.order_values());
            }
            MetricDecr => {
                let mut ord = order(self.order_values());
                ord.reverse();
                self.ordering = ord;
            }
            SearchMatch => {
                if let Some(state) = &self.seq_search_state {
                    let mut matches: Vec<usize> = Vec::new();
                    let mut non_matches: Vec<usize> = Vec::new();
                    for (idx, spans) in state.spans_by_seq.iter().enumerate() {
                        if spans.is_empty() {
                            non_matches.push(idx);
                        } else {
                            matches.push(idx);
                        }
                    }
                    matches.extend(non_matches);
                    self.ordering = matches;
                } else {
                    self.ordering = (0..self.alignment.num_seq()).collect();
                }
            }
            SourceFile => {
                self.ordering = (0..self.alignment.num_seq()).collect();
            }
            User => {
                // Do not change ordering if no user ordering provided, or if it had
                // problems (this is checked early on, in main(), around l. 180 (as of commit
                // 13a2e2e).).
                match &self.user_ordering {
                    None => {
                        // Note: self.ordering_criterion is not supposed to have value 'User' unless a
                        // valid ordering was supplied (see prev_ordering_criterion() and
                        // next_ordering_criterion()).
                    }
                    Some(uord_vec) => {
                        // Good ordering
                        // Technically, we could index by &str, but I'm not sure we'd gain a lot.
                        let mut hdr2rank: HashMap<String, usize> = HashMap::new();
                        for (idx, hdr) in self.alignment.headers.iter().enumerate() {
                            hdr2rank.insert(hdr.to_string(), idx);
                        }
                        // Iterate over ordering, looking up file index from the above hash.
                        let mut result: Vec<usize> = Vec::new();
                        // TODO: now that we no longer check for discrepancies here, this should be
                        //feasible in a sinmple map.
                        for hdr in uord_vec.iter() {
                            match hdr2rank.get(hdr) {
                                Some(rank) => result.push(*rank),
                                None => break,
                            }
                        }
                        self.ordering = result;
                    }
                }
            }
        }
        self.reverse_ordering = order(&self.ordering);
    }

    pub fn next_ordering_criterion(&mut self) {
        self.ordering_criterion = match self.ordering_criterion {
            SourceFile => MetricIncr,
            MetricIncr => MetricDecr,
            // move to User IFF valid ordering
            MetricDecr => SearchMatch,
            SearchMatch => match self.user_ordering {
                Some(_) => User,
                None => SourceFile,
            },
            User => SourceFile,
        };
        self.recompute_ordering();
    }

    pub fn prev_ordering_criterion(&mut self) {
        self.ordering_criterion = match self.ordering_criterion {
            MetricIncr => SourceFile,
            MetricDecr => MetricIncr,
            SearchMatch => MetricDecr,
            User => SearchMatch,
            // move to User IFF valid ordering
            SourceFile => match self.user_ordering {
                Some(_) => User,
                None => SearchMatch,
            },
        };
        self.recompute_ordering();
    }

    // Maps a rank (= index in the original alignment) to the corresponding line on the screen
    // (which may or may not be visible). This is affected by the ordering. This is NOT
    // user-facing, hence 0-based.
    pub fn rank_to_screenline(&self, rank: usize) -> usize {
        self.reverse_ordering[rank]
    }

    pub fn next_metric(&mut self) {
        self.metric = match self.metric {
            PctIdWrtConsensus => SeqLen,
            SeqLen => PctIdWrtConsensus,
        };
        self.recompute_ordering();
    }

    // NOTE: for now, there are only two metrics, so next and prev are the same. This might change,
    // however.
    pub fn prev_metric(&mut self) {
        self.metric = match self.metric {
            PctIdWrtConsensus => SeqLen,
            SeqLen => PctIdWrtConsensus,
        };
        self.recompute_ordering();
    }

    pub fn output_info(&self) {
        println!("name: {}", self.filename);
        println!("nb_sequences: {}", self.num_seq());
        println!("nb_columns: {}", self.aln_len());
        println!();
    }

    pub fn get_seq_ordering(&self) -> SeqOrdering {
        self.ordering_criterion
    }

    pub fn ordering_status_label(&self) -> String {
        match self.ordering_criterion {
            SourceFile => String::from("o:original"),
            SearchMatch => String::from("o:match"),
            User => String::from("o:tree"),
            MetricIncr => format!("o:{}↑", self.metric.short_label()),
            MetricDecr => format!("o:{}↓", self.metric.short_label()),
        }
    }

    pub fn get_metric(&self) -> Metric {
        self.metric
    }

    // TODO: rename to order_by_metric
    pub fn order_values(&self) -> &Vec<f64> {
        match self.metric {
            PctIdWrtConsensus => &self.alignment.id_wrt_consensus,
            SeqLen => &self.alignment.relative_seq_len,
        }
    }

    // Label search

    pub fn regex_search_labels(&mut self, pattern: &str) {
        // self.debug_msg("Regex search");
        match compute_label_search_state(&self.alignment.headers, pattern) {
            Ok(state) => {
                self.set_selection_from_ranks(&state.match_linenums);
                self.search_state = Some(state);
                self.label_search_source = Some(LabelSearchSource::Regex);
                self.tree_selection_range = None;
                self.update_tree_lines_for_selection();
            }
            Err(e) => {
                self.error_msg(format!("Malformed regex {}.", e));
                self.search_state = None;
                self.label_search_source = None;
                self.tree_selection_range = None;
                self.update_tree_lines_for_selection();
            }
        };
    }

    pub fn select_label_by_rank(&mut self, rank: usize) -> Result<(), TermalError> {
        if rank >= self.alignment.headers.len() {
            return Err(TermalError::Format(String::from(
                "Sequence number out of range",
            )));
        }
        if let Some(id) = self.current_view_ids.get(rank).copied() {
            self.set_selection_from_ids(&[id]);
        }
        Ok(())
    }

    pub fn current_label_match_screenlinenum(&self) -> Option<usize> {
        if let Some(state) = &self.search_state {
            if state.match_linenums.len() > 0 {
                Some(self.rank_to_screenline(state.match_linenums[state.current]))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn increment_current_lbl_match(&mut self, count: isize) {
        match &self.search_state {
            Some(state) => {
                let nb_matches = state.match_linenums.len();
                if nb_matches > 0 {
                    // (i+n).rem(l)
                    let new =
                        (state.current as isize + count).rem_euclid(nb_matches as isize) as usize;
                    //let new = (state.current + count) % nb_matches.;
                    self.search_state.as_mut().unwrap().current = new;
                    if let Some(rank) = self.current_label_match_rank() {
                        if let Some(id) = self.current_view_ids.get(rank).copied() {
                            self.cursor_id = Some(id);
                        }
                    }
                    self.info_msg(format!(
                        "match #{}/{}",
                        self.search_state.as_ref().unwrap().current + 1, // +1 <- user is 1-based
                        self.search_state.as_ref().unwrap().match_linenums.len()
                    ));
                } else {
                    self.info_msg("No match.");
                }
            }
            None => {
                self.info_msg("No current search.");
            }
        }
    }

    pub fn current_label_match_rank(&self) -> Option<usize> {
        self.search_state
            .as_ref()
            .and_then(|state| state.match_linenums.get(state.current).copied())
    }

    // Returns true IFF there is a search result AND header of rank `rank` (i.e., without
    // correction for order) is a match.
    pub fn cursor_rank(&self) -> Option<usize> {
        let id = self.cursor_id?;
        self.current_view_ids
            .iter()
            .position(|seq_id| *seq_id == id)
    }

    pub fn is_cursor_rank(&self, rank: usize) -> bool {
        self.cursor_rank().map(|cur| cur == rank).unwrap_or(false)
    }

    pub fn is_label_selected(&self, rank: usize) -> bool {
        if let Some(id) = self.current_view_ids.get(rank) {
            self.selected_ids.contains(id)
        } else {
            false
        }
    }

    pub fn selection_ranks(&self) -> Vec<usize> {
        self.current_view_ids
            .iter()
            .enumerate()
            .filter_map(|(rank, id)| self.selected_ids.contains(id).then_some(rank))
            .collect()
    }

    pub fn toggle_selection_on_cursor(&mut self) {
        let Some(id) = self.cursor_id else {
            return;
        };
        if !self.current_view_ids.contains(&id) {
            return;
        }
        if self.selected_ids.len() == 1 && self.selected_ids.contains(&id) {
            self.clear_selection();
        } else {
            self.set_selection_from_ids(&[id]);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_ids.clear();
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.selected_ids.clear();
        }
    }

    pub fn select_all_in_view(&mut self) {
        self.selected_ids = self.current_view_ids.iter().copied().collect();
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.selected_ids = self.selected_ids.clone();
        }
    }

    pub fn clear_cursor(&mut self) {
        self.cursor_id = None;
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.cursor_id = None;
        }
    }

    pub fn toggle_cursor(&mut self) {
        if self.cursor_id.is_some() {
            self.clear_cursor();
            return;
        }
        let ids = self.cursor_cycle_ids();
        self.cursor_id = ids.first().copied();
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.cursor_id = self.cursor_id;
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.cursor_id.is_none() {
            return;
        }
        let ids = self.cursor_cycle_ids();
        if ids.is_empty() {
            self.cursor_id = None;
            return;
        }
        let idx = match self.cursor_id {
            Some(id) => ids.iter().position(|item| *item == id),
            None => None,
        };
        let current = idx.unwrap_or(0) as isize;
        let len = ids.len() as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.cursor_id = Some(ids[next]);
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.cursor_id = self.cursor_id;
        }
    }

    fn cursor_cycle_ids(&self) -> Vec<usize> {
        let use_selection = !self.selected_ids.is_empty();
        let mut ids = Vec::new();
        for &rank in &self.ordering {
            if let Some(id) = self.current_view_ids.get(rank).copied() {
                if !use_selection || self.selected_ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        ids
    }

    fn set_selection_from_ranks(&mut self, ranks: &[usize]) {
        let ids: Vec<usize> = ranks
            .iter()
            .filter_map(|rank| self.current_view_ids.get(*rank).copied())
            .collect();
        self.set_selection_from_ids(&ids);
    }

    fn set_selection_from_ids(&mut self, ids: &[usize]) {
        self.selected_ids.clear();
        for id in ids {
            if self.current_view_ids.contains(id) {
                self.selected_ids.insert(*id);
            }
        }
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.selected_ids = self.selected_ids.clone();
        }
        self.cursor_id = self.selected_ids.iter().copied().next();
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.cursor_id = self.cursor_id;
        }
    }

    pub fn reset_lbl_search(&mut self) {
        self.search_state = None;
        self.label_search_source = None;
        self.tree_selection_range = None;
        self.update_tree_lines_for_selection();
    }

    pub fn set_label_matches_from_tree(&mut self, matches: Vec<usize>, tree_range: (usize, usize)) {
        if matches.is_empty() {
            self.tree_selection_range = None;
            self.selected_ids.clear();
            if let Some(view) = self.views.get_mut(&self.current_view) {
                view.selected_ids.clear();
            }
            self.update_tree_lines_for_selection();
            return;
        }
        let state = build_label_state_from_matches(
            String::from("<tree>"),
            matches,
            self.alignment.headers.len(),
        );
        self.tree_selection_range = Some(tree_range);
        self.selected_ids.clear();
        for rank in &state.match_linenums {
            if let Some(id) = self.current_view_ids.get(*rank).copied() {
                self.selected_ids.insert(id);
            }
        }
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.selected_ids = self.selected_ids.clone();
        }
        if let Some(first) = state.match_linenums.first().copied() {
            if let Some(id) = self.current_view_ids.get(first).copied() {
                self.cursor_id = Some(id);
            }
        }
        self.update_tree_lines_for_selection();
    }

    pub fn regex_search_sequences(&mut self, pattern: &str) {
        if pattern.is_empty() {
            self.clear_seq_search();
            return;
        }
        match compute_seq_search_state(&self.alignment.sequences, pattern, SearchKind::Regex) {
            Ok(state) => {
                self.seq_search_state = Some(state);
                if matches!(self.ordering_criterion, SearchMatch) {
                    self.recompute_ordering();
                }
            }
            Err(e) => {
                self.error_msg(format!("Malformed regex {}.", e));
                self.clear_seq_search();
            }
        }
    }

    pub fn seq_search_spans(&self) -> Option<&[Vec<(usize, usize)>]> {
        self.seq_search_state
            .as_ref()
            .map(|state| state.spans_by_seq.as_slice())
    }

    pub fn current_seq_search_pattern(&self) -> Option<&str> {
        self.seq_search_state
            .as_ref()
            .map(|state| state.pattern.as_str())
    }

    pub fn current_seq_search_kind(&self) -> Option<SearchKind> {
        self.seq_search_state.as_ref().map(|state| state.kind)
    }

    pub fn seq_search_counts(&self) -> Option<(usize, usize)> {
        self.seq_search_state
            .as_ref()
            .map(|state| (state.total_matches, state.sequences_with_matches))
    }

    pub fn has_seq_search(&self) -> bool {
        self.seq_search_state
            .as_ref()
            .map(|state| state.total_matches > 0)
            .unwrap_or(false)
    }

    pub fn current_seq_match(&self) -> Option<SeqMatch> {
        self.seq_search_state
            .as_ref()
            .and_then(|state| state.matches.get(state.current_match).copied())
    }

    pub fn increment_current_seq_match(&mut self, count: isize) -> Option<(usize, usize)> {
        if let Some(state) = &mut self.seq_search_state {
            if state.matches.is_empty() {
                return None;
            }
            let len = state.matches.len() as isize;
            let new = (state.current_match as isize + count).rem_euclid(len) as usize;
            state.current_match = new;
            return Some((state.current_match + 1, state.matches.len()));
        }
        None
    }

    pub fn clear_seq_search(&mut self) {
        self.seq_search_state = None;
        if matches!(self.ordering_criterion, SearchMatch) {
            self.recompute_ordering();
        }
    }

    pub fn search_color_config(&self) -> &SearchColorConfig {
        &self.search_color_config
    }

    pub fn set_search_color_config(&mut self, config: SearchColorConfig) {
        self.search_registry.set_palette(config.palette.clone());
        self.search_color_config = config;
    }

    pub fn add_saved_search(&mut self, name: String, query: String) -> Result<(), String> {
        self.add_saved_search_with_kind(name, query, SearchKind::Regex)
    }

    pub fn add_saved_search_with_kind(
        &mut self,
        name: String,
        query: String,
        kind: SearchKind,
    ) -> Result<(), String> {
        if query.is_empty() {
            return Err(String::from("Empty search query"));
        }
        let state = match kind {
            SearchKind::Regex => compute_seq_search_state(&self.alignment.sequences, &query, kind)
                .map_err(|e| format!("Malformed regex {}.", e))?,
            SearchKind::Emboss if self.emboss_bin_dir.is_none() => {
                return Err(String::from(
                    "Emboss search unavailable. Create .termalconfig in $HOME or current directory with emboss_bin_dir.",
                ));
            }
            SearchKind::Emboss => compute_emboss_search_state(
                &self.alignment.headers,
                &self.alignment.sequences,
                &query,
                self.emboss_bin_dir.as_deref(),
            )
            .map_err(|e| format!("Emboss search failed: {}", e))?,
        };
        self.search_registry
            .add_search(name, query, kind, state.spans_by_seq);
        if let Some(entry) = self.search_registry.searches.last() {
            self.active_search_ids.insert(entry.id);
            self.sync_search_registry_enabled();
            if let Some(view) = self.views.get_mut(&self.current_view) {
                view.active_search_ids = self.active_search_ids.clone();
            }
        }
        Ok(())
    }

    pub fn delete_saved_search(&mut self, index: usize) -> bool {
        let removed = self.search_registry.delete(index);
        if removed {
            self.active_search_ids = self
                .search_registry
                .searches
                .iter()
                .filter(|entry| entry.enabled)
                .map(|entry| entry.id)
                .collect();
            if let Some(view) = self.views.get_mut(&self.current_view) {
                view.active_search_ids = self.active_search_ids.clone();
            }
        }
        removed
    }

    pub fn toggle_saved_search(&mut self, index: usize) -> bool {
        let toggled = self.search_registry.toggle(index);
        if toggled {
            self.active_search_ids = self
                .search_registry
                .searches
                .iter()
                .filter(|entry| entry.enabled)
                .map(|entry| entry.id)
                .collect();
            if let Some(view) = self.views.get_mut(&self.current_view) {
                view.active_search_ids = self.active_search_ids.clone();
            }
        }
        toggled
    }

    pub fn saved_searches(&self) -> &[SearchEntry] {
        self.search_registry.entries()
    }

    pub fn set_emboss_bin_dir(&mut self, dir: Option<PathBuf>) {
        self.emboss_bin_dir = dir;
    }

    pub fn set_mafft_bin_dir(&mut self, dir: Option<PathBuf>) {
        self.mafft_bin_dir = dir;
    }

    pub fn emboss_search_sequences(&mut self, pattern: &str) {
        if pattern.is_empty() {
            self.clear_seq_search();
            return;
        }
        if self.emboss_bin_dir.is_none() {
            self.error_msg(
                "Emboss search unavailable. Create .termalconfig in $HOME or current directory with emboss_bin_dir.",
            );
            self.clear_seq_search();
            return;
        }
        match compute_emboss_search_state(
            &self.alignment.headers,
            &self.alignment.sequences,
            pattern,
            self.emboss_bin_dir.as_deref(),
        ) {
            Ok(state) => {
                self.seq_search_state = Some(state);
                if matches!(self.ordering_criterion, SearchMatch) {
                    self.recompute_ordering();
                }
            }
            Err(e) => {
                self.error_msg(format!("Emboss search failed: {}", e));
                self.clear_seq_search();
            }
        }
    }

    pub fn remove_sequence(&mut self, rank: usize) -> Option<(String, String)> {
        let mut removed = self.remove_sequences(&[rank]);
        removed.pop()
    }

    pub fn remove_sequences(&mut self, ranks: &[usize]) -> Vec<(String, String)> {
        self.remove_sequences_with_ranks(ranks)
            .into_iter()
            .map(|removed| (removed.header, removed.sequence))
            .collect()
    }

    fn remove_sequences_with_ranks(&mut self, ranks: &[usize]) -> Vec<RemovedSeq> {
        if ranks.is_empty() {
            return Vec::new();
        }
        let mut header_set: HashSet<String> = HashSet::new();
        let current_label_header = self
            .current_label_match_rank()
            .and_then(|rank| self.alignment.headers.get(rank))
            .cloned();
        let current_seq_match = self.seq_search_state.as_ref().and_then(|state| {
            let match_entry = state.matches.get(state.current_match).copied();
            let header = match_entry.and_then(|m| self.alignment.headers.get(m.seq_index).cloned());
            let span = match_entry.map(|m| (m.start, m.end));
            header.zip(span)
        });
        let seq_search_kind = self.seq_search_state.as_ref().map(|state| state.kind);
        let seq_search_pattern = self
            .seq_search_state
            .as_ref()
            .map(|state| state.pattern.clone());
        let label_search_pattern = self
            .search_state
            .as_ref()
            .map(|state| state.pattern.clone());
        let label_search_headers = if self.label_search_source == Some(LabelSearchSource::Tree) {
            self.search_state.as_ref().map(|state| {
                state
                    .match_linenums
                    .iter()
                    .filter_map(|idx| self.alignment.headers.get(*idx).cloned())
                    .collect::<Vec<String>>()
            })
        } else {
            None
        };
        let label_search_source = self.label_search_source;

        let mut removed: Vec<RemovedSeq> = Vec::new();
        let mut removed_ids: Vec<usize> = Vec::new();
        let mut sorted: Vec<usize> = ranks.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for rank in sorted {
            if let Some(item) = self.alignment.remove_seq(rank) {
                let id = self.current_view_ids.get(rank).copied().unwrap_or_default();
                if rank < self.current_view_ids.len() {
                    self.current_view_ids.remove(rank);
                }
                removed_ids.push(id);
                removed.push(RemovedSeq {
                    rank,
                    id,
                    header: item.0,
                    sequence: item.1,
                });
            }
        }
        header_set.extend(self.alignment.headers.iter().cloned());
        if let Some(ordering) = &mut self.user_ordering {
            ordering.retain(|hdr| header_set.contains(hdr));
        }

        self.recompute_search_state(
            current_label_header,
            current_seq_match,
            label_search_pattern,
            label_search_headers,
            label_search_source,
            seq_search_kind,
            seq_search_pattern,
        );
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.sequence_ids = self.current_view_ids.clone();
        }
        for id in removed_ids {
            self.selected_ids.remove(&id);
            if self.cursor_id == Some(id) {
                self.cursor_id = None;
            }
        }
        removed
    }

    fn recompute_search_state(
        &mut self,
        current_label_header: Option<String>,
        current_seq_match: Option<(String, (usize, usize))>,
        label_search_pattern: Option<String>,
        label_search_headers: Option<Vec<String>>,
        label_search_source: Option<LabelSearchSource>,
        seq_search_kind: Option<SearchKind>,
        seq_search_pattern: Option<String>,
    ) {
        match label_search_source {
            Some(LabelSearchSource::Tree) => {
                if let Some(headers) = label_search_headers {
                    let matches = map_headers_to_indices(&self.alignment.headers, &headers);
                    if matches.is_empty() {
                        self.search_state = None;
                    } else {
                        let mut state = build_label_state_from_matches(
                            String::from("<tree>"),
                            matches,
                            self.alignment.headers.len(),
                        );
                        if let Some(target) = current_label_header {
                            if let Some(pos) = state
                                .match_linenums
                                .iter()
                                .position(|&idx| self.alignment.headers.get(idx) == Some(&target))
                            {
                                state.current = pos;
                            }
                        }
                        self.search_state = Some(state);
                        self.label_search_source = Some(LabelSearchSource::Tree);
                    }
                } else {
                    self.search_state = None;
                }
            }
            Some(LabelSearchSource::Regex) | None => {
                if let Some(pattern) = label_search_pattern {
                    match compute_label_search_state(&self.alignment.headers, &pattern) {
                        Ok(mut state) => {
                            if let Some(target) = current_label_header {
                                if let Some(pos) = state.match_linenums.iter().position(|&idx| {
                                    self.alignment.headers.get(idx) == Some(&target)
                                }) {
                                    state.current = pos;
                                }
                            }
                            self.search_state = Some(state);
                            self.label_search_source = Some(LabelSearchSource::Regex);
                            self.tree_selection_range = None;
                        }
                        Err(e) => {
                            self.error_msg(format!("Malformed regex {}.", e));
                            self.search_state = None;
                            self.label_search_source = None;
                            self.tree_selection_range = None;
                        }
                    }
                }
            }
        }

        if let (Some(kind), Some(pattern)) = (seq_search_kind, seq_search_pattern) {
            let state = match kind {
                SearchKind::Regex => {
                    compute_seq_search_state(&self.alignment.sequences, &pattern, kind)
                        .map_err(|e| TermalError::Format(format!("Malformed regex {}.", e)))
                }
                SearchKind::Emboss => compute_emboss_search_state(
                    &self.alignment.headers,
                    &self.alignment.sequences,
                    &pattern,
                    self.emboss_bin_dir.as_deref(),
                ),
            };
            match state {
                Ok(mut state) => {
                    if let Some((header, (start, end))) = current_seq_match {
                        if let Some(seq_index) =
                            self.alignment.headers.iter().position(|h| h == &header)
                        {
                            if let Some(pos) = state.matches.iter().position(|m| {
                                m.seq_index == seq_index && m.start == start && m.end == end
                            }) {
                                state.current_match = pos;
                            }
                        }
                    }
                    self.seq_search_state = Some(state);
                }
                Err(e) => {
                    self.error_msg(format!("Search failed: {}", e));
                    self.seq_search_state = None;
                }
            }
        }

        self.refresh_saved_searches();
        self.recompute_ordering();
    }

    pub fn reject_sequences(
        &mut self,
        ranks: &[usize],
        path: &Path,
    ) -> Result<RejectResult, TermalError> {
        if ranks.is_empty() {
            return Ok(RejectResult {
                count: 0,
                action: RejectAction::AlreadyRejected,
            });
        }
        match self.current_view_kind() {
            ViewKind::Custom => {
                let removed = self.remove_sequences_with_ranks(ranks);
                if removed.is_empty() {
                    return Ok(RejectResult {
                        count: 0,
                        action: RejectAction::RemovedFromView,
                    });
                }
                if let Some(view) = self.views.get_mut(&self.current_view) {
                    view.sequence_ids = self.current_view_ids.clone();
                }
                self.prune_selection_and_cursor();
                return Ok(RejectResult {
                    count: removed.len(),
                    action: RejectAction::RemovedFromView,
                });
            }
            ViewKind::Rejected => {
                return Ok(RejectResult {
                    count: 0,
                    action: RejectAction::AlreadyRejected,
                });
            }
            ViewKind::Original | ViewKind::Filtered => {}
        }
        self.ensure_filtered_rejected_views();
        let mut removed_new: Vec<RemovedSeq> = Vec::new();
        for &rank in ranks {
            if let Some(id) = self.current_view_ids.get(rank).copied() {
                if self.rejected_ids.contains(&id) {
                    continue;
                }
                if let Some(rec) = self.records.get(id) {
                    removed_new.push(RemovedSeq {
                        rank,
                        id,
                        header: rec.header.clone(),
                        sequence: rec.sequence.clone(),
                    });
                }
            }
        }
        if removed_new.is_empty() {
            return Ok(RejectResult {
                count: 0,
                action: RejectAction::AlreadyRejected,
            });
        }
        let backup = if path.exists() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let mut tmp = std::env::temp_dir();
            tmp.push(format!(
                "termal-reject-{}-{}.bak",
                std::process::id(),
                stamp
            ));
            fs::copy(path, &tmp)?;
            Some(tmp)
        } else {
            None
        };
        let mut removed_sorted = removed_new.clone();
        removed_sorted.sort_by_key(|rec| rec.rank);
        let write_result = (|| -> Result<(), TermalError> {
            for rec in &removed_sorted {
                self.append_sequence_fasta(path, &rec.header, &rec.sequence)?;
            }
            Ok(())
        })();
        if let Err(e) = write_result {
            if let Some(ref backup_path) = backup {
                fs::copy(backup_path, path).ok();
            } else if path.exists() {
                fs::remove_file(path).ok();
            }
            return Err(e);
        }
        if let Some(backup_path) = backup {
            fs::remove_file(backup_path).ok();
        }
        for rec in &removed_sorted {
            self.rejected_ids.insert(rec.id);
        }
        self.rebuild_filtered_rejected_views();
        let removed_len = removed_sorted.len();
        if self.current_view != "original" {
            let removed = self.remove_sequences_with_ranks(ranks);
            if removed.is_empty() {
                return Ok(RejectResult {
                    count: removed_len,
                    action: RejectAction::RejectedToFile,
                });
            }
            if let Some(view) = self.views.get_mut(&self.current_view) {
                view.sequence_ids = self.current_view_ids.clone();
            }
        }
        Ok(RejectResult {
            count: removed_len,
            action: RejectAction::RejectedToFile,
        })
    }

    pub fn write_alignment_fasta(&self, path: &Path) -> Result<(), TermalError> {
        let file = fs::File::create(path)?;
        let mut writer = BufWriter::new(file);
        for (header, seq) in self
            .alignment
            .headers
            .iter()
            .zip(self.alignment.sequences.iter())
        {
            writeln!(writer, ">{}", header)?;
            writeln!(writer, "{}", seq)?;
        }
        Ok(())
    }

    pub fn append_sequence_fasta(
        &self,
        path: &Path,
        header: &str,
        sequence: &str,
    ) -> Result<(), TermalError> {
        let file = fs::File::options().create(true).append(true).open(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, ">{}", header)?;
        writeln!(writer, "{}", sequence)?;
        Ok(())
    }

    pub fn rejected_output_path(&self) -> PathBuf {
        self.views
            .get("rejected")
            .map(|view| view.output_path.clone())
            .unwrap_or_else(|| self.output_path_for_view("rejected"))
    }

    pub fn tree_lines(&self) -> &[String] {
        &self.tree_lines
    }

    pub fn tree_panel_width(&self) -> u16 {
        self.tree_panel_width
    }

    pub fn has_tree_panel(&self) -> bool {
        !self.tree_lines.is_empty()
    }

    pub fn tree(&self) -> Option<&TreeNode> {
        self.tree.as_ref()
    }

    pub fn tree_selection_range(&self) -> Option<(usize, usize)> {
        self.tree_selection_range
    }

    fn update_tree_lines_for_selection(&mut self) {
        if let Some(tree) = &self.tree {
            let selection = self.tree_selection_range;
            if let Ok((lines, _order)) = tree_lines_and_order_with_selection(tree, selection) {
                self.tree_lines = lines;
                self.tree_panel_width = self
                    .tree_lines
                    .iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0)
                    .min(u16::MAX as usize) as u16;
            }
        }
    }

    pub fn map_tree_leaf_ranks(&self, leaf_names: &[String]) -> Result<Vec<usize>, TermalError> {
        let mapped_headers = self.map_order_to_headers(leaf_names.to_vec())?;
        let mut index_map: HashMap<&String, usize> = HashMap::new();
        for (idx, header) in self.alignment.headers.iter().enumerate() {
            index_map.insert(header, idx);
        }
        let mut result = Vec::new();
        for header in mapped_headers {
            if let Some(idx) = index_map.get(&header) {
                result.push(*idx);
            }
        }
        Ok(result)
    }

    pub fn realign_with_mafft(&mut self) -> Result<(), TermalError> {
        if self.mafft_bin_dir.is_none() {
            return Err(TermalError::Format(String::from(
                "mafft not configured. Create .termalconfig in $HOME or current directory with mafft_bin_dir.",
            )));
        }
        let mut input_path = std::env::temp_dir();
        let unique = format!("termal-mafft-{}.fa", std::process::id());
        input_path.push(unique);
        self.write_alignment_fasta(&input_path)?;

        let mut output_path = std::env::temp_dir();
        let unique_out = format!("termal-mafft-{}.out.fa", std::process::id());
        output_path.push(unique_out);

        let tool_path = self
            .mafft_bin_dir
            .as_ref()
            .map(|dir| dir.join("mafft"))
            .unwrap_or_else(|| PathBuf::from("mafft"));
        let output = Command::new(tool_path)
            .arg("--treeout")
            .arg("--reorder")
            .arg(&input_path)
            .output()
            .map_err(|e| TermalError::Format(format!("Failed to run mafft: {}", e)))?;
        if !output.status.success() {
            let msg = String::from_utf8_lossy(&output.stderr);
            return Err(TermalError::Format(format!("mafft failed: {}", msg)));
        }
        fs::write(&output_path, output.stdout)?;

        let tree_path = PathBuf::from(format!("{}.tree", input_path.display()));
        let tree_text = fs::read_to_string(&tree_path)?;
        let tree = parse_newick(&tree_text)?;
        let (lines, order) = tree_lines_and_order(&tree)?;

        let seq_file = read_fasta_file(&output_path)?;
        let mafft_alignment = Alignment::from_file(seq_file);
        let view_ids = self.current_view_ids.clone();
        self.update_records_from_alignment(&mafft_alignment, &view_ids)?;
        self.alignment = self.build_alignment_for_ids(&view_ids);
        self.search_state = None;
        self.seq_search_state = None;
        self.label_search_source = None;
        self.tree_selection_range = None;
        self.refresh_saved_searches();
        self.set_user_ordering(order)?;
        self.tree_lines = lines;
        self.tree_panel_width = self
            .tree_lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
            .min(u16::MAX as usize) as u16;
        self.tree = Some(tree);
        self.tree_newick = Some(tree_text);
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.tree = self.tree.clone();
            view.tree_newick = self.tree_newick.clone();
            view.tree_lines = self.tree_lines.clone();
            view.tree_panel_width = self.tree_panel_width;
        }
        self.recompute_ordering();

        fs::remove_file(&input_path).ok();
        fs::remove_file(&output_path).ok();
        fs::remove_file(&tree_path).ok();
        Ok(())
    }

    pub fn set_user_ordering(&mut self, order: Vec<String>) -> Result<(), TermalError> {
        let mapped = self.map_order_to_headers(order)?;
        self.user_ordering = Some(mapped);
        self.ordering_criterion = User;
        self.recompute_ordering();
        Ok(())
    }

    fn map_order_to_headers(&self, order: Vec<String>) -> Result<Vec<String>, TermalError> {
        let expected: HashSet<String> = self.alignment.headers.iter().cloned().collect();
        let mut token_map: HashMap<String, String> = HashMap::new();
        let mut normalized_map: HashMap<String, String> = HashMap::new();
        for header in &self.alignment.headers {
            let normalized = normalize_tree_label(header);
            insert_unique(&mut normalized_map, normalized, header)?;
            let token = header.split_whitespace().next().unwrap_or("").to_string();
            if token.is_empty() {
                continue;
            }
            insert_unique(&mut token_map, token.clone(), header)?;
            let token_norm = normalize_tree_label(&token);
            insert_unique(&mut token_map, token_norm, header)?;
        }

        let mut mapped: Vec<String> = Vec::with_capacity(order.len());
        for name in order {
            let normalized = normalize_tree_label(&name);
            if expected.contains(&name) {
                mapped.push(name);
                continue;
            }
            if let Some(header) = normalized_map.get(&name) {
                mapped.push(header.clone());
                continue;
            }
            if let Some(header) = normalized_map.get(&normalized) {
                mapped.push(header.clone());
                continue;
            }
            if let Some(header) = token_map.get(&name) {
                mapped.push(header.clone());
                continue;
            }
            if let Some(header) = token_map.get(&normalized) {
                mapped.push(header.clone());
                continue;
            }
            return Err(TermalError::Format(format!(
                "Tree leaf does not match header: {}",
                name
            )));
        }

        let provided: HashSet<String> = mapped.iter().cloned().collect();
        if expected.len() != provided.len() || expected != provided {
            return Err(TermalError::Format(String::from(
                "Tree leaves do not match alignment headers",
            )));
        }

        Ok(mapped)
    }

    fn refresh_saved_searches(&mut self) {
        let sequences = &self.alignment.sequences;
        for entry in &mut self.search_registry.searches {
            let state = match entry.kind {
                SearchKind::Regex => compute_seq_search_state(sequences, &entry.query, entry.kind)
                    .map_err(|e| TermalError::Format(format!("Malformed regex: {}", e))),
                SearchKind::Emboss => compute_emboss_search_state(
                    &self.alignment.headers,
                    sequences,
                    &entry.query,
                    self.emboss_bin_dir.as_deref(),
                ),
            };
            entry.spans_by_seq = match state {
                Ok(state) => state.spans_by_seq,
                Err(_) => vec![Vec::new(); sequences.len()],
            };
        }
    }

    pub fn refresh_saved_searches_public(&mut self) {
        self.refresh_saved_searches();
    }

    pub fn notes(&self) -> &str {
        &self.notes
    }

    pub fn set_notes(&mut self, notes: String) {
        self.notes = notes;
    }

    pub fn view_notes(&self) -> &str {
        &self.view_notes
    }

    pub fn set_view_notes(&mut self, notes: String) {
        self.view_notes = notes;
        if let Some(view) = self.views.get_mut(&self.current_view) {
            view.notes = self.view_notes.clone();
        }
    }

    // Messages

    pub fn current_message(&self) -> &CurrentMessage {
        &self.current_msg
    }

    pub fn clear_msg(&mut self) {
        self.current_msg = CurrentMessage {
            prefix: String::from(""),
            message: String::from(""),
            kind: MessageKind::Info,
        }
    }

    pub fn info_msg(&mut self, msg: impl Into<String>) {
        self.current_msg = CurrentMessage {
            prefix: String::from(""),
            message: msg.into(),
            kind: MessageKind::Info,
        };
    }

    pub fn recompute_current_seq_search(&mut self) {
        let (kind, pattern, current) = match &self.seq_search_state {
            Some(state) => (state.kind, state.pattern.clone(), state.current_match),
            None => return,
        };
        match kind {
            SearchKind::Regex => self.regex_search_sequences(&pattern),
            SearchKind::Emboss => self.emboss_search_sequences(&pattern),
        }
        if let Some(state) = &mut self.seq_search_state {
            if current < state.matches.len() {
                state.current_match = current;
            }
        }
    }

    pub fn warning_msg(&mut self, msg: impl Into<String>) {
        self.current_msg = CurrentMessage {
            prefix: String::from("WARNING: "),
            message: msg.into(),
            kind: MessageKind::Warning,
        };
    }

    pub fn error_msg(&mut self, msg: impl Into<String>) {
        self.current_msg = CurrentMessage {
            prefix: String::from("ERROR: "),
            message: msg.into(),
            kind: MessageKind::Error,
        };
    }

    pub fn debug_msg(&mut self, msg: impl Into<String>) {
        self.current_msg = CurrentMessage {
            prefix: String::from(""),
            message: msg.into(),
            kind: MessageKind::Debug,
        };
    }

    pub fn argument_msg(&mut self, pfx: impl Into<String>, msg: impl Into<String>) {
        self.current_msg = CurrentMessage {
            prefix: pfx.into(),
            message: msg.into(),
            kind: MessageKind::Argument,
        };
    }

    pub fn add_argument_char(&mut self, c: char) {
        self.current_msg.message.push(c);
        self.current_msg.kind = MessageKind::Argument;
    }

    pub fn pop_argument_char(&mut self) {
        self.current_msg.message.pop();
        self.current_msg.kind = MessageKind::Argument;
    }
}

fn normalize_tree_label(label: &str) -> String {
    let trimmed = label.trim();
    let stripped = strip_numeric_prefix(trimmed);
    stripped
        .chars()
        .map(|c| match c {
            c if c.is_whitespace() => '_',
            '.' => '_',
            _ => c,
        })
        .collect()
}

fn strip_numeric_prefix(label: &str) -> &str {
    let bytes = label.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx < bytes.len() && bytes[idx] == b'_' {
        return &label[idx + 1..];
    }
    label
}

fn insert_unique(
    map: &mut HashMap<String, String>,
    key: String,
    header: &str,
) -> Result<(), TermalError> {
    if key.is_empty() {
        return Ok(());
    }
    if map.contains_key(&key) && map.get(&key).map(String::as_str) != Some(header) {
        return Err(TermalError::Format(format!(
            "Non-unique header token: {}",
            key
        )));
    }
    map.insert(key, header.to_string());
    Ok(())
}

// Computes an ordering WRT an array, that is, an array of indices of elements of the source array,
// after sorting. Eg [3, -2, 7] -> [1, 0, 2], because the smalllest element has index 1, the next
// has index 0, and the largest has index 2 (in the original array).
fn order<T: PartialOrd>(elems: &[T]) -> Vec<usize> {
    // let result: Vec<usize> = Vec::with_capacity(elems.len());
    let init_order: Vec<usize> = (0..elems.len()).collect();
    let zip_iter = init_order.iter().zip(elems);
    let mut unsorted_pairs: Vec<(&usize, &T)> = zip_iter.collect();
    unsorted_pairs.sort_by(|(_, t1), (_, t2)| t1.partial_cmp(t2).expect("Unorder!"));
    unsorted_pairs
        .into_iter()
        .map(|(u, _)| *u)
        .collect::<Vec<usize>>()
}

fn ungapped_seq_and_map(seq: &str) -> (String, Vec<usize>) {
    let mut ungapped = String::with_capacity(seq.len());
    let mut map: Vec<usize> = Vec::with_capacity(seq.len());
    for (idx, ch) in seq.chars().enumerate() {
        if is_gap(ch) {
            continue;
        }
        ungapped.push(ch);
        map.push(idx);
    }
    (ungapped, map)
}

fn is_gap(c: char) -> bool {
    matches!(c, '-' | '.' | ' ')
}

impl SearchRegistry {
    fn new(palette: Vec<SearchColor>) -> Self {
        Self {
            searches: Vec::new(),
            palette,
            next_color_index: 0,
        }
    }

    fn entries(&self) -> &[SearchEntry] {
        &self.searches
    }

    fn add_search(
        &mut self,
        name: String,
        query: String,
        kind: SearchKind,
        spans_by_seq: Vec<Vec<(usize, usize)>>,
    ) {
        let color = self.palette[self.next_color_index % self.palette.len()];
        self.next_color_index += 1;
        let id = self.searches.len() + 1;
        self.searches.push(SearchEntry {
            id,
            name,
            query,
            kind,
            enabled: true,
            color,
            spans_by_seq,
        });
    }

    fn delete(&mut self, index: usize) -> bool {
        if index >= self.searches.len() {
            return false;
        }
        self.searches.remove(index);
        for (idx, entry) in self.searches.iter_mut().enumerate() {
            entry.id = idx + 1;
        }
        true
    }

    fn toggle(&mut self, index: usize) -> bool {
        if let Some(entry) = self.searches.get_mut(index) {
            entry.enabled = !entry.enabled;
            true
        } else {
            false
        }
    }

    fn set_palette(&mut self, palette: Vec<SearchColor>) {
        self.palette = palette;
        if self.palette.is_empty() {
            self.palette = DEFAULT_SEARCH_PALETTE.to_vec();
        }
        self.next_color_index %= self.palette.len();
    }
}

fn compute_seq_search_state(
    sequences: &[String],
    pattern: &str,
    kind: SearchKind,
) -> Result<SeqSearchState, regex::Error> {
    let re = RegexBuilder::new(pattern).case_insensitive(true).build()?;
    let mut spans_by_seq: Vec<Vec<(usize, usize)>> = Vec::with_capacity(sequences.len());
    let mut total_matches = 0;
    let mut sequences_with_matches = 0;
    let mut matches: Vec<SeqMatch> = Vec::new();
    for seq in sequences {
        let (ungapped, map) = ungapped_seq_and_map(seq);
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for m in re.find_iter(&ungapped) {
            if m.start() == m.end() {
                continue;
            }
            if m.end() == 0 || m.end() > map.len() {
                continue;
            }
            let g_start = map[m.start()];
            let g_end = map[m.end() - 1] + 1;
            spans.push((g_start, g_end));
        }
        if !spans.is_empty() {
            sequences_with_matches += 1;
            total_matches += spans.len();
        }
        spans_by_seq.push(spans);
    }
    for (seq_index, spans) in spans_by_seq.iter().enumerate() {
        for (start, end) in spans {
            matches.push(SeqMatch {
                seq_index,
                start: *start,
                end: *end,
            });
        }
    }
    Ok(SeqSearchState {
        kind,
        pattern: pattern.to_string(),
        spans_by_seq,
        total_matches,
        sequences_with_matches,
        matches,
        current_match: 0,
    })
}

fn compute_label_search_state(
    headers: &[String],
    pattern: &str,
) -> Result<SearchState, regex::Error> {
    let re = RegexBuilder::new(pattern).case_insensitive(true).build()?;
    let matches: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter_map(|(i, line)| re.is_match(line).then_some(i))
        .collect();

    Ok(SearchState {
        pattern: String::from(pattern),
        regex: re,
        match_linenums: matches,
        current: 0,
    })
}

fn build_label_state_from_matches(
    pattern: String,
    matches: Vec<usize>,
    header_len: usize,
) -> SearchState {
    let mut filtered: Vec<usize> = Vec::new();
    for idx in matches {
        if idx < header_len {
            filtered.push(idx);
        }
    }
    let regex = Regex::new("$^").unwrap();
    SearchState {
        pattern,
        regex,
        match_linenums: filtered,
        current: 0,
    }
}

fn map_headers_to_indices(headers: &[String], subset: &[String]) -> Vec<usize> {
    let mut indices: Vec<usize> = Vec::new();
    for header in subset {
        if let Some(pos) = headers.iter().position(|h| h == header) {
            indices.push(pos);
        }
    }
    indices
}

fn parse_color_value(value: &Value) -> Result<SearchColor, TermalError> {
    match value {
        Value::String(s) => {
            let hex = HexColor::parse_rgb(s).map_err(|e| format!("Bad color {}: {}", s, e))?;
            Ok((hex.r, hex.g, hex.b))
        }
        Value::Array(items) if items.len() == 3 => {
            let r = items[0].as_u64().unwrap_or(0).min(255) as u8;
            let g = items[1].as_u64().unwrap_or(0).min(255) as u8;
            let b = items[2].as_u64().unwrap_or(0).min(255) as u8;
            Ok((r, g, b))
        }
        _ => Err(TermalError::Format("Invalid color value".to_string())),
    }
}

fn compute_emboss_search_state(
    headers: &[String],
    sequences: &[String],
    pattern: &str,
    emboss_bin_dir: Option<&Path>,
) -> Result<SeqSearchState, TermalError> {
    let emboss_bin_dir = emboss_bin_dir.ok_or_else(|| {
        TermalError::Format(String::from(
            "Emboss tools not configured. Create .termalconfig in $HOME or current directory with emboss_bin_dir.",
        ))
    })?;
    let is_nucleic = sequences
        .iter()
        .all(|seq| seq.chars().all(|c| is_gap(c) || is_acgt(c)));
    let tool = if is_nucleic { "fuzznuc" } else { "fuzzpro" };
    let tool_path = emboss_bin_dir.join(tool);
    let (pmis, emboss_pattern) = parse_emboss_query(pattern);
    let emboss_pattern = emboss_pattern.to_ascii_uppercase();

    let tmp_path = emboss_temp_fasta(headers, sequences)?;
    let mut cmd = std::process::Command::new(tool_path);
    cmd.arg("-seq")
        .arg(&tmp_path)
        .arg("-pat")
        .arg(&emboss_pattern)
        .arg("-out")
        .arg("stdout")
        .arg("-rformat")
        .arg("gff");
    if let Some(mismatches) = pmis {
        cmd.arg("-pmis").arg(mismatches.to_string());
    }
    let output = cmd
        .output()
        .map_err(|e| TermalError::Format(format!("Failed to run {}: {}", tool, e)))?;
    fs::remove_file(&tmp_path).ok();

    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(TermalError::Format(format!("{} failed: {}", tool, msg)));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_gff_to_state(headers, sequences, &stdout, pattern)
}

fn parse_emboss_query(query: &str) -> (Option<u32>, &str) {
    let trimmed = query.trim();
    let mut parts = trimmed.splitn(2, ' ');
    let Some(first) = parts.next() else {
        return (None, trimmed);
    };
    let Some(rest) = parts.next() else {
        return (None, trimmed);
    };
    if !first.chars().all(|c| c.is_ascii_digit()) {
        return (None, trimmed);
    }
    let Ok(value) = first.parse::<u32>() else {
        return (None, trimmed);
    };
    let pattern = rest.trim_start();
    if pattern.is_empty() {
        return (None, trimmed);
    }
    (Some(value), pattern)
}

fn emboss_temp_fasta(headers: &[String], sequences: &[String]) -> Result<PathBuf, TermalError> {
    let mut path = std::env::temp_dir();
    let unique = format!("termal-emboss-{}.fa", std::process::id());
    path.push(unique);
    let file = fs::File::create(&path)?;
    let mut writer = BufWriter::new(file);
    for (header, seq) in headers.iter().zip(sequences.iter()) {
        let ungapped: String = seq
            .chars()
            .filter(|c| !is_gap(*c))
            .map(|c| c.to_ascii_uppercase())
            .collect();
        writeln!(writer, ">{}", header)?;
        writeln!(writer, "{}", ungapped)?;
    }
    Ok(path)
}

fn parse_gff_to_state(
    headers: &[String],
    sequences: &[String],
    gff: &str,
    pattern: &str,
) -> Result<SeqSearchState, TermalError> {
    let mut header_to_index: HashMap<&str, usize> = HashMap::new();
    for (idx, header) in headers.iter().enumerate() {
        header_to_index.insert(header.as_str(), idx);
        if let Some(token) = header.split_whitespace().next() {
            header_to_index.entry(token).or_insert(idx);
        }
    }
    let mut spans_by_seq: Vec<Vec<(usize, usize)>> = vec![Vec::new(); sequences.len()];
    for line in gff.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        let seqid = parts[0];
        let start: usize = parts[3].parse().unwrap_or(0);
        let end: usize = parts[4].parse().unwrap_or(0);
        if start == 0 || end == 0 {
            continue;
        }
        let Some(&seq_index) = header_to_index.get(seqid) else {
            continue;
        };
        let map = ungapped_to_gapped_map(&sequences[seq_index]);
        if start > map.len() || end > map.len() || start > end {
            continue;
        }
        let g_start = map[start - 1];
        let g_end = map[end - 1] + 1;
        spans_by_seq[seq_index].push((g_start, g_end));
    }
    let mut total_matches = 0;
    let mut sequences_with_matches = 0;
    let mut matches: Vec<SeqMatch> = Vec::new();
    for (seq_index, spans) in spans_by_seq.iter().enumerate() {
        if !spans.is_empty() {
            sequences_with_matches += 1;
            total_matches += spans.len();
        }
        for (start, end) in spans {
            matches.push(SeqMatch {
                seq_index,
                start: *start,
                end: *end,
            });
        }
    }
    Ok(SeqSearchState {
        kind: SearchKind::Emboss,
        pattern: pattern.to_string(),
        spans_by_seq,
        total_matches,
        sequences_with_matches,
        matches,
        current_match: 0,
    })
}

fn ungapped_to_gapped_map(seq: &str) -> Vec<usize> {
    let mut map: Vec<usize> = Vec::new();
    for (idx, ch) in seq.chars().enumerate() {
        if is_gap(ch) {
            continue;
        }
        map.push(idx);
    }
    map
}

fn is_acgt(c: char) -> bool {
    matches!(c, 'A' | 'C' | 'G' | 'T' | 'a' | 'c' | 'g' | 't')
}
fn next_available_output_path(original: &str, tag: &str) -> PathBuf {
    let path = Path::new(original);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(original);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name);
    let ext = path.extension().and_then(|name| name.to_str());
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    for idx in 1..=9999 {
        let suffix = format!("{}{:02}", tag, idx);
        let new_name = match ext {
            Some(ext) => format!("{}.{}.{}", stem, suffix, ext),
            None => format!("{}.{}", stem, suffix),
        };
        let candidate = if parent.as_os_str().is_empty() {
            PathBuf::from(&new_name)
        } else {
            parent.join(&new_name)
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{}.{}", stem, tag))
}

#[cfg(test)]
mod tests {

    use super::{SearchColorConfig, ToolsConfig};
    use crate::{
        alignment::Alignment,
        app::{order, App, SearchKind, SeqMatch, SeqOrdering},
        tree::{parse_newick, tree_lines_and_order},
    };
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_order_00() {
        assert_eq!(vec![2, 1, 0], order(&vec![20.0, 15.0, 10.0]));
    }

    #[test]
    fn test_order_05() {
        assert_eq!(
            vec![3, 2, 0, 1, 4],
            order(&vec![12.23, 34.89, 7.0, -23.2, 100.0]),
        );
    }

    #[test]
    fn test_order_10() {
        // Reverse order
        let orig = vec![3.0, 2.0, 5.0, 1.0, 4.0];
        let direct_order = order(&orig);
        assert_eq!(vec![3, 1, 0, 4, 2], direct_order);
        let reverse_order = order(&direct_order);
        assert_eq!(vec![2, 1, 4, 0, 3], reverse_order);
    }

    #[test]
    fn test_ordering_00() {
        let hdrs = vec![
            String::from("R1"),
            String::from("R2"),
            String::from("R3"),
            String::from("R4"),
        ];
        let seqs = vec![
            String::from("catgcatatg"), // 0 diffs WRT consensus
            String::from("cCtgcatatg"), // 1 diffs WRT consensus
            String::from("catAcTtatg"), // 2 diffs WRT consensus
            String::from("caGgAataAg"), // 3 diffs WRT consensus
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        assert_eq!(app.ordering, vec![0, 1, 2, 3]);
        app.next_ordering_criterion();
        // Ordering is now by increasing metric, which is (by default) %id WRT consensus. Given the above
        // sequences, this effectively reverses the order.
        assert_eq!(app.ordering, vec![3, 2, 1, 0]);
        app.next_ordering_criterion();
        // Now by decreasing metric, which in this case is (by construction) the same as the
        // original.
        assert_eq!(app.ordering, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_ordering_05() {
        let hdrs = vec![
            String::from("R1"),
            String::from("R2"),
            String::from("R3"),
            String::from("R4"),
            String::from("R5"),
        ];
        let seqs = vec![
            String::from("catgcatatg"), // 0 diffs WRT consensus
            String::from("caGgAaCaAg"), // 4 diffs WRT consensus
            String::from("catAcTtatg"), // 2 diffs WRT consensus
            String::from("cCtgcatatg"), // 1 diffs WRT consensus
            String::from("caGgAataAg"), // 3 diffs WRT consensus
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        assert_eq!(app.ordering, vec![0, 1, 2, 3, 4]);
        app.next_ordering_criterion();
        assert_eq!(app.ordering, vec![1, 4, 2, 3, 0]);
        assert_eq!(app.reverse_ordering, vec![4, 0, 2, 3, 1]);
        app.next_ordering_criterion();
        assert_eq!(app.ordering, vec![0, 3, 2, 4, 1]);
        assert_eq!(app.reverse_ordering, vec![0, 4, 2, 1, 3]);
    }

    #[test]
    fn test_termal_config_from_value() {
        let value = json!({
            "palette": ["#010203"],
            "current_search": [4, 5, 6],
            "emboss_bin_dir": "/opt/emboss",
            "mafft_bin_dir": "/opt/mafft"
        });
        let colors = SearchColorConfig::from_value(&value);
        assert_eq!(colors.palette[0], (1, 2, 3));
        assert_eq!(colors.current_search, (4, 5, 6));

        let tools = ToolsConfig::from_value(&value);
        assert_eq!(tools.emboss_bin_dir, Some(PathBuf::from("/opt/emboss")));
        assert_eq!(tools.mafft_bin_dir, Some(PathBuf::from("/opt/mafft")));
    }

    #[test]
    fn test_update_records_from_alignment() {
        let hdrs = vec![String::from("A"), String::from("B")];
        let seqs = vec![String::from("AA"), String::from("CC")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);

        let updated = Alignment::from_vecs(
            vec![String::from("A"), String::from("B")],
            vec![String::from("TT"), String::from("GG")],
        );
        app.update_records_from_alignment(&updated, &[1]).unwrap();

        assert_eq!(app.records[0].sequence, "AA");
        assert_eq!(app.records[1].sequence, "GG");
    }

    #[test]
    fn test_ordering_status_label() {
        let hdrs = vec![String::from("R1"), String::from("R2")];
        let seqs = vec![String::from("AA"), String::from("AA")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        assert_eq!(app.ordering_status_label(), "o:original");
        app.next_ordering_criterion();
        assert_eq!(app.ordering_status_label(), "o:%id↑");
        app.next_ordering_criterion();
        assert_eq!(app.ordering_status_label(), "o:%id↓");
        app.next_ordering_criterion();
        assert_eq!(app.ordering_status_label(), "o:match");
        app.set_user_ordering(vec![String::from("R1"), String::from("R2")])
            .unwrap();
        assert_eq!(app.ordering_status_label(), "o:tree");
    }

    #[test]
    fn test_create_view_from_selection() {
        let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
        let seqs = vec![String::from("AA"), String::from("BB"), String::from("CC")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.select_label_by_rank(1).unwrap();
        app.create_view_from_selection("picked").unwrap();

        let view = app.views.get("picked").expect("view");
        assert_eq!(view.sequence_ids, vec![1]);
        assert!(view.tree.is_none());
        assert_eq!(
            view.user_ordering.as_ref().unwrap(),
            &vec!["R2".to_string()]
        );
    }

    #[test]
    fn test_select_label_by_rank() {
        let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
        let seqs = vec![String::from("AA"), String::from("BB"), String::from("CC")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.select_label_by_rank(1).unwrap();
        assert_eq!(app.cursor_rank(), Some(1));
        assert!(app.is_label_selected(1));
    }

    #[test]
    fn test_rank_to_screenline_00() {
        let hdrs = vec![
            String::from("R1"),
            String::from("R2"),
            String::from("R3"),
            String::from("R4"),
            String::from("R5"),
        ];
        let seqs = vec![
            String::from("catgcatatg"), // 0 diffs WRT consensus
            String::from("caGgAaCaAg"), // 4 diffs WRT consensus
            String::from("catAcTtatg"), // 2 diffs WRT consensus
            String::from("cCtgcatatg"), // 1 diffs WRT consensus
            String::from("caGgAataAg"), // 3 diffs WRT consensus
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        assert_eq!(app.ordering, vec![0, 1, 2, 3, 4]);
        // Until ordering changes, rank == screenline
        assert_eq!(0, app.rank_to_screenline(0));
        assert_eq!(1, app.rank_to_screenline(1));
        assert_eq!(2, app.rank_to_screenline(2));
        assert_eq!(3, app.rank_to_screenline(3));
        assert_eq!(4, app.rank_to_screenline(4));
        app.next_ordering_criterion();
        assert_eq!(app.ordering, vec![1, 4, 2, 3, 0]);
        assert_eq!(app.reverse_ordering, vec![4, 0, 2, 3, 1]);
        // Now the ordering is by metric, so rank != screenline
        assert_eq!(app.rank_to_screenline(0), 4);
        assert_eq!(app.rank_to_screenline(1), 0);
        assert_eq!(app.rank_to_screenline(2), 2);
        assert_eq!(app.rank_to_screenline(3), 3);
        assert_eq!(app.rank_to_screenline(4), 1);
    }

    #[test]
    fn test_regex_lbl_search_10() {
        let hdrs = vec![
            String::from("Accipiter"),
            String::from("Aquila"),
            String::from("Milvus"),
            String::from("Buteo"),
            String::from("Pernis"),
        ];
        let seqs = vec![
            String::from("catgcatatg"), // 0 diffs WRT consensus
            String::from("caGgAaCaAg"), // 4 diffs WRT consensus
            String::from("catAcTtatg"), // 2 diffs WRT consensus
            String::from("cCtgcatatg"), // 1 diffs WRT consensus
            String::from("caGgAataAg"), // 3 diffs WRT consensus
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.regex_search_labels("^A");
        match app.search_state {
            Some(state) => {
                assert_eq!(state.pattern, "^A");
                assert_eq!(state.match_linenums, vec![0, 1]);
                assert_eq!(state.current, 0);
            }
            None => panic!(),
        }
    }

    #[test]
    fn test_regex_lbl_search_case_insensitive() {
        let hdrs = vec![String::from("Accipiter"), String::from("Aquila")];
        let seqs = vec![String::from("catg"), String::from("catg")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.regex_search_labels("^a");
        assert!(app.is_label_selected(0));
        assert!(app.is_label_selected(1));
    }

    #[test]
    fn test_regex_seq_search_spans() {
        let hdrs = vec![String::from("R1")];
        let seqs = vec![String::from("A-C--GT")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.regex_search_sequences("CG");
        let spans = app.seq_search_spans().unwrap();
        assert_eq!(spans[0], vec![(2, 6)]);
        let counts = app.seq_search_counts().unwrap();
        assert_eq!(counts, (1, 1));
        let cur = app.current_seq_match().unwrap();
        assert_eq!(
            cur,
            SeqMatch {
                seq_index: 0,
                start: 2,
                end: 6
            }
        );
    }

    #[test]
    fn test_search_ordering_groups_matches() {
        let hdrs = vec![
            String::from("R1"),
            String::from("R2"),
            String::from("R3"),
            String::from("R4"),
        ];
        let seqs = vec![
            String::from("AA--"),
            String::from("BBBB"),
            String::from("A-A-"),
            String::from("CCCC"),
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.regex_search_sequences("aa");
        app.next_ordering_criterion();
        app.next_ordering_criterion();
        app.next_ordering_criterion();
        assert_eq!(app.ordering, vec![0, 2, 1, 3]);
    }

    #[test]
    fn test_remove_sequences_preserves_search_state() {
        let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
        let seqs = vec![String::from("AA"), String::from("BB"), String::from("AA")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.regex_search_sequences("AA");
        app.next_ordering_criterion();
        app.next_ordering_criterion();
        app.next_ordering_criterion();
        app.remove_sequences(&[1]);
        assert_eq!(app.get_seq_ordering(), SeqOrdering::SearchMatch);
        assert_eq!(app.current_seq_search_pattern(), Some("AA"));
        assert_eq!(app.seq_search_spans().unwrap().len(), 2);
    }

    #[test]
    fn test_remove_sequences_prunes_user_ordering() {
        let hdrs = vec![
            String::from("R1"),
            String::from("R2"),
            String::from("R3"),
            String::from("R4"),
        ];
        let seqs = vec![
            String::from("AA"),
            String::from("BB"),
            String::from("CC"),
            String::from("DD"),
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.set_user_ordering(vec![
            String::from("R3"),
            String::from("R1"),
            String::from("R4"),
            String::from("R2"),
        ])
        .unwrap();
        app.remove_sequences(&[1]);
        assert_eq!(app.alignment.headers.len(), 3);
        assert_eq!(app.user_ordering.as_ref().unwrap().len(), 3);
        assert!(!app
            .user_ordering
            .as_ref()
            .unwrap()
            .contains(&String::from("R2")));
    }

    #[test]
    fn test_remove_sequences_preserves_ordering_lengths() {
        let hdrs = vec![
            String::from("R1"),
            String::from("R2"),
            String::from("R3"),
            String::from("R4"),
        ];
        let seqs = vec![
            String::from("AA"),
            String::from("BB"),
            String::from("AA"),
            String::from("CC"),
        ];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);

        app.next_ordering_criterion();
        app.remove_sequences(&[1]);
        assert_eq!(app.ordering.len(), app.alignment.num_seq());
        assert_eq!(app.reverse_ordering.len(), app.alignment.num_seq());

        app.regex_search_sequences("AA");
        app.next_ordering_criterion();
        app.next_ordering_criterion();
        app.next_ordering_criterion();
        app.remove_sequences(&[0]);
        assert_eq!(app.ordering.len(), app.alignment.num_seq());
        assert_eq!(app.reverse_ordering.len(), app.alignment.num_seq());
    }

    #[test]
    fn test_session_save_and_load() {
        let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
        let seqs = vec![String::from("AA"), String::from("BB"), String::from("AA")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        let tree = parse_newick("(R1,(R2,R3));").unwrap();
        let (lines, _order) = tree_lines_and_order(&tree).unwrap();
        app.tree_lines = lines;
        app.tree_panel_width = app
            .tree_lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
            .min(u16::MAX as usize) as u16;
        app.tree_newick = Some(String::from("(R1,(R2,R3));"));
        app.tree = Some(tree);
        app.set_notes(String::from("Session notes"));
        app.set_view_notes(String::from("View notes"));
        app.add_saved_search_with_kind(
            String::from("motif"),
            String::from("AA"),
            SearchKind::Regex,
        )
        .unwrap();
        app.regex_search_sequences("AA");
        app.set_label_matches_from_tree(vec![0, 2], (0, 2));

        let mut path = PathBuf::from(std::env::temp_dir());
        path.push("termal-test-session.trml");
        let _ = std::fs::remove_file(&path);
        app.save_session(&path).unwrap();

        let loaded = App::from_session_file(&path).unwrap();
        assert_eq!(loaded.alignment.headers.len(), 3);
        assert_eq!(loaded.tree_lines.len(), 3);
        assert!(loaded.tree.is_some());
        assert_eq!(loaded.saved_searches().len(), 1);
        assert_eq!(loaded.current_seq_search_pattern(), Some("AA"));
        assert_eq!(loaded.selection_ranks(), vec![0, 2]);
        assert_eq!(loaded.notes(), "Session notes");
        assert_eq!(loaded.view_notes(), "View notes");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_tree_ordering_maps_header_tokens() {
        let hdrs = vec![String::from("seq 1"), String::from("seq2")];
        let seqs = vec![String::from("AA"), String::from("AA")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.set_user_ordering(vec![String::from("seq"), String::from("seq2")])
            .unwrap();
        assert_eq!(
            app.user_ordering.unwrap(),
            vec![String::from("seq 1"), String::from("seq2")]
        );
    }

    #[test]
    fn test_tree_ordering_maps_underscored_headers() {
        let hdrs = vec![String::from("1 CELEG-F08G5 1a"), String::from("seq2")];
        let seqs = vec![String::from("AA"), String::from("AA")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.set_user_ordering(vec![String::from("1_CELEG-F08G5_1a"), String::from("seq2")])
            .unwrap();
        assert_eq!(
            app.user_ordering.unwrap(),
            vec![String::from("1 CELEG-F08G5 1a"), String::from("seq2")]
        );
    }

    #[test]
    fn test_tree_ordering_strips_numeric_prefix_and_dots() {
        let hdrs = vec![String::from("CELEG-F08G5.1a"), String::from("seq2")];
        let seqs = vec![String::from("AA"), String::from("AA")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.set_user_ordering(vec![String::from("1_CELEG-F08G5_1a"), String::from("seq2")])
            .unwrap();
        assert_eq!(
            app.user_ordering.unwrap(),
            vec![String::from("CELEG-F08G5.1a"), String::from("seq2")]
        );
    }

    #[test]
    fn test_search_registry_add_toggle_delete() {
        let hdrs = vec![String::from("R1")];
        let seqs = vec![String::from("ACGT")];
        let aln = Alignment::from_vecs(hdrs, seqs);
        let mut app = App::new("TEST", aln, None);
        app.regex_search_sequences("CG");
        let query = app.current_seq_search_pattern().unwrap().to_string();
        app.add_saved_search(query.clone(), query).unwrap();
        assert_eq!(app.saved_searches().len(), 1);
        assert!(app.saved_searches()[0].enabled);
        assert!(app.toggle_saved_search(0));
        assert!(!app.saved_searches()[0].enabled);
        assert!(app.delete_saved_search(0));
        assert!(app.saved_searches().is_empty());
    }

    #[test]
    fn test_parse_emboss_query() {
        assert_eq!(super::parse_emboss_query("2 ABC"), (Some(2), "ABC"));
        assert_eq!(super::parse_emboss_query("10   A*B"), (Some(10), "A*B"));
        assert_eq!(super::parse_emboss_query("ABC"), (None, "ABC"));
        assert_eq!(super::parse_emboss_query("2"), (None, "2"));
        assert_eq!(super::parse_emboss_query("2  "), (None, "2"));
    }

    #[test]
    fn test_parse_gff_matches_header_token() {
        let headers = vec![String::from("seq 1"), String::from("seq2")];
        let sequences = vec![String::from("ABCD"), String::from("EFGH")];
        let gff = "seq\tsrc\tfeat\t2\t4\t.\t.\t.\tID=seq.1\n";
        let state = super::parse_gff_to_state(&headers, &sequences, gff, "TEST").unwrap();
        assert_eq!(state.spans_by_seq[0], vec![(1, 4)]);
        assert!(state.spans_by_seq[1].is_empty());
    }
}
