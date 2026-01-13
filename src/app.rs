// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::{
    collections::HashMap,
    fmt, fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use hex_color::HexColor;
use regex::{Regex, RegexBuilder};
use serde_json::Value;

use crate::{
    alignment::Alignment,
    app::Metric::{PctIdWrtConsensus, SeqLen},
    app::SeqOrdering::{MetricDecr, MetricIncr, SearchMatch, SourceFile, User},
    errors::TermalError,
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
#[derive(Clone, Copy)]
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

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metric = match self {
            PctIdWrtConsensus => "%id (cons)",
            SeqLen => "seq len",
        };
        write!(f, "{}", metric)
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
    // Whether a header matches or not; used when iterating over all headers and determining
    // whether to highlight or not. As many elements as there are sequences (and hence headers) in the alignment.
    hdr_match_status: Vec<bool>,
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

pub struct EmbossConfig {
    pub emboss_bin_dir: Option<PathBuf>,
}

impl EmbossConfig {
    pub fn from_file(path: &str) -> Result<Self, TermalError> {
        let contents = fs::read_to_string(path)?;
        let value: Value =
            serde_json::from_str(&contents).map_err(|e| format!("Invalid JSON: {}", e))?;
        let emboss_bin_dir = value
            .get("emboss_bin_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        Ok(Self { emboss_bin_dir })
    }
}

impl SearchColorConfig {
    pub fn from_file(path: &str) -> Result<Self, TermalError> {
        let contents = fs::read_to_string(path)?;
        let value: Value =
            serde_json::from_str(&contents).map_err(|e| format!("Invalid JSON: {}", e))?;
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
        Ok(Self {
            palette: if palette.is_empty() {
                DEFAULT_SEARCH_PALETTE.to_vec()
            } else {
                palette
            },
            current_search,
            min_component,
            gap_dim_factor,
            luminance_threshold,
        })
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

pub struct App {
    pub filename: String,
    pub alignment: Alignment,
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
    emboss_bin_dir: Option<PathBuf>,
    filtered_output_path: PathBuf,
    rejected_output_path: PathBuf,
}

impl App {
    pub fn new(path: &str, alignment: Alignment, usr_ord: Option<Vec<String>>) -> Self {
        let len = alignment.num_seq();
        let cur_msg = CurrentMessage {
            prefix: String::from(""),
            message: String::from(""),
            kind: MessageKind::Info,
        };
        let search_color_config = SearchColorConfig::default();
        let filtered_output_path = next_available_output_path(path, "filt");
        let rejected_output_path = next_available_output_path(path, "rej");
        App {
            filename: path.to_string(),
            alignment,
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
            emboss_bin_dir: None,
            filtered_output_path,
            rejected_output_path,
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
        let try_re = RegexBuilder::new(pattern).case_insensitive(true).build();
        match try_re {
            Ok(re) => {
                // actually numbers of matching lines, but a bit longish
                let matches: Vec<usize> = self
                    .alignment
                    .headers
                    .iter()
                    .enumerate()
                    .filter_map(|(i, line)| re.is_match(line).then_some(i))
                    .collect();

                // Start with all false, and flip to true only for matching lines
                let mut match_linenum_vec: Vec<bool> = vec![false; self.alignment.num_seq()];
                for i in &matches {
                    match_linenum_vec[*i] = true;
                }

                self.search_state = Some(SearchState {
                    pattern: String::from(pattern),
                    regex: re,
                    match_linenums: matches,
                    current: 0,
                    hdr_match_status: match_linenum_vec,
                });
            }
            Err(e) => {
                self.error_msg(format!("Malformed regex {}.", e));
                self.search_state = None;
            }
        }
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

    pub fn is_current_label_match(&self, rank: usize) -> bool {
        self.current_label_match_rank()
            .map(|current| current == rank)
            .unwrap_or(false)
    }

    // Returns true IFF there is a search result AND header of rank `rank` (i.e., without
    // correction for order) is a match.
    pub fn is_label_search_match(&self, rank: usize) -> bool {
        if let Some(state) = &self.search_state {
            state.hdr_match_status[rank]
        } else {
            false
        }
    }

    pub fn reset_lbl_search(&mut self) {
        self.search_state = None;
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
        Ok(())
    }

    pub fn delete_saved_search(&mut self, index: usize) -> bool {
        self.search_registry.delete(index)
    }

    pub fn toggle_saved_search(&mut self, index: usize) -> bool {
        self.search_registry.toggle(index)
    }

    pub fn saved_searches(&self) -> &[SearchEntry] {
        self.search_registry.entries()
    }

    pub fn set_emboss_bin_dir(&mut self, dir: Option<PathBuf>) {
        self.emboss_bin_dir = dir;
    }

    pub fn emboss_search_sequences(&mut self, pattern: &str) {
        if pattern.is_empty() {
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
        let removed = self.alignment.remove_seq(rank)?;
        self.search_state = None;
        self.seq_search_state = None;
        self.refresh_saved_searches();
        self.recompute_ordering();
        Some(removed)
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

    pub fn filtered_path(&self) -> PathBuf {
        self.filtered_output_path.clone()
    }

    pub fn rejected_path(&self) -> PathBuf {
        self.rejected_output_path.clone()
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
    let is_nucleic = sequences
        .iter()
        .all(|seq| seq.chars().all(|c| is_gap(c) || is_acgt(c)));
    let tool = if is_nucleic { "fuzznuc" } else { "fuzzpro" };
    let tool_path = emboss_bin_dir
        .map(|dir| dir.join(tool))
        .unwrap_or_else(|| PathBuf::from(tool));
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

    use crate::{
        alignment::Alignment,
        app::{order, App, SeqMatch},
    };

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
                assert_eq!(
                    state.hdr_match_status,
                    vec![true, true, false, false, false]
                );
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
        assert!(app.is_label_search_match(0));
        assert!(app.is_label_search_match(1));
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
