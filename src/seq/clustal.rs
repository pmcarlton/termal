// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::errors::TermalError;
use crate::seq::file::SeqFile;
use crate::seq::record::SeqRecord;

pub fn read_clustal_file<P: AsRef<Path>>(path: P) -> Result<SeqFile, TermalError> {
    let file = File::open(path)?;
    let mut order: Vec<String> = Vec::new();
    let mut sequences: HashMap<String, String> = HashMap::new();

    for line in BufReader::new(file).lines() {
        let l = line?;
        let trimmed = l.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("CLUSTAL") || trimmed.starts_with("MUSCLE") {
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
        {
            continue;
        }
        let mut fields = trimmed.split_whitespace();
        let name = fields
            .next()
            .ok_or_else(|| TermalError::Format(String::from("Missing sequence id")))?;
        let fragment = fields
            .next()
            .ok_or_else(|| TermalError::Format(String::from("Missing sequence fragment")))?;
        let entry = sequences.entry(name.to_string()).or_insert_with(|| {
            order.push(name.to_string());
            String::new()
        });
        entry.push_str(fragment);
    }

    if order.is_empty() {
        return Err(TermalError::Format(String::from("No sequences found")));
    }

    let mut result: SeqFile = Vec::new();
    for name in order {
        let sequence = sequences.remove(&name).unwrap_or_default();
        result.push(SeqRecord {
            header: name,
            sequence,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_clustal_file() {
        let path = "data/test-clustal.aln";
        let records = read_clustal_file(path).expect("Test file not found");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].header, "seq1");
        assert_eq!(records[0].sequence, "ATG-CTG");
        assert_eq!(records[1].header, "seq2");
        assert_eq!(records[1].sequence, "AT-ACT-");
    }
}
