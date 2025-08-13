// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader};

use crate::seq::record::SeqRecord;
use crate::seq::file::SeqFile;

pub fn read_stockholm_file<P: AsRef<Path>>(path: P) -> Result<SeqFile, std::io::Error> {
    let file = File::open(path)?;
    let mut result: SeqFile = Vec::new();
    let mut current_record = SeqRecord { header: String::new(), sequence: String::new() };
    let mut first_header = true;

    for line in BufReader::new(file).lines() {
        let l: String = line.unwrap();
        if l.starts_with(">") { 
            if first_header {
                first_header = false;
            } else {
                // push existing record
                result.push(current_record);
            }
            current_record = SeqRecord { header: String::new(), sequence: String::new() };
            current_record.header.push_str(&l[1..]);
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
    fn test_read_stockholm_file_1() {
        let path = "data/PF00571.sto";
        let fasta: SeqFile = read_stockholm_file(path).expect("Test file not found");
        assert_eq!(fasta[0].header, "seq1");
        assert_eq!(fasta[0].sequence, "GAATTC");
    }

    // TODO: more tests
}
