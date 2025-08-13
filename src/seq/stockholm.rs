// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader, Error, ErrorKind};

use crate::seq::record::SeqRecord;
use crate::seq::file::SeqFile;

pub fn read_stockholm_file<P: AsRef<Path>>(path: P) -> Result<SeqFile, std::io::Error> {
    let file = File::open(path)?;
    let mut result: SeqFile = Vec::new();

    for line in BufReader::new(file).lines() {
        let l: String = line.unwrap();
        let first_char = l.chars().next().unwrap();
        match first_char {
            '/' => { break; } // Assuming '/' is the beginning of '//', which conceivably might not be
                          // true
            '#' => {} // Annotation -> ignore.
            _ => {
                let mut fields = l.split_whitespace();

                match (fields.next(), fields.next(), fields.next()) {
                    (Some(seqname), Some(aln_seq), None) => {
                        let record = SeqRecord { header: String::from(seqname), sequence: String::from(aln_seq) };
                        result.push(record);
                    }
                    // TODO: use a specific kind of Error for this, not a std::io::Error.
                    _ => return Err(Error::new(ErrorKind::InvalidData, "Expected exactly two fields"))
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_stockholm_file_len() {
        let path = "data/PF00571.sto";
        let fasta: SeqFile = read_stockholm_file(path).expect("Test file not found");
        assert_eq!(fasta.len(), 5);
    }

    #[test]
    fn test_read_stockholm_file_1st_record() {
        let path = "data/PF00571.sto";
        let fasta: SeqFile = read_stockholm_file(path).expect("Test file not found");
        assert_eq!(fasta[0].header, "O83071/192-246");
        assert_eq!(fasta[0].sequence, "MTCRAQLIAVPRASSLAE..AIACAQKM....RVSRVPVYERS");
    }

    #[test]
    fn test_read_stockholm_file_last_record() {
        let path = "data/PF00571.sto";
        let fasta: SeqFile = read_stockholm_file(path).expect("Test file not found");
        assert_eq!(fasta[4].header, "O31699/88-139");
        assert_eq!(fasta[4].sequence, "EVMLTDIPRLHINDPIMK..GFGMVINN......GFVCVENDE");
    }

    // TODO: more tests
}
