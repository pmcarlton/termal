// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::{collections::HashMap, fmt};

use regex::Regex;

use crate::{
    alignment::Alignment,
    app::Metric::{PctIdWrtConsensus, SeqLen},
    app::SeqOrdering::{MetricDecr, MetricIncr, SourceFile, User},
};

type SearchColor = (u8, u8, u8);

const SEARCH_COLORS: [SearchColor; 8] = [
    (255, 105, 97),
    (119, 221, 119),
    (135, 206, 235),
    (255, 179, 71),
    (255, 105, 180),
    (176, 196, 222),
    (255, 218, 185),
    (152, 251, 152),
];
#[derive(Clone, Copy)]
pub enum SeqOrdering {
    SourceFile,
    MetricIncr,
    MetricDecr,
    User,
}

impl fmt::Display for SeqOrdering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sord = match self {
            SourceFile => '-',
            MetricIncr => '↑',
            MetricDecr => '↓',
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
    pub pattern: String,
    pub spans_by_seq: Vec<Vec<(usize, usize)>>,
    pub total_matches: usize,
    pub sequences_with_matches: usize,
}

pub struct SearchEntry {
    pub id: usize,
    pub name: String,
    pub query: String,
    pub enabled: bool,
    pub color: SearchColor,
    pub spans_by_seq: Vec<Vec<(usize, usize)>>,
}

pub struct SearchRegistry {
    searches: Vec<SearchEntry>,
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
    current_msg: CurrentMessage,
}

impl App {
    pub fn new(path: &str, alignment: Alignment, usr_ord: Option<Vec<String>>) -> Self {
        let len = alignment.num_seq();
        let cur_msg = CurrentMessage {
            prefix: String::from(""),
            message: String::from(""),
            kind: MessageKind::Info,
        };
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
            search_registry: SearchRegistry::new(),
            current_msg: cur_msg,
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
            MetricDecr => match self.user_ordering {
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
            User => MetricDecr,
            // move to User IFF valid ordering
            SourceFile => match self.user_ordering {
                Some(_) => User,
                None => MetricDecr,
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
        let try_re = Regex::new(pattern);
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
            self.seq_search_state = None;
            return;
        }
        match compute_seq_search_state(&self.alignment.sequences, pattern) {
            Ok(state) => self.seq_search_state = Some(state),
            Err(e) => {
                self.error_msg(format!("Malformed regex {}.", e));
                self.seq_search_state = None;
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

    pub fn seq_search_counts(&self) -> Option<(usize, usize)> {
        self.seq_search_state
            .as_ref()
            .map(|state| (state.total_matches, state.sequences_with_matches))
    }

    pub fn add_saved_search(&mut self, name: String, query: String) -> Result<(), String> {
        if query.is_empty() {
            return Err(String::from("Empty search query"));
        }
        let state = compute_seq_search_state(&self.alignment.sequences, &query)
            .map_err(|e| format!("Malformed regex {}.", e))?;
        self.search_registry
            .add_search(name, query, state.spans_by_seq);
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
    fn new() -> Self {
        Self {
            searches: Vec::new(),
            next_color_index: 0,
        }
    }

    fn entries(&self) -> &[SearchEntry] {
        &self.searches
    }

    fn add_search(&mut self, name: String, query: String, spans_by_seq: Vec<Vec<(usize, usize)>>) {
        let color = SEARCH_COLORS[self.next_color_index % SEARCH_COLORS.len()];
        self.next_color_index += 1;
        let id = self.searches.len() + 1;
        self.searches.push(SearchEntry {
            id,
            name,
            query,
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
}

fn compute_seq_search_state(
    sequences: &[String],
    pattern: &str,
) -> Result<SeqSearchState, regex::Error> {
    let re = Regex::new(pattern)?;
    let mut spans_by_seq: Vec<Vec<(usize, usize)>> = Vec::with_capacity(sequences.len());
    let mut total_matches = 0;
    let mut sequences_with_matches = 0;
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
    Ok(SeqSearchState {
        pattern: pattern.to_string(),
        spans_by_seq,
        total_matches,
        sequences_with_matches,
    })
}

#[cfg(test)]
mod tests {

    use crate::{
        alignment::Alignment,
        app::{order, App},
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
}
