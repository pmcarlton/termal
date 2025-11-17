// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::{
    collections::HashMap,
    fmt,
};

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
}

impl App {
    pub fn new(path: &str, alignment: Alignment, usr_ord: Option<Vec<String>>) -> Self {
        let len = alignment.num_seq();
        App {
            filename: path.to_string(),
            alignment,
            ordering_criterion: SourceFile,
            metric: PctIdWrtConsensus,
            ordering: (0..len).collect(),
            user_ordering: usr_ord,
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
                match &self.user_ordering {
                    None => {
                        // Do not change ordering if no user ordering provided; better yet: prevent
                        // ordering_criterion from being set to User if no user ordering was
                        // provided.
                    }
                    Some(uord_vec) => {
                        // Technically, we could index by &str, but I'm not sure we'd gain a lot.
                        let mut hdr2rank: HashMap<String, usize> = HashMap::new();
                        for (idx, hdr) in self.alignment.headers
                            .iter().enumerate() {
                                hdr2rank.insert(hdr.to_string(), idx);
                        }
                        // Iterate over ordering, looking up file index from the above hash.
                        let mut result: Vec<usize> = Vec::new();
                        for hdr in uord_vec.iter() {
                            match hdr2rank.get(hdr) {
                                Some(rank) => result.push(*rank),
                                None       => break,
                            }
                            
                        }
                        // If result's length is not the same as the number of sequences in the
                        // alignment, there was a problem (most likely a header was not found in
                        // the hdr2rank hash above) -> do nothing. TODO: warn the user, if possible
                        // once and for all and as early as possible (if user passes -o, do the
                        // check as soon as we have an Alignment struct).
                        if result.len() == self.alignment.headers.len() {
                            self.ordering = result;
                        } else {
                            // TODO: warn user (see above)
                        }
                    }
                }
            }
        }
    }

    pub fn next_ordering_criterion(&mut self) {
        self.ordering_criterion = match self.ordering_criterion {
            SourceFile => MetricIncr,
            MetricIncr => MetricDecr,
            MetricDecr => User,
            User       => SourceFile, 
        };
        self.recompute_ordering();
    }

    pub fn prev_ordering_criterion(&mut self) {
        self.ordering_criterion = match self.ordering_criterion {
            MetricIncr => SourceFile,
            MetricDecr => MetricIncr,
            SourceFile => MetricDecr,
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
}
