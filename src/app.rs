// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::{
    collections::HashMap,
    fmt,
};

use regex::Regex;

use crate::{
    alignment::Alignment,
    app::Metric::{PctIdWrtConsensus, SeqLen},
    app::SeqOrdering::{MetricDecr, MetricIncr, SourceFile, User},
};

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
            User       => 'u',
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
    pub pattern: String,
    regex: Regex,
    pub match_linenums: Vec<usize>,
    pub current: usize,
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
    user_ordering: Option<Vec<String>>,
    pub search_state: Option<SearchState>,
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
            user_ordering: usr_ord,
            search_state: None,
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
                        for (idx, hdr) in self.alignment.headers
                            .iter().enumerate() {
                                hdr2rank.insert(hdr.to_string(), idx);
                        }
                        // Iterate over ordering, looking up file index from the above hash.
                        let mut result: Vec<usize> = Vec::new();
                        // TODO: now that we no check for discrepancies here, this should be
                        //feasible in a sinmple map.
                        for hdr in uord_vec.iter() {
                            match hdr2rank.get(hdr) {
                                Some(rank) => result.push(*rank),
                                None       => break,
                            }
                            
                        }
                        self.ordering = result;
                    }
                }
            }
        }
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
            User       => SourceFile, 
        };
        self.recompute_ordering();
    }

    pub fn prev_ordering_criterion(&mut self) {
        self.ordering_criterion = match self.ordering_criterion {
            MetricIncr => SourceFile,
            MetricDecr => MetricIncr,
            User       => MetricDecr,
            // move to User IFF valid ordering
            SourceFile => match self.user_ordering {
                            Some(_) => User,
                            None => MetricDecr,
            },
        };
        self.recompute_ordering();
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
        self.debug_msg("Regex search");
        let try_re = Regex::new(pattern);
        match try_re {
            Ok(re) => {
                // actually numbers of matching lines, but a bit longish
                let matches: Vec<usize> = self.alignment.headers
                    .iter()
                    .enumerate()
                    .filter_map(|(i,line)| re.is_match(line).then_some(i))
                    .collect();
                
                self.search_state = Some(SearchState {
                    pattern: String::from(pattern),
                    regex: re,
                    match_linenums: matches,
                    current: 0
                });
            }
            Err(e) => {
                self.error_msg(format!("Malformed regex {}.", e));
                self.search_state = None;
            }
        }
    }

    pub fn current_label_match_linenum(&self) -> Option<usize> {
        if let Some(state) = &self.search_state {
            if state.match_linenums.len() > 0 {
                Some(state.match_linenums[state.current])
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
                    let new = (state.current as isize + count).rem_euclid(nb_matches as isize) as usize;
                    //let new = (state.current + count) % nb_matches.;
                    self.search_state.as_mut().unwrap().current = new;
                    self.info_msg(format!("match #{}/{}",
                            self.search_state.as_ref().unwrap().current + 1, // +1 <- user is 1-based
                            self.search_state.as_ref().unwrap().match_linenums.len()));
                } else {
                    self.info_msg("No match.");
                }
            }
            None => {
                self.info_msg("No current search.");
            }
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
fn order(nums: &Vec<f64>) -> Vec<usize> {
    // let result: Vec<usize> = Vec::with_capacity(nums.len());
    let init_order: Vec<usize> = (0..nums.len()).collect();
    let zip_iter = init_order.iter().zip(nums);
    let mut unsorted_pairs: Vec<(&usize, &f64)> = zip_iter.collect();
    unsorted_pairs.sort_by(|(_, t1), (_, t2)| t1.partial_cmp(t2).expect("Unorder!"));
    unsorted_pairs
        .into_iter()
        .map(|(u, _)| *u)
        .collect::<Vec<usize>>()
}

#[cfg(test)]
mod tests {

    use crate::app::order;

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
    fn test_regex_lbl_search_10() { todo!(); }
}
