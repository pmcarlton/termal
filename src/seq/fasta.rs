// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier
// Modifications (c) 2026 Peter Carlton

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::seq::file::SeqFile;
use crate::seq::record::SeqRecord;

pub fn read_fasta_file<P: AsRef<Path>>(path: P) -> Result<SeqFile, std::io::Error> {
    let file = File::open(path)?;
    let mut result: SeqFile = Vec::new();
    let mut current_record = SeqRecord {
        header: String::new(),
        sequence: String::new(),
    };
    let mut first_header = true;
    let mut seen_ids: HashSet<String> = HashSet::new();

    for line in BufReader::new(file).lines() {
        let l: String = line.unwrap();
        if let Some(hdr) = l.strip_prefix(">") {
            if first_header {
                first_header = false;
            } else {
                // push existing record
                result.push(current_record);
            }
            current_record = SeqRecord {
                header: String::new(),
                sequence: String::new(),
            };
            let id = hdr.split_whitespace().next().unwrap_or("").to_string();
            if id.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Empty FASTA header",
                ));
            }
            if !seen_ids.insert(id.clone()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Duplicate FASTA ID: {}", id),
                ));
            }
            current_record.header.push_str(&id);
        } else {
            // append line to current record'd sequence
            current_record.sequence.push_str(&l);
        }
    }
    result.push(current_record);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_fasta_file_1() {
        let path = "data/test1.fas";
        let fasta: SeqFile = read_fasta_file(path).expect("Test file not found");
        assert_eq!(fasta[0].header, "seq1");
        assert_eq!(fasta[0].sequence, "GAATTC");
    }

    #[test]
    fn test_read_fasta_file_2() {
        let path = "data/test2.fas";
        let fasta: SeqFile = read_fasta_file(path).expect("Test file not found");
        assert_eq!(fasta[0].header, "seq1");
        assert_eq!(fasta[0].sequence, "TTGCCG-CGA");
        assert_eq!(fasta[1].header, "seq2");
        assert_eq!(fasta[1].sequence, "TTCCCGGCGA");
        assert_eq!(fasta[2].header, "seq3");
        assert_eq!(fasta[2].sequence, "TTACCG-CAA");
    }

    #[test]
    fn test_read_fasta_file_3() {
        let path = "data/test3.pep";
        let fasta: SeqFile = read_fasta_file(path).expect("Test file not found");
        assert_eq!(fasta[0].header, "Some");
        assert_eq!(fasta[0].sequence, "HWYQYDSWSWHQIQDPWVASLMTGSEHNTTIVDLNVLGAMDCLWLCYCQPECFEVFSLCIEVDLPSCCWAKALCAFHMWDSMAKQCWMPEMGEVSYFYALSMFHYFLLHSRPIQPWQTHHIPYDSIVVDLIANYFYNMIVQDVDKNSNIRFDRSVMRDVMIYEFENTYATGVVFNVNGKCGQFCKNMIYVGTIETQKEYEMFKNLDCAVQKRHNLQPNCENIAMKMRIQYNGKRFRMDYWERYRCNDIKQVLPQPFTEVAMEHRTFKLWPTTRLMMSNPKCRQCLEWAAVETGWIFTTNF");
    }
}
